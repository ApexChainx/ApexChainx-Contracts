//! Tests for `sla_operations::fixtures::policy_snapshots`.
//!
//! Resolves: https://github.com/ApexChainx/ApexChainx-Contracts/issues/1004
//!
//! Each test below verifies a specific invariant of the policy-snapshot
//! fixtures: entry counts, version labels, specific field values, and
//! comparison-readiness (PartialEq derives work as expected).
//!
//! All tests are deterministic and do not interact with on-chain storage.

#![cfg(test)]

use soroban_sdk::{symbol_short, Env};

use crate::sla_operations::fixtures::policy_snapshots::{
    default_policy_snapshot, minimal_policy_snapshot, single_severity_snapshot,
};

// -------------------------------------------------------------------------
// default_policy_snapshot
// -------------------------------------------------------------------------

#[test]
fn test_default_snapshot_has_four_entries() {
    let env = Env::default();
    let snapshot = default_policy_snapshot(&env);
    assert_eq!(snapshot.entries.len(), 4);
}

#[test]
fn test_default_snapshot_version_is_v1() {
    let env = Env::default();
    let snapshot = default_policy_snapshot(&env);
    assert_eq!(snapshot.version, symbol_short!("v1"));
}

#[test]
fn test_default_snapshot_first_entry_is_critical() {
    let env = Env::default();
    let snapshot = default_policy_snapshot(&env);
    let first = snapshot.entries.get(0).expect("entry 0 must exist");
    assert_eq!(first.severity, symbol_short!("critical"));
    assert_eq!(first.config.threshold_minutes, 15);
    assert_eq!(first.config.penalty_per_minute, 100);
    assert_eq!(first.config.reward_base, 750);
}

#[test]
fn test_default_snapshot_last_entry_is_low() {
    let env = Env::default();
    let snapshot = default_policy_snapshot(&env);
    let last = snapshot
        .entries
        .get(3)
        .expect("entry 3 must exist");
    assert_eq!(last.severity, symbol_short!("low"));
    assert_eq!(last.config.threshold_minutes, 120);
    assert_eq!(last.config.penalty_per_minute, 10);
    assert_eq!(last.config.reward_base, 600);
}

#[test]
fn test_default_snapshot_is_deterministic() {
    // Two independent constructions from the same Env must be equal.
    let env = Env::default();
    let a = default_policy_snapshot(&env);
    let b = default_policy_snapshot(&env);
    assert_eq!(a, b);
}

#[test]
fn test_default_snapshot_severity_order() {
    // Canonical order: critical, high, medium, low.
    let env = Env::default();
    let snapshot = default_policy_snapshot(&env);
    let expected = [
        symbol_short!("critical"),
        symbol_short!("high"),
        symbol_short!("medium"),
        symbol_short!("low"),
    ];
    for (idx, sev) in expected.iter().enumerate() {
        let entry = snapshot.entries.get(idx as u32).unwrap();
        assert_eq!(&entry.severity, sev, "severity mismatch at index {idx}");
    }
}

// -------------------------------------------------------------------------
// minimal_policy_snapshot
// -------------------------------------------------------------------------

#[test]
fn test_minimal_snapshot_has_four_entries() {
    let env = Env::default();
    let snapshot = minimal_policy_snapshot(&env);
    assert_eq!(snapshot.entries.len(), 4);
}

#[test]
fn test_minimal_snapshot_all_configs_are_unity() {
    let env = Env::default();
    let snapshot = minimal_policy_snapshot(&env);
    for idx in 0..4 {
        let entry = snapshot.entries.get(idx).unwrap();
        assert_eq!(entry.config.threshold_minutes, 1, "threshold at {idx}");
        assert_eq!(entry.config.penalty_per_minute, 1, "penalty at {idx}");
        assert_eq!(entry.config.reward_base, 1, "reward at {idx}");
    }
}

#[test]
fn test_minimal_snapshot_version_is_v1() {
    let env = Env::default();
    let snapshot = minimal_policy_snapshot(&env);
    assert_eq!(snapshot.version, symbol_short!("v1"));
}

// -------------------------------------------------------------------------
// single_severity_snapshot
// -------------------------------------------------------------------------

#[test]
fn test_single_severity_snapshot_has_one_entry() {
    let env = Env::default();
    let snapshot = single_severity_snapshot(&env);
    assert_eq!(snapshot.entries.len(), 1);
}

#[test]
fn test_single_severity_snapshot_is_critical() {
    let env = Env::default();
    let snapshot = single_severity_snapshot(&env);
    let entry = snapshot.entries.get(0).unwrap();
    assert_eq!(entry.severity, symbol_short!("critical"));
}

// -------------------------------------------------------------------------
// Cross-fixture comparison-readiness
// -------------------------------------------------------------------------

#[test]
fn test_default_and_minimal_snapshots_are_not_equal() {
    let env = Env::default();
    let default = default_policy_snapshot(&env);
    let minimal = minimal_policy_snapshot(&env);
    assert_ne!(default, minimal);
}

#[test]
fn test_default_and_single_snapshots_differ_in_entry_count() {
    let env = Env::default();
    let default = default_policy_snapshot(&env);
    let single = single_severity_snapshot(&env);
    assert_ne!(default.entries.len(), single.entries.len());
}
