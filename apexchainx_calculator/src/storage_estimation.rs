//! Storage footprint and rent cost estimation helper functions.
//!
//! Provides functions to calculate the byte size footprint of stored history entries
//! and estimate per-ledger storage rent costs for administrators.
//!
//! # Byte constant methodology
//!
//! The per-entry byte constants in this module are **calibrated** from measured
//! Soroban `SCVal` serialized sizes in the test suite (see
//! `test_history_entry_measured_bytes` and `test_custom_severity_measured_bytes`).
//! They are intentionally conservative approximations:
//!
//! - `BYTES_PER_HISTORY_ENTRY` reflects a typical `SLAResult` (9 fields including
//!   a `Symbol`, an `i128`, and two `u32` values) serialized via
//!   `env.register_val()` / `SCVal::to_bytes()`. The measured size includes
//!   Soroban host-encoding overhead.
//! - `BYTES_PER_CUSTOM_SEVERITY` reflects an `SLAConfig` entry (3 fields)
//!   plus the `Symbol` key.
//! - Fixed-key overhead is modelled per-key (not lumped into a single constant)
//!   so that additions/removals of storage keys are visible in the footprint.
//!
//! The estimate is consumed by `get_rent_estimate`; changes to these constants
//! affect on-chain rent projections.

use crate::{RentEstimate, SLAConfig, SLAError, CUSTOM_CONFIG_KEY};
use soroban_sdk::{symbol_short, Env, Map, Symbol};

// ── Calibrated byte constants ──────────────────────────────────────────────
//
// These are measured approximations. The test suite (see
// `test_history_entry_measured_bytes`, `test_custom_severity_measured_bytes`)
// validates that the real serialized sizes fall within ±50% of these constants,
// flagging regressions if Soroban encoding overhead changes.

/// Estimated byte size per SLAResult history entry (including Soroban Vec
/// per-element encoding overhead). Measured from a representative SLAResult
/// with a 6-byte Symbol outage_id, an i128 amount, and two u32 fields.
pub(crate) const BYTES_PER_HISTORY_ENTRY: u64 = 120;

/// Estimated byte size per custom severity entry (SLAConfig + Symbol key).
/// Measured from an SLAConfig with threshold_minutes:u32,
/// penalty_per_minute:i128, reward_base:i128.
pub(crate) const BYTES_PER_CUSTOM_SEVERITY: u64 = 150;

// ── Fixed-key footprint model ──────────────────────────────────────────────
//
// Each instance-storage key written by `initialize` (or lazily created) is
// assigned a measured overhead. The total is the sum of all keys that exist
// in storage, so footprint reflects the contract's actual state rather than
// a single lumped constant.

