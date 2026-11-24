//! Policy-snapshot fixtures for the SLA operations workspace.
//!
//! Each function returns a fully-constructed [`SLAConfigSnapshot`] suitable
//! for use in workspace tests. All values are deterministic and stable —
//! changing a fixture is a breaking change to the tests that use it.
//!
//! # Design constraints
//! - `no_std` compatible: only `soroban_sdk` primitives, no `std` imports.
//! - No on-chain storage access; fixtures are pure data constructors.
//! - Threshold/penalty/reward values mirror the defaults from `initialize()`
//!   so that tests can reason about expected outcomes without an env setup.

use soroban_sdk::{symbol_short, Env, Vec};

use crate::{SLAConfig, SLAConfigEntry, SLAConfigSnapshot};

/// Returns a snapshot that reflects the default configuration set during
/// `initialize()`: critical (15 min / 100 pen / 750 rew), high (30 / 50 / 750),
/// medium (60 / 25 / 750), low (120 / 10 / 600).
///
/// Use this fixture when a test needs a valid, complete policy snapshot without
/// spinning up a full contract environment.
pub fn default_policy_snapshot(env: &Env) -> SLAConfigSnapshot {
    let mut entries = Vec::new(env);

    entries.push_back(SLAConfigEntry {
        severity: symbol_short!("critical"),
        config: SLAConfig {
            threshold_minutes: 15,
            penalty_per_minute: 100,
            reward_base: 750,
        },
    });
    entries.push_back(SLAConfigEntry {
        severity: symbol_short!("high"),
        config: SLAConfig {
            threshold_minutes: 30,
            penalty_per_minute: 50,
            reward_base: 750,
        },
    });
    entries.push_back(SLAConfigEntry {
        severity: symbol_short!("medium"),
        config: SLAConfig {
            threshold_minutes: 60,
            penalty_per_minute: 25,
            reward_base: 750,
        },
    });
    entries.push_back(SLAConfigEntry {
        severity: symbol_short!("low"),
        config: SLAConfig {
            threshold_minutes: 120,
            penalty_per_minute: 10,
            reward_base: 600,
        },
    });

    SLAConfigSnapshot {
        version: symbol_short!("v1"),
        entries,
    }
}

/// Returns a snapshot where every severity has the same boundary values
/// (threshold = 1, penalty = 1, reward = 1). Useful for edge-case tests that
/// verify behaviour at the minimum-valid configuration boundary.
pub fn minimal_policy_snapshot(env: &Env) -> SLAConfigSnapshot {
    let mut entries = Vec::new(env);

    for sev in [
        symbol_short!("critical"),
        symbol_short!("high"),
        symbol_short!("medium"),
        symbol_short!("low"),
    ] {
        entries.push_back(SLAConfigEntry {
            severity: sev,
            config: SLAConfig {
                threshold_minutes: 1,
                penalty_per_minute: 1,
                reward_base: 1,
            },
        });
    }

    SLAConfigSnapshot {
        version: symbol_short!("v1"),
        entries,
    }
}

/// Returns a snapshot with only a single severity entry (`critical`).
/// Useful for tests that verify partial-configuration behaviour or
/// single-severity lookup paths.
pub fn single_severity_snapshot(env: &Env) -> SLAConfigSnapshot {
    let mut entries = Vec::new(env);
    entries.push_back(SLAConfigEntry {
        severity: symbol_short!("critical"),
        config: SLAConfig {
            threshold_minutes: 15,
            penalty_per_minute: 100,
            reward_base: 750,
        },
    });

    SLAConfigSnapshot {
        version: symbol_short!("v1"),
        entries,
    }
}
