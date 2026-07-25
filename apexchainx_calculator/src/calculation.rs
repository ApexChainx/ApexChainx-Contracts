use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

use crate::{
    SLAConfig, SLAError, SLAResult, SLAStats, SeverityTelemetry,
    STATS_KEY, HISTORY_KEY, RETENTION_LIMIT_KEY,
    SEVERITY_CALC_COUNTS_KEY, SEVERITY_VIOL_COUNTS_KEY,
    LAST_CALCULATION_LEDGER_KEY, LAST_VIOLATION_LEDGER_KEY,
    PAUSED_KEY, MAX_HISTORY_SIZE,
    EVENT_SLA_CALC, EVENT_SETTLE_INTENT, EVENT_VERSION, EVENT_STATS_SAT,
};

pub fn calculate_sla(
    env: &Env,
    caller: &Address,
    outage_id: Symbol,
    severity: Symbol,
    mttr_minutes: u32,
) -> Result<SLAResult, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    require_not_paused(env)?;
    crate::SLACalculatorContract::require_operator(env, caller)?;

    let cfg = crate::SLACalculatorContract::load_config(env, &severity)?;
    let config_version_hash = crate::SLACalculatorContract::compute_config_version_hash(env)?;
    let result = compute_result(
        outage_id.clone(),
        mttr_minutes,
        &cfg,
        config_version_hash,
        env.ledger().timestamp(),
    )?;
    let met = result.status != symbol_short!("viol");
    record_severity_telemetry(env, &severity, met);
    let mut history: Vec<SLAResult> = env
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

    history.push_back(result.clone());

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
    } else {
        env.storage().instance().set(&HISTORY_KEY, &history);
    }

    if result.status == symbol_short!("viol") {
        increment_stats(env, false, 0, -result.amount);
    } else {
        increment_stats(env, true, result.amount, 0);
    }

    publish_sla_event(env, severity.clone(), &result);
    publish_settlement_intent_event(env, severity, &result);

    Ok(result)
}

pub fn calculate_sla_view(
    env: &Env,
    outage_id: Symbol,
    severity: Symbol,
    mttr_minutes: u32,
) -> Result<SLAResult, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let cfg = crate::SLACalculatorContract::load_config(env, &severity)?;
    let config_version_hash = crate::SLACalculatorContract::compute_config_version_hash(env)?;
    compute_result(
        outage_id,
        mttr_minutes,
        &cfg,
        config_version_hash,
        env.ledger().timestamp(),
    )
}

pub fn get_stats(env: &Env) -> Result<SLAStats, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    env.storage()
        .instance()
        .get(&STATS_KEY)
        .ok_or(SLAError::NotInitialized)
}

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

pub fn compute_result(
    outage_id: Symbol,
    mttr_minutes: u32,
    cfg: &SLAConfig,
    config_version_hash: u64,
    recorded_at: u64,
) -> Result<SLAResult, SLAError> {
    let threshold = cfg.threshold_minutes;

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

pub fn record_severity_telemetry(env: &Env, severity: &Symbol, met: bool) {
    let index = crate::SLACalculatorContract::canonical_severity_index(severity).unwrap_or(0);
    let mut calculations = load_counts(env, &SEVERITY_CALC_COUNTS_KEY);
    let mut violations = load_counts(env, &SEVERITY_VIOL_COUNTS_KEY);
    let mut last_calculations = load_counts(env, &LAST_CALCULATION_LEDGER_KEY);
    let mut last_violations = load_counts(env, &LAST_VIOLATION_LEDGER_KEY);

    let now = env.ledger().timestamp();
    let week_seconds = 7u64 * 24u64 * 60u64 * 60u64;
    let last_calc = count_lane(last_calculations, index) as u64;
    let last_violation = count_lane(last_violations, index) as u64;
    let calc_stale = last_calc != 0 && now.saturating_sub(last_calc) >= week_seconds;
    let violation_stale = last_violation != 0 && now.saturating_sub(last_violation) >= week_seconds;
    if calc_stale || violation_stale {
        calculations = set_count_lane(calculations, index, 0);
        violations = set_count_lane(violations, index, 0);
    }

    calculations = set_count_lane(
        calculations,
        index,
        count_lane(calculations, index).saturating_add(1),
    );
    if !met {
        violations = set_count_lane(
            violations,
            index,
            count_lane(violations, index).saturating_add(1),
        );
    }

    let current_ledger = if now > u64::from(u32::MAX) {
        u32::MAX
    } else {
        now as u32
    };
    last_calculations = set_count_lane(last_calculations, index, current_ledger);
    if !met {
        last_violations = set_count_lane(last_violations, index, current_ledger);
    }

    env.storage()
        .instance()
        .set(&SEVERITY_CALC_COUNTS_KEY, &calculations);
    env.storage()
        .instance()
        .set(&SEVERITY_VIOL_COUNTS_KEY, &violations);
    env.storage()
        .instance()
        .set(&LAST_CALCULATION_LEDGER_KEY, &last_calculations);
    env.storage()
        .instance()
        .set(&LAST_VIOLATION_LEDGER_KEY, &last_violations);
}

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
            emit_stats_saturated(
                env,
                symbol_short!("totcalc"),
                stats.total_calculations as i128,
                1,
            );
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
                emit_stats_saturated(
                    env,
                    symbol_short!("totviol"),
                    stats.total_violations as i128,
                    1,
                );
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
            result.payment_type.clone(),
            result.rating.clone(),
            result.mttr_minutes,
            result.threshold_minutes,
            result.amount,
        ),
    );
}

fn publish_settlement_intent_event(env: &Env, severity: Symbol, result: &SLAResult) {
    env.events().publish(
        (EVENT_SETTLE_INTENT, EVENT_VERSION, severity),
        (
            result.outage_id.clone(),
            result.status.clone(),
            result.payment_type.clone(),
            result.amount,
            result.config_version_hash,
            result.recorded_at,
        ),
    );
}