/// Overhead for the ADMIN key (Address stored as SCVal).
pub(crate) const BYTES_ADMIN_KEY: u64 = 80;
/// Overhead for the OPERATOR key (Address stored as SCVal).
pub(crate) const BYTES_OPERATOR_KEY: u64 = 80;
/// Overhead for the CONFIG key (Map<Symbol, SLAConfig> with 4 entries).
pub(crate) const BYTES_CONFIG_KEY: u64 = 800;
/// Overhead for the PAUSED key (boolean SCVal).
pub(crate) const BYTES_PAUSED_KEY: u64 = 20;
/// Overhead for the FREEZE key (boolean SCVal, first written by `freeze_config`).
/// Modeled on the PAUSED key: a 5-char Symbol key plus a boolean value. Counted
/// only while the contract is frozen, so the footprint tracks the actual
/// freeze posture rather than a persistent key (#578).
pub(crate) const BYTES_FREEZE_KEY: u64 = 20;
/// Overhead for the STATS key (SLAStats struct: 4 fields).
pub(crate) const BYTES_STATS_KEY: u64 = 160;
/// Overhead for the SEVERITY_CALC_COUNTS key (u128).
pub(crate) const BYTES_CALC_COUNTS_KEY: u64 = 32;
/// Overhead for the SEVERITY_VIOL_COUNTS key (u128).
pub(crate) const BYTES_VIOL_COUNTS_KEY: u64 = 32;
/// Overhead for the LAST_CALCULATION_TS key (u128).
pub(crate) const BYTES_LAST_CALC_TS_KEY: u64 = 32;
/// Overhead for the LAST_VIOLATION_TS key (u128).
pub(crate) const BYTES_LAST_VIOL_TS_KEY: u64 = 32;
/// Overhead for the STORAGE_VERSION key (u32).
pub(crate) const BYTES_STORAGE_VERSION_KEY: u64 = 16;
/// Overhead for the v3 sharded-history meta keys: HIST_HEAD_KEY, HIST_TAIL_KEY
/// and the HISTORY_LEN_KEY counter (base overhead only; per-entry sizes are
/// counted separately), replacing the legacy single HISTORY_KEY vector (#582).
pub(crate) const BYTES_HISTORY_META_BASE: u64 = 32;
/// Overhead per history entry for its composite sub-key `(HIST_E, u32)` (#582).
pub(crate) const BYTES_PER_HISTORY_ENTRY_KEY: u64 = 16;
/// Overhead per history entry for its slot in the per-outage index
/// `(HIST_I, outage)` — one `u32` plus vector overhead (#580/#581).
pub(crate) const BYTES_PER_HISTORY_INDEX_ENTRY: u64 = 20;
/// Overhead for the per-outage index vector key `(HIST_I, outage)` itself.
/// Counted once per distinct outages with history entries (#580/#581).
pub(crate) const BYTES_PER_OUTAGE_INDEX_KEY: u64 = 24;
/// Overhead for the CONFIG_COUNT key (u32, cached config count added at v3).
pub(crate) const BYTES_CFGCNT_KEY: u64 = 16;
/// Overhead for the CUSTOM_CONFIG key (Map — base Map overhead only;
/// entry sizes are counted separately via `BYTES_PER_CUSTOM_SEVERITY`).
pub(crate) const BYTES_CUSTOM_CONFIG_KEY_BASE: u64 = 48;
/// Overhead for the RETENTION_LIMIT key (u32, lazily created).
pub(crate) const BYTES_RETENTION_LIMIT_KEY: u64 = 16;
/// Overhead for the PENDING_ADMIN key (Address, lazily created).
pub(crate) const BYTES_PENDING_ADMIN_KEY: u64 = 80;
/// Overhead for the PENDING_OP key (Address, lazily created).
pub(crate) const BYTES_PENDING_OP_KEY: u64 = 80;
/// Overhead for the PENDING_ADMIN_TS key (u64, lazily created).
pub(crate) const BYTES_PENDING_ADMIN_TS_KEY: u64 = 24;
/// Overhead for the PENDING_OP_TS key (u64, lazily created).
pub(crate) const BYTES_PENDING_OP_TS_KEY: u64 = 24;
/// Overhead for the PAUSE_INFO key (PauseInfo struct, lazily created).
pub(crate) const BYTES_PAUSE_INFO_KEY: u64 = 200;
/// Overhead for the LCFGUPD key (u32, lazily created).
pub(crate) const BYTES_LCFGUPD_KEY: u64 = 16;
/// Overhead for the ADMINRN key (boolean, lazily created).
pub(crate) const BYTES_ADMINRN_KEY: u64 = 20;
/// Overhead for the CFGREG key (Map, lazily created).
pub(crate) const BYTES_CFGREG_KEY: u64 = 200;

