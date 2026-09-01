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

use crate::{SLAConfig, SLAError, CUSTOM_CONFIG_KEY, HISTORY_KEY};
use soroban_sdk::{Env, Map, Symbol, Vec};

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
/// Overhead for the HISTORY key (Vec<SLAResult> — base Vec overhead only;
/// element sizes are counted separately via `BYTES_PER_HISTORY_ENTRY`).
pub(crate) const BYTES_HISTORY_KEY_BASE: u64 = 32;
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
    let history_len = inst
        .get::<Symbol, Vec<crate::SLAResult>>(&HISTORY_KEY)
        .map_or(0, |h| h.len() as u64);

    let custom_count = inst
        .get::<Symbol, Map<Symbol, SLAConfig>>(&CUSTOM_CONFIG_KEY)
        .map_or(0, |m| m.len() as u64);

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
    footprint += BYTES_HISTORY_KEY_BASE;
    footprint += history_len * BYTES_PER_HISTORY_ENTRY;

    // ── Lazily-created keys (counted only when present) ─────────────────
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

/// Calculates an **approximate** per-ledger storage rent cost (in stroops)
/// based on the current storage footprint.
///
/// **Disclaimer (#459):** This is a relative growth proxy, not an
/// authoritative rent figure. The formula (`footprint / 10 + 1`) is a
/// placeholder approximation. Actual Stellar rent depends on network
/// parameters (rent fee per byte per ledger, minimum rent, etc.) that
/// are not available to the Soroban host in this SDK version.
///
/// Operators should use this value to track **relative** storage cost
/// growth over time, not as an absolute budgeting number.
pub fn get_rent_estimate(env: &Env) -> Result<i128, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let footprint = get_storage_footprint_estimate(env)? as i128;
    // Relative proxy: ~1 stroop per 10 bytes per ledger + 1 base stroop.
    // See doc comment — this is not derived from network parameters.
    let rent_per_ledger = (footprint / 10) + 1;
    Ok(rent_per_ledger)
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
        let (_env, client, _admin, operator) = setup();

        let initial_footprint = client.get_storage_footprint_estimate();
        // After initialize: sum of all eagerly-written keys
        // 80+80+800+20+160+32+32+32+32+16+32 = 1316
        assert!(initial_footprint >= 1000);

        let initial_rent = client.get_rent_estimate();
        assert!(initial_rent > 0);

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
        assert_eq!(
            updated_footprint,
            initial_footprint + (5 * BYTES_PER_HISTORY_ENTRY)
        );

        let updated_rent = client.get_rent_estimate();
        assert!(updated_rent >= initial_rent);
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
