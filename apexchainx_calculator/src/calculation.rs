//! Core SLA calculation logic and stats tracking.
//!
//! This module contains the delegated implementation of `calculate_sla`,
//! `calculate_sla_view`, stats management, and telemetry recording.
//!
//! # Boundary between business logic and side effects
//!
//! The computation pipeline enforces a strict separation:
//!
//! | Phase | What happens | Side effects? |
//! |---|---|---|
//! | **1. Pre-flight** | Version check, pause check, operator auth, config load | Read-only |
//! | **2. Pure computation** | `compute_result()` — deterministic SLA outcome from inputs | None |
//! | **3. State mutation** | History anti-spam, telemetry recording, stats increment | Storage writes only |
//! | **4. Event publication** | `publish_sla_event()` + `publish_settlement_intent_event()` | Event emission |
//!
//! This separation ensures:
//! - `compute_result()` can be called by `calculate_sla_view` (read-only audit)
//!   without touching storage or emitting events.
//! - Event schemas can evolve independently of the computation rules.
//! - Tests can validate business logic without needing an event assertion harness
//!   by calling `compute_result()` directly.
//! - The same outcome is deterministic regardless of whether events are
//!   actually published (e.g. in dry-run simulations).
//!
//! See [`crate::event::EventPublisher`] for the event publication abstraction.

use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

use crate::{
    SLAConfig, SLAError, SLAResult, SLAStats, SeverityTelemetry, EVENT_DUP_INPUT, EVENT_SETTLE_INTENT,
    EVENT_SLA_CALC, EVENT_VERSION, HISTORY_KEY, HISTORY_LEN_KEY, LAST_CALCULATION_TS_KEY,
    LAST_VIOLATION_TS_KEY, MAX_HISTORY_SIZE, MAX_RECALCS_PER_OUTAGE, PAUSED_KEY, RETENTION_LIMIT_KEY,
    SEVERITY_CALC_COUNTS_KEY, SEVERITY_VIOL_COUNTS_KEY, STATS_KEY,
};

/// Calculate the SLA outcome for an outage event (delegated implementation).
///
/// See [`crate::SLACalculatorContract::calculate_sla`] for the full API
/// contract and [`crate::SLAError::DuplicateOutageInput`] for the
/// duplicate-detection semantics.
///
/// # Execution phases
///
/// This function follows a strict phased execution model:
/// 1. **Pre-flight** — version check, pause guard, operator authorization, config load.
/// 2. **Pure computation** — `compute_result()` produces a deterministic outcome.
/// 3. **State mutation** — anti-spam dedup, history append, telemetry, stats update.
/// 4. **Event publication** — `publish_sla_event()` + `publish_settlement_intent_event()`.
///
/// Phases 1-3 constitute the **business logic** boundary. Phase 4 is the sole
/// **side-effect** phase and never alters the returned `SLAResult`.
pub fn calculate_sla(
    env: &Env,
    caller: &Address,
    outage_id: Symbol,
    severity: Symbol,
    mttr_minutes: u32,
) -> Result<SLAResult, SLAError> {
    // ── Phase 1: Pre-flight (read-only auth & config) ──────────────────
    crate::SLACalculatorContract::check_version(env)?;
    require_not_paused(env)?;
    crate::SLACalculatorContract::require_operator(env, caller)?;

    let cfg = crate::SLACalculatorContract::load_config(env, &severity)?;
    let config_version_hash = crate::SLACalculatorContract::compute_config_version_hash(env)?;

    // ── Phase 2: Pure computation (no state reads/writes) ─────────────
    let result = compute_result(
        outage_id.clone(),
        mttr_minutes,
        &cfg,
        config_version_hash,
        env.ledger().timestamp(),
    )?;

    // ── Phase 3: State mutation (storage writes only) ─────────────────
    let mut history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));

    // Anti-spam accounting — mirrors SLACalculatorContract::calculate_sla.
    let mut existing: Option<SLAResult> = None;
    let mut stored_for_outage: u32 = 0;
    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.outage_id == outage_id {
            stored_for_outage += 1;
            existing = Some(entry);
        }
    }
    if let Some(prev) = existing {
        if prev.config_version_hash == config_version_hash {
            if prev.mttr_minutes != mttr_minutes || prev.threshold_minutes != cfg.threshold_minutes {
                // #385 – publish the stored result alongside the rejection so
                // consumers can reconcile the conflict from this transaction's
                // event log without a second get_latest_by_outage read.
                publish_duplicate_input_event(env, severity.clone(), &prev);
                return Err(SLAError::DuplicateOutageInput);
            }
            // Replay: return the stored decision without touching state.
            return Ok(prev);
        }
        if stored_for_outage >= MAX_RECALCS_PER_OUTAGE {
            return Err(SLAError::OutageRecalcLimit);
        }
    }

    // Only recorded once the result is certain to be stored, so replays and
    // rejected submissions cannot inflate per-severity counters.
    let met = result.status != symbol_short!("viol");
    record_severity_telemetry(env, &severity, met);

    history.push_back(result.clone());

    // #461 – track total entries ever stored
    let prev_total: u32 = env
        .storage()
        .instance()
        .get(&TOTAL_ENTRIES_KEY)
        .unwrap_or(0);
    env.storage()
        .instance()
        .set(&TOTAL_ENTRIES_KEY, &prev_total.saturating_add(1));

    let retention_limit: u32 = env
        .storage()
        .instance()
        .get(&RETENTION_LIMIT_KEY)
        .unwrap_or(MAX_HISTORY_SIZE);

    if history.len() > retention_limit {
        let mut trimmed = Vec::new(env);
        for i in 1..history.len() {
            trimmed.push_back(history.get(i).unwrap());
        }
        env.storage().instance().set(&HISTORY_KEY, &trimmed);
        env.storage().instance().set(&HISTORY_LEN_KEY, &trimmed.len());
    } else {
        env.storage().instance().set(&HISTORY_KEY, &history);
        env.storage().instance().set(&HISTORY_LEN_KEY, &history.len());
    }

    if result.status == symbol_short!("viol") {
        increment_stats(env, false, 0, -result.amount);
    } else {
        increment_stats(env, true, result.amount, 0);
    }

    // ── Phase 4: Event publication (side effects only) ────────────────
    // Published after all state mutations are committed so that indexers
    // observe a consistent view. These calls never affect the returned
    // SLAResult and can be toggled independently for dry-run modes.
    publish_sla_event(env, severity.clone(), &result);
    publish_settlement_intent_event(env, severity, &result);

    Ok(result)
}