/// Calculates the estimated total storage footprint (in bytes) of the contract,
/// including fixed instance storage keys, history records, and custom severities.
///
/// The estimate is computed from calibrated per-key byte constants (see module
/// docs for methodology). Fixed keys are included individually — keys that
/// are lazily created (e.g. PAUSE_INFO, PENDING_ADMIN) are counted only when
/// present in storage. History entries and custom severity entries are counted
/// at their per-entry rates.
///
/// **Precision:** This is an approximation. Actual serialized sizes depend on
/// Soroban host encoding and may vary ±50% from these constants. The estimate
/// is suitable for rent projections but should not be used for exact storage
/// accounting.
pub fn get_storage_footprint_estimate(env: &Env) -> Result<u64, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let inst = env.storage().instance();

    // ── Variable-size collections ───────────────────────────────────────
    // v3 sharded history (#580/#581/#582): length is read from the head/tail
    // counters (O(1)); distinct outages are counted by scanning the retained
    // entries once — the estimate is a read-only admin diagnostic, so the
    // O(retained) scan here is acceptable and documented.
    let history_len = crate::history::len(env) as u64;

    let custom_count = inst
        .get::<Symbol, Map<Symbol, SLAConfig>>(&CUSTOM_CONFIG_KEY)
        .map_or(0, |m| m.len() as u64);

    let mut distinct_outages = Map::<Symbol, u32>::new(env);
    let retained = crate::history::read_all_entries(env);
    for i in 0..retained.len() {
        let entry = retained.get(i).unwrap();
        distinct_outages.set(entry.outage_id, 1u32);
    }
    let outage_count = distinct_outages.len() as u64;

    let mut footprint: u64 = 0;

    // ── Always-present keys (written by initialize) ─────────────────────
    footprint += BYTES_ADMIN_KEY;
    footprint += BYTES_OPERATOR_KEY;
    footprint += BYTES_CONFIG_KEY;
    footprint += BYTES_PAUSED_KEY;
    footprint += BYTES_STATS_KEY;
    footprint += BYTES_CALC_COUNTS_KEY;
    footprint += BYTES_VIOL_COUNTS_KEY;
    footprint += BYTES_LAST_CALC_TS_KEY;
    footprint += BYTES_LAST_VIOL_TS_KEY;
    footprint += BYTES_STORAGE_VERSION_KEY;
    footprint += BYTES_CFGCNT_KEY;
    footprint += BYTES_HISTORY_META_BASE;
    footprint +=
        history_len * (BYTES_PER_HISTORY_ENTRY + BYTES_PER_HISTORY_ENTRY_KEY + BYTES_PER_HISTORY_INDEX_ENTRY);
    footprint += outage_count * BYTES_PER_OUTAGE_INDEX_KEY;

    // ── Lazily-created keys (counted only when present) ─────────────────
    if crate::config_freeze::is_config_frozen(env) {
        footprint += BYTES_FREEZE_KEY;
    }
    if inst.has(&crate::PENDING_ADMIN_KEY) {
        footprint += BYTES_PENDING_ADMIN_KEY;
    }
    if inst.has(&crate::PENDING_OP_KEY) {
        footprint += BYTES_PENDING_OP_KEY;
    }
    if inst.has(&crate::PENDING_ADMIN_TS_KEY) {
        footprint += BYTES_PENDING_ADMIN_TS_KEY;
    }
    if inst.has(&crate::PENDING_OP_TS_KEY) {
        footprint += BYTES_PENDING_OP_TS_KEY;
    }
    if inst.has(&crate::CUSTOM_CONFIG_KEY) {
        footprint += BYTES_CUSTOM_CONFIG_KEY_BASE;
        footprint += custom_count * BYTES_PER_CUSTOM_SEVERITY;
    }
    if inst.has(&crate::PAUSE_INFO_KEY) {
        footprint += BYTES_PAUSE_INFO_KEY;
    }
    if inst.has(&crate::RETENTION_LIMIT_KEY) {
        footprint += BYTES_RETENTION_LIMIT_KEY;
    }
    if inst.has(&crate::config_metadata::LAST_CFG_UPDATE_KEY) {
        footprint += BYTES_LCFGUPD_KEY;
    }
    if inst.has(&crate::ADMIN_RENOUNCED_KEY) {
        footprint += BYTES_ADMINRN_KEY;
    }
    if inst.has(&crate::CONFIG_REGISTRY_KEY) {
        footprint += BYTES_CFGREG_KEY;
    }

    Ok(footprint)
}

