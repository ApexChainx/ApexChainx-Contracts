//! Tests for `sla_operations::fixtures::report_expectations`.
//!
//! Resolves: https://github.com/ApexChainx/ApexChainx-Contracts/issues/1006
//!
//! Each test verifies the correctness and consistency of the report-expectation
//! fixtures: field values match the documented assumptions, payment types and
//! statuses are consistent, and fixtures are suitable for direct comparison
//! with contract output (PartialEq).
//!
//! All tests are deterministic and do not interact with on-chain storage.

#![cfg(test)]

use soroban_sdk::symbol_short;

use crate::sla_operations::fixtures::report_expectations::{
    critical_met_expectation, critical_violation_expectation, high_met_at_boundary_expectation,
    low_violation_expectation,
};

// -------------------------------------------------------------------------
// critical_violation_expectation
// -------------------------------------------------------------------------

#[test]
fn test_critical_violation_status_is_viol() {
    let result = critical_violation_expectation();
    assert_eq!(result.status, symbol_short!("viol"));
}

#[test]
fn test_critical_violation_payment_type_is_pen() {
    let result = critical_violation_expectation();
    assert_eq!(result.payment_type, symbol_short!("pen"));
}

#[test]
fn test_critical_violation_amount_is_negative() {
    let result = critical_violation_expectation();
    assert!(result.amount < 0, "penalty amount must be negative");
}

#[test]
fn test_critical_violation_amount_matches_calculation() {
    // mttr=20, threshold=15, penalty_per_minute=100 → -(5×100)=-500
    let result = critical_violation_expectation();
    assert_eq!(result.mttr_minutes, 20);
    assert_eq!(result.threshold_minutes, 15);
    let overtime = (result.mttr_minutes - result.threshold_minutes) as i128;
    assert_eq!(result.amount, -(overtime * 100));
}

#[test]
fn test_critical_violation_rating_is_poor() {
    let result = critical_violation_expectation();
    assert_eq!(result.rating, symbol_short!("poor"));
}

#[test]
fn test_critical_violation_is_deterministic() {
    let a = critical_violation_expectation();
    let b = critical_violation_expectation();
    assert_eq!(a, b);
}

// -------------------------------------------------------------------------
// critical_met_expectation
// -------------------------------------------------------------------------

#[test]
fn test_critical_met_status_is_met() {
    let result = critical_met_expectation();
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
fn test_critical_met_payment_type_is_rew() {
    let result = critical_met_expectation();
    assert_eq!(result.payment_type, symbol_short!("rew"));
}

#[test]
fn test_critical_met_amount_is_positive() {
    let result = critical_met_expectation();
    assert!(result.amount > 0, "reward amount must be positive");
}

#[test]
fn test_critical_met_amount_matches_reward_base() {
    let result = critical_met_expectation();
    assert_eq!(result.amount, 750);
}

#[test]
fn test_critical_met_mttr_is_below_threshold() {
    let result = critical_met_expectation();
    assert!(
        result.mttr_minutes < result.threshold_minutes,
        "mttr must be below threshold for a met SLA"
    );
}

// -------------------------------------------------------------------------
// high_met_at_boundary_expectation
// -------------------------------------------------------------------------

#[test]
fn test_high_boundary_status_is_met() {
    let result = high_met_at_boundary_expectation();
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
fn test_high_boundary_mttr_equals_threshold() {
    // Boundary condition: mttr == threshold still counts as "met".
    let result = high_met_at_boundary_expectation();
    assert_eq!(result.mttr_minutes, result.threshold_minutes);
}

#[test]
fn test_high_boundary_payment_type_is_rew() {
    let result = high_met_at_boundary_expectation();
    assert_eq!(result.payment_type, symbol_short!("rew"));
}

// -------------------------------------------------------------------------
// low_violation_expectation
// -------------------------------------------------------------------------

#[test]
fn test_low_violation_status_is_viol() {
    let result = low_violation_expectation();
    assert_eq!(result.status, symbol_short!("viol"));
}

#[test]
fn test_low_violation_amount_matches_calculation() {
    // mttr=150, threshold=120, penalty_per_minute=10 → -(30×10)=-300
    let result = low_violation_expectation();
    assert_eq!(result.mttr_minutes, 150);
    assert_eq!(result.threshold_minutes, 120);
    let overtime = (result.mttr_minutes - result.threshold_minutes) as i128;
    assert_eq!(result.amount, -(overtime * 10));
}

#[test]
fn test_low_violation_rating_is_poor() {
    let result = low_violation_expectation();
    assert_eq!(result.rating, symbol_short!("poor"));
}

// -------------------------------------------------------------------------
// Cross-fixture consistency checks
// -------------------------------------------------------------------------

#[test]
fn test_violation_fixtures_all_have_negative_amounts() {
    for amount in [
        critical_violation_expectation().amount,
        low_violation_expectation().amount,
    ] {
        assert!(amount < 0, "violation fixture must have negative amount");
    }
}

#[test]
fn test_met_fixtures_all_have_positive_amounts() {
    for amount in [
        critical_met_expectation().amount,
        high_met_at_boundary_expectation().amount,
    ] {
        assert!(amount > 0, "met fixture must have positive amount");
    }
}

#[test]
fn test_all_fixtures_have_non_zero_threshold() {
    for threshold in [
        critical_violation_expectation().threshold_minutes,
        critical_met_expectation().threshold_minutes,
        high_met_at_boundary_expectation().threshold_minutes,
        low_violation_expectation().threshold_minutes,
    ] {
        assert!(threshold > 0, "threshold must always be non-zero");
    }
}

#[test]
fn test_violation_and_met_critical_fixtures_are_not_equal() {
    let viol = critical_violation_expectation();
    let met = critical_met_expectation();
    assert_ne!(viol, met);
}