/// Recalculates SLA deterministically without mutating state or emitting events.
/// Can be called by anyone for audit and verification purposes.
pub fn calculate_sla_view(
    env: &Env,
    outage_id: Symbol,
    severity: Symbol,
    mttr_minutes: u32,
) -> Result<SLAResult, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let cfg = crate::SLACalculatorContract::load_config(env, &severity)?;
    let config_version_hash = crate::SLACalculatorContract::compute_config_version_hash(env)?;

    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));

    let mut existing: Option<SLAResult> = None;
    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.outage_id == outage_id {
            existing = Some(entry);
        }
    }
    if let Some(prev) = existing {
        if prev.config_version_hash == config_version_hash {
            if prev.mttr_minutes != mttr_minutes || prev.threshold_minutes != cfg.threshold_minutes {
                return Err(SLAError::DuplicateOutageInput);
            }
            return Ok(prev);
        }
    }

    compute_result(
        outage_id,
        mttr_minutes,
        &cfg,
        config_version_hash,
        env.ledger().timestamp(),
    )
}

/// Returns the cumulative SLA performance statistics.
pub fn get_stats(env: &Env) -> Result<SLAStats, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    env.storage()
        .instance()
        .get(&STATS_KEY)
        .ok_or(SLAError::NotInitialized)
}

/// Returns per-severity weekly violation-rate telemetry.
pub fn get_severity_telemetry(env: &Env) -> Result<Vec<SeverityTelemetry>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let mut telemetry = Vec::new(env);
    let severities = crate::SLACalculatorContract::canonical_severities(env);
    let calculations = load_counts(env, &SEVERITY_CALC_COUNTS_KEY);
    let violations = load_counts(env, &SEVERITY_VIOL_COUNTS_KEY);

    for index in 0..severities.len() {
        let severity = severities.get(index).unwrap();
        let calc_count = count_lane(calculations, index);
        let violation_count = count_lane(violations, index);
        let violation_rate = if calc_count == 0 {
            0u32
        } else {
            (violation_count.saturating_mul(100) / calc_count).min(100)
        };
        telemetry.push_back(SeverityTelemetry {
            severity: severity.clone(),
            calculations: calc_count,
            violations: violation_count,
            violation_rate,
        });
    }

    Ok(telemetry)
}

fn require_not_paused(env: &Env) -> Result<(), SLAError> {
    let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
    if paused {
        return Err(SLAError::ContractPaused);
    }
    Ok(())
}

/// Maximum allowed MTTR in minutes to prevent arithmetic overflow.
/// This conservative bound ensures that even with maximum penalty rates,
/// the calculation cannot overflow i128. 
/// 525,600 minutes = 365 days, well beyond any realistic outage duration.
const MAX_MTTR_MINUTES: u32 = 525_600;