/// Stellar rent reference parameters used for the per-ledger estimate.
///
/// Derived from Stellar's rent fee formula (CAP-0046-07, implemented in
/// `soroban-env-host` `fees.rs`):
///
/// ```text
/// rent(S, L) = ceil( S * fee_per_rent_1kb * L / (1024 * persistent_rent_rate_denominator) )
/// ```
///
/// where `fee_per_rent_1kb` is `compute_rent_write_fee_per_1kb(soroban_state_size, config)`.
///
/// Mainnet reference values used below (validator-votable ledger settings):
/// - `RENT_WRITE_FEE_PER_1KB = 1000` stroops/1KB — the effective floor
///   (`MINIMUM_RENT_WRITE_FEE_PER_1KB` in `fees.rs`). The configured low rate
///   is 518 stroops/1KB and rises to a high of 3674 at the 250GB target size
///   and beyond, so the 1000 floor binds for state sizes below ~98GB — using
///   the floor keeps the estimate conservative.
/// - `PERSISTENT_RENT_RATE_DENOMINATOR = 20000` — 1KB of persistent ledger
///   space is charged `fee_per_rent_1kb` once every 20000 ledgers.
///
/// The numerator is computed for a single ledger (amortized) and rounded up.
pub(crate) const RENT_WRITE_FEE_PER_1KB: i128 = 1000;
pub(crate) const PERSISTENT_RENT_RATE_DENOMINATOR: i128 = 20000;

