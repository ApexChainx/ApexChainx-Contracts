//! Report-expectation fixtures for the SLA operations workspace.
//!
//! Each function returns a fully-constructed [`SLAResult`] that represents the
//! expected output of a known calculation. Tests compare actual contract output
//! against these expectations to confirm deterministic behaviour.
//!
//! # Naming convention
//!
//! Functions are named `<severity>_<outcome>_expectation`, e.g.
//! `critical_violation_expectation` or `high_met_expectation`.
//!
//! # Design constraints
//! - `no_std` compatible: only `soroban_sdk` primitives, no `std` imports.
//! - Values are derived from the default configuration (see `policy_snapshots`).
//! - `config_version_hash` and `recorded_at` are set to `0` in fixtures because
//!   they are environment-dependent; tests that need them must override the fields
//!   after constructing the fixture.

use soroban_sdk::symbol_short;

use crate::SLAResult;

/// Expected result for a **critical** outage that **violated** the SLA.
///
/// Assumptions:
/// - `mttr_minutes` = 20 (5 minutes over the 15-minute threshold).
/// - `penalty_per_minute` = 100, so `amount` = -(5 × 100) = -500.
/// - `rating` = "poor" (violation always maps to "poor").
pub fn critical_violation_expectation() -> SLAResult {
    SLAResult {
        outage_id: symbol_short!("OT001"),
        status: symbol_short!("viol"),
        mttr_minutes: 20,
        threshold_minutes: 15,
        amount: -500,
        payment_type: symbol_short!("pen"),
        rating: symbol_short!("poor"),
        config_version_hash: 0,
        recorded_at: 0,
    }
}

/// Expected result for a **critical** outage that **met** the SLA.
///
/// Assumptions:
/// - `mttr_minutes` = 10 (5 minutes under the 15-minute threshold).
/// - `reward_base` = 750, so `amount` = 750.
/// - `rating` = "top" (critical severity met with large margin maps to "top").
pub fn critical_met_expectation() -> SLAResult {
    SLAResult {
        outage_id: symbol_short!("OT002"),
        status: symbol_short!("met"),
        mttr_minutes: 10,
        threshold_minutes: 15,
        amount: 750,
        payment_type: symbol_short!("rew"),
        rating: symbol_short!("top"),
        config_version_hash: 0,
        recorded_at: 0,
    }
}

/// Expected result for a **high** outage that **met** the SLA exactly at
/// the threshold boundary (`mttr_minutes` == `threshold_minutes`).
///
/// Assumptions:
/// - `mttr_minutes` = 30 (exactly the 30-minute threshold → "met").
/// - `reward_base` = 750, so `amount` = 750.
/// - `rating` = "good" (met exactly at boundary maps to "good").
pub fn high_met_at_boundary_expectation() -> SLAResult {
    SLAResult {
        outage_id: symbol_short!("OT003"),
        status: symbol_short!("met"),
        mttr_minutes: 30,
        threshold_minutes: 30,
        amount: 750,
        payment_type: symbol_short!("rew"),
        rating: symbol_short!("good"),
        config_version_hash: 0,
        recorded_at: 0,
    }
}

/// Expected result for a **low** outage that **violated** the SLA.
///
/// Assumptions:
/// - `mttr_minutes` = 150 (30 minutes over the 120-minute threshold).
/// - `penalty_per_minute` = 10, so `amount` = -(30 × 10) = -300.
/// - `rating` = "poor".
pub fn low_violation_expectation() -> SLAResult {
    SLAResult {
        outage_id: symbol_short!("OT004"),
        status: symbol_short!("viol"),
        mttr_minutes: 150,
        threshold_minutes: 120,
        amount: -300,
        payment_type: symbol_short!("pen"),
        rating: symbol_short!("poor"),
        config_version_hash: 0,
        recorded_at: 0,
    }
}