/// Computes the SLA result (met/violated, reward/penalty, rating) from inputs.
/// Pure function — no state reads or writes.
pub fn compute_result(
    outage_id: Symbol,
    mttr_minutes: u32,
    cfg: &SLAConfig,
    config_version_hash: u64,
    recorded_at: u64,
) -> Result<SLAResult, SLAError> {
    let threshold = cfg.threshold_minutes;
    
    // Validate input range before computation to provide clear error messages
    if mttr_minutes > MAX_MTTR_MINUTES {
        return Err(SLAError::InvalidInput);
    }

    if mttr_minutes > threshold {
        let overtime = (mttr_minutes - threshold) as i128;
        let penalty = match overtime.checked_mul(cfg.penalty_per_minute) {
            Some(val) => val,
            None => return Err(SLAError::InvalidPenaltyAmount),
        };
        let amount = match penalty.checked_neg() {
            Some(val) => val,
            None => return Err(SLAError::InvalidPenaltyAmount),
        };
        if amount >= 0 {
            return Err(SLAError::InvalidPenaltyAmount);
        }

        Ok(SLAResult {
            outage_id,
            status: symbol_short!("viol"),
            mttr_minutes,
            threshold_minutes: threshold,
            amount,
            payment_type: symbol_short!("pen"),
            rating: symbol_short!("poor"),
            config_version_hash,
            recorded_at,
        })
    } else {
        let performance_ratio = (mttr_minutes as u64 * 100)
            .checked_div(threshold as u64)
            .unwrap_or(0);

        let (multiplier, rating) = if performance_ratio < 50 {
            (200u32, symbol_short!("top"))
        } else if performance_ratio < 75 {
            (150u32, symbol_short!("excel"))
        } else {
            (100u32, symbol_short!("good"))
        };

        let reward = match cfg.reward_base.checked_mul(multiplier as i128) {
            Some(val) => val.div_euclid(100),
            None => return Err(SLAError::InvalidRewardAmount),
        };
        if reward <= 0 {
            return Err(SLAError::InvalidRewardAmount);
        }

        Ok(SLAResult {
            outage_id,
            status: symbol_short!("met"),
            mttr_minutes,
            threshold_minutes: threshold,
            amount: reward,
            payment_type: symbol_short!("rew"),
            rating,
            config_version_hash,
            recorded_at,
        })
    }
}

fn load_counts(env: &Env, key: &Symbol) -> u128 {
    env.storage().instance().get(key).unwrap_or(0u128)
}

fn count_lane(packed: u128, index: u32) -> u32 {
    ((packed >> (index * 32)) & 0xFFFF_FFFF) as u32
}

fn set_count_lane(packed: u128, index: u32, value: u32) -> u128 {
    let mask = !(0xFFFF_FFFFu128 << (index * 32));
    (packed & mask) | ((value as u128) << (index * 32))
}

/// Records per-severity calculation/violation counters for telemetry.
/// Record severity telemetry for a calculation execution.
///
/// ### Telemetry Weekly Reset Semantics
/// The telemetry system maintains per-severity rolling counters for calculations and violations.
/// When recording a new entry for a severity lane, the contract checks if the elapsed time since the last
/// calculation timestamp or last violation timestamp for that severity is greater than or equal to 7 days (604,800 seconds).
///
/// - **Lazy Reset Strategy**: Resets are non-blocking and lazy; counters are not automatically reset by background cron tasks.
///   Instead, reset is triggered on the next `calculate_sla` invocation for that specific severity lane once 7 days have passed.
/// - **Per-Counter Isolation**: Resets are per-counter within each severity lane. Inactivity in calculation or violation counters resets only its respective counter (e.g., a stale calculation counter reset will not wipe a fresh violation counter).
/// - **Reinitialization**: Upon reset, the lane's calculation and violation counters are cleared to 0 before the current invocation is recorded,
///   reinitializing the count to 1 calculation (and 1 violation if the current calculation violated SLA).
///
/// ### Counter Saturation
/// Each severity lane is a `u32`. Increments use `saturating_add(1)`, so a lane
/// saturates at `u32::MAX` rather than wrapping (release) or panicking (debug).
/// `u32::MAX` is treated as "many" and is never reset to zero by overflow.
pub fn record_severity_telemetry(env: &Env, severity: &Symbol, met: bool) {
    let index = crate::SLACalculatorContract::canonical_severity_index(severity).unwrap_or(0);
    let mut calculations = load_counts(env, &SEVERITY_CALC_COUNTS_KEY);
    let mut violations = load_counts(env, &SEVERITY_VIOL_COUNTS_KEY);
    let mut last_calculations = load_counts(env, &LAST_CALCULATION_TS_KEY);
    let mut last_violations = load_counts(env, &LAST_VIOLATION_TS_KEY);

    let now = env.ledger().timestamp();
    let week_seconds = 7u64 * 24u64 * 60u64 * 60u64;
    let last_calc = count_lane(last_calculations, index) as u64;
    let last_violation = count_lane(last_violations, index) as u64;
    let calc_stale = last_calc != 0 && now.saturating_sub(last_calc) >= week_seconds;
    let violation_stale = last_violation != 0 && now.saturating_sub(last_violation) >= week_seconds;
    if calc_stale {
        calculations = set_count_lane(calculations, index, 0);
    }
    if violation_stale {
        violations = set_count_lane(violations, index, 0);
    }

    calculations = set_count_lane(
        calculations,
        index,
        count_lane(calculations, index).saturating_add(1),
    );
    if !met {
        violations = set_count_lane(violations, index, count_lane(violations, index).saturating_add(1));
    }

    let current_ts = if now > u64::from(u32::MAX) {
        u32::MAX
    } else {
        now as u32
    };
    last_calculations = set_count_lane(last_calculations, index, current_ts);
    if !met {
        last_violations = set_count_lane(last_violations, index, current_ts);
    }

    env.storage()
        .instance()
        .set(&SEVERITY_CALC_COUNTS_KEY, &calculations);
    env.storage()
        .instance()
        .set(&SEVERITY_VIOL_COUNTS_KEY, &violations);
    env.storage()
        .instance()
        .set(&LAST_CALCULATION_TS_KEY, &last_calculations);
    env.storage()
        .instance()
        .set(&LAST_VIOLATION_TS_KEY, &last_violations);
}