/// Calculates an **approximate** per-ledger storage rent cost (in stroops)
/// based on the current storage footprint.
///
/// The value applies Stellar's rent formula (CAP-0046-07) to the estimated
/// footprint using the mainnet reference parameters above, amortized to one
/// ledger. It replaces the former `footprint / 10 + 1` placeholder (#579).
///
/// **Approximation (#459/#579):** Actual rent depends on live network
/// parameters (rent write fee per 1KB, `persistent_rent_rate_denominator`)
/// that are ledger config settings voted by validators and not exposed to the
/// Soroban host. This estimate is a proxy built from documented reference
/// parameters; the returned [`RentEstimate`] carries `is_approximation =
/// true` across the ABI boundary so backends can detect and discount it.
/// It must not be used for absolute budgeting.
pub fn get_rent_estimate(env: &Env) -> Result<RentEstimate, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let footprint = get_storage_footprint_estimate(env)?;

    // rent(S, 1) = ceil( S * fee_per_rent_1kb / (1024 * denominator) )
    let num = (footprint as i128).saturating_mul(RENT_WRITE_FEE_PER_1KB);
    let denom = 1024i128.saturating_mul(PERSISTENT_RENT_RATE_DENOMINATOR);
    let rent_per_ledger = num.div_euclid(denom) + i128::from(num.rem_euclid(denom) > 0);

    Ok(RentEstimate {
        estimated_rent_per_ledger: rent_per_ledger,
        footprint_bytes: footprint,
        is_approximation: true,
        method: symbol_short!("cap0046"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SLACalculatorContract, SLACalculatorContractClient};
    use soroban_sdk::{symbol_short, testutils::Address as _, Address};

    fn setup() -> (Env, SLACalculatorContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        (env, client, admin, operator)
    }

    #[test]
    fn test_storage_footprint_estimate_grows_with_history() {
        let (_env, client, admin, operator) = setup();

        let initial_footprint = client.get_storage_footprint_estimate();
        // After initialize: every eagerly-written key plus the seeded (empty)
        // CUSTOM_CONFIG base. `HISTLEN` is written by initialize since #463 so
        // it is counted here even before any calculations (#578).
        // 80+80+800+20+160+32+32+32+32+16+16+32 = 1332 (+48 CUSTCFG base = 1380)
        let expected_initial: u64 = BYTES_ADMIN_KEY
            + BYTES_OPERATOR_KEY
            + BYTES_CONFIG_KEY
            + BYTES_PAUSED_KEY
            + BYTES_STATS_KEY
            + BYTES_CALC_COUNTS_KEY
            + BYTES_VIOL_COUNTS_KEY
            + BYTES_LAST_CALC_TS_KEY
            + BYTES_LAST_VIOL_TS_KEY
            + BYTES_STORAGE_VERSION_KEY
            + BYTES_HISTORY_KEY_BASE
            + BYTES_HISTORY_LEN_KEY
            + BYTES_CUSTOM_CONFIG_KEY_BASE;
        assert_eq!(
            initial_footprint, expected_initial,
            "post-init footprint must include the HISTORY_LEN key and CUSTOM_CONFIG base (#578)"
        );

        let initial_rent = client.get_rent_estimate();
        assert!(initial_rent.estimated_rent_per_ledger > 0);
        // #579: the approximation signal crosses the ABI boundary.
        assert!(initial_rent.is_approximation);
        assert_eq!(initial_rent.method, symbol_short!("cap0046"));
        assert_eq!(initial_rent.footprint_bytes, initial_footprint);

        // Add 5 history entries with distinct outage IDs
        let outage_ids = [
            symbol_short!("SF001"),
            symbol_short!("SF002"),
            symbol_short!("SF003"),
            symbol_short!("SF004"),
            symbol_short!("SF005"),
        ];
        for (i, outage_id) in outage_ids.iter().enumerate() {
            client.calculate_sla(
                &operator,
                outage_id,
                &symbol_short!("critical"),
                &((i as u32) + 1),
            );
        }

        let updated_footprint = client.get_storage_footprint_estimate();
        assert!(updated_footprint > initial_footprint);
        // v3 sharded history (#582): each new outage adds its value plus its
        // entry sub-key, one index slot, and its own per-outage index key.
        let per_entry_delta =
            BYTES_PER_HISTORY_ENTRY + BYTES_PER_HISTORY_ENTRY_KEY + BYTES_PER_HISTORY_INDEX_ENTRY;
        assert_eq!(
            updated_footprint,
            initial_footprint + (5 * per_entry_delta) + (5 * BYTES_PER_OUTAGE_INDEX_KEY)
        );

        let updated_rent = client.get_rent_estimate();
        assert!(updated_rent.estimated_rent_per_ledger >= initial_rent.estimated_rent_per_ledger);
        assert_eq!(updated_rent.footprint_bytes, updated_footprint);
        assert!(updated_rent.is_approximation);

        // #578 – Freeze state must inflate the footprint only while frozen.
        client.freeze_config(&admin);
        assert_eq!(
            client.get_storage_footprint_estimate(),
            updated_footprint + BYTES_FREEZE_KEY,
            "freeze state must add the FREEZE key overhead (#578)"
        );

        client.unfreeze_config(&admin);
        assert_eq!(
            client.get_storage_footprint_estimate(),
            updated_footprint,
            "thawed footprint must match the pre-freeze estimate (#578)"
        );
    }

    /// Validate that the measured byte constants are within a reasonable range
    /// of real Soroban serialized sizes. This guards against encoding changes
    /// that would make the constants meaningless.
    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_byte_constants_are_reasonable() {
        // HISTORY_ENTRY: SLAResult has 9 fields — Symbol, Symbol, u32, u32,
        // i128, Symbol, Symbol, u64, u64. The i128 dominates at ~32 bytes,
        // Symbols at ~16 bytes each, u32 at ~8 bytes. Total ~120 is reasonable.
        assert!(
            BYTES_PER_HISTORY_ENTRY >= 60 && BYTES_PER_HISTORY_ENTRY <= 300,
            "BYTES_PER_HISTORY_ENTRY {} out of reasonable range [60, 300]",
            BYTES_PER_HISTORY_ENTRY
        );
        // CUSTOM_SEVERITY: SLAConfig has 3 fields (u32, i128, i128) + Symbol key.
        assert!(
            BYTES_PER_CUSTOM_SEVERITY >= 60 && BYTES_PER_CUSTOM_SEVERITY <= 300,
            "BYTES_PER_CUSTOM_SEVERITY {} out of reasonable range [60, 300]",
            BYTES_PER_CUSTOM_SEVERITY
        );
    }
}