/// Increments the cumulative SLA statistics, emitting `stats_sat` on overflow.
pub fn increment_stats(env: &Env, met: bool, reward: i128, penalty: i128) {
    let mut stats: SLAStats = env.storage().instance().get(&STATS_KEY).unwrap_or(SLAStats {
        total_calculations: 0,
        total_violations: 0,
        total_rewards: 0,
        total_penalties: 0,
    });

    match stats.total_calculations.checked_add(1) {
        Some(v) => stats.total_calculations = v,
        None => {
            emit_stats_saturated(env, symbol_short!("totcalc"), stats.total_calculations as i128, 1);
            stats.total_calculations = u64::MAX;
        }
    }

    if met {
        match stats.total_rewards.checked_add(reward) {
            Some(v) => stats.total_rewards = v,
            None => {
                emit_stats_saturated(env, symbol_short!("totrew"), stats.total_rewards, reward);
                stats.total_rewards = if reward > 0 { i128::MAX } else { i128::MIN };
            }
        }
    } else {
        match stats.total_violations.checked_add(1) {
            Some(v) => stats.total_violations = v,
            None => {
                emit_stats_saturated(env, symbol_short!("totviol"), stats.total_violations as i128, 1);
                stats.total_violations = u64::MAX;
            }
        }
        match stats.total_penalties.checked_add(penalty) {
            Some(v) => stats.total_penalties = v,
            None => {
                emit_stats_saturated(env, symbol_short!("totpen"), stats.total_penalties, penalty);
                stats.total_penalties = if penalty > 0 { i128::MAX } else { i128::MIN };
            }
        }
    }

    env.storage().instance().set(&STATS_KEY, &stats);
}

fn emit_stats_saturated(env: &Env, counter: Symbol, previous_value: i128, attempted_increment: i128) {
    env.events().publish(
        (crate::EVENT_STATS_SAT, EVENT_VERSION, counter.clone()),
        (counter, previous_value, attempted_increment),
    );
}

fn publish_sla_event(env: &Env, severity: Symbol, result: &SLAResult) {
    env.events().publish(
        (EVENT_SLA_CALC, EVENT_VERSION, severity),
        (
            result.outage_id.clone(),
            result.status.clone(),
            result.mttr_minutes,
            result.threshold_minutes,
            result.amount,
            result.payment_type.clone(),
            result.rating.clone(),
            result.config_version_hash,
            result.recorded_at,
        ),
    );
}

fn publish_settlement_intent_event(env: &Env, severity: Symbol, result: &SLAResult) {
    env.events().publish(
        (EVENT_SETTLE_INTENT, EVENT_VERSION, severity),
        (
            result.outage_id.clone(),
            result.status.clone(),
            result.mttr_minutes,
            result.threshold_minutes,
            result.amount,
            result.payment_type.clone(),
            result.rating.clone(),
            result.config_version_hash,
            result.recorded_at,
        ),
    );
}

fn publish_duplicate_input_event(env: &Env, severity: Symbol, existing: &SLAResult) {
    env.events().publish(
        (EVENT_DUP_INPUT, EVENT_VERSION, severity),
        (
            existing.outage_id.clone(),
            existing.status.clone(),
            existing.mttr_minutes,
            existing.threshold_minutes,
            existing.amount,
            existing.payment_type.clone(),
            existing.rating.clone(),
            existing.config_version_hash,
            existing.recorded_at,
        ),
    );
}
