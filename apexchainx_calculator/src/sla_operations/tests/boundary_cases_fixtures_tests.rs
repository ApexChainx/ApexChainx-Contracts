//! Tests for `fixtures::boundary_cases`.
//!
//! Boundary fixtures encode the two rules the workspace relies on:
//! `met <=> mttr_minutes <= threshold_minutes` and truncated basis points with
//! a zero result for a zero denominator. Expectations are asserted literally so
//! a change in the rule shows up as a test failure rather than silently
//! re-deriving itself.

use crate::sla_operations::fixtures::boundary_cases::{mttr_cases, ratio_cases};
use alloc::{string::String, vec, vec::Vec};

const DOCUMENTED_CASES: [(&str, u32, u32, bool); 9] = [
    ("zero_mttr", 0, 15, true),
    ("one_under", 14, 15, true),
    ("exactly_at_threshold", 15, 15, true),
    ("one_over", 16, 15, false),
    ("min_threshold_met", 1, 1, true),
    ("min_threshold_violated", 2, 1, false),
    ("max_mttr", u32::MAX, 15, false),
    ("max_threshold", 15, u32::MAX, true),
    ("both_max", u32::MAX, u32::MAX, true),
];

const DOCUMENTED_RATIOS: [(u32, u32, u32); 5] = [
    (0, 10, 0),
    (1, 3, 3_333),
    (2, 3, 6_666),
    (10, 10, 10_000),
    (5, 0, 0),
];

fn names() -> Vec<&'static str> {
    mttr_cases().iter().map(|c| c.name).collect()
}

fn explicit_expectation(name: &str) -> Option<bool> {
    DOCUMENTED_CASES
        .iter()
        .find(|(case_name, _, _, _)| *case_name == name)
        .map(|(_, _, _, expected)| *expected)
}

#[test]
fn mttr_cases_have_the_documented_names_in_order() {
    assert_eq!(
        names(),
        vec![
            "zero_mttr",
            "one_under",
            "exactly_at_threshold",
            "one_over",
            "min_threshold_met",
            "min_threshold_violated",
            "max_mttr",
            "max_threshold",
            "both_max",
        ]
    );
}

#[test]
fn mttr_cases_carry_the_documented_values() {
    let cases = mttr_cases();

    assert_eq!(cases.len(), DOCUMENTED_CASES.len());
    for (case, (name, mttr, threshold, expect_met)) in cases.iter().zip(DOCUMENTED_CASES) {
        assert_eq!(case.name, name);
        assert_eq!(case.mttr_minutes, mttr, "{name}");
        assert_eq!(case.threshold_minutes, threshold, "{name}");
        assert_eq!(case.expect_met, expect_met, "{name}");
    }
}

#[test]
fn every_case_matches_its_explicit_expectation() {
    for case in mttr_cases() {
        let expected = explicit_expectation(case.name).expect("case is documented");

        assert_eq!(case.expect_met, expected, "{}", case.name);
    }
}

#[test]
fn expectations_follow_the_inclusive_mttr_rule() {
    for case in mttr_cases() {
        assert_eq!(
            case.expect_met,
            case.mttr_minutes <= case.threshold_minutes,
            "{}",
            case.name
        );
    }
}

#[test]
fn the_threshold_itself_is_met() {
    let at_threshold = mttr_cases()
        .into_iter()
        .find(|c| c.name == "exactly_at_threshold")
        .expect("exactly_at_threshold case");

    assert_eq!(at_threshold.mttr_minutes, at_threshold.threshold_minutes);
    assert!(at_threshold.expect_met);
}

#[test]
fn one_minute_over_the_threshold_is_violated() {
    let over = mttr_cases()
        .into_iter()
        .find(|c| c.name == "one_over")
        .expect("one_over case");

    assert_eq!(over.mttr_minutes, over.threshold_minutes + 1);
    assert!(!over.expect_met);
}

#[test]
fn zero_and_maximum_values_are_covered() {
    let cases = mttr_cases();

    assert!(cases.iter().any(|c| c.mttr_minutes == 0));
    assert!(cases
        .iter()
        .any(|c| c.threshold_minutes == 0 || c.threshold_minutes == u32::MAX));
    assert!(cases
        .iter()
        .any(|c| c.mttr_minutes == u32::MAX && c.threshold_minutes == u32::MAX));
}

#[test]
fn ratio_cases_match_the_documented_pairs() {
    let cases = ratio_cases();

    assert_eq!(cases.len(), DOCUMENTED_RATIOS.len());
    for (case, documented) in cases.iter().zip(DOCUMENTED_RATIOS) {
        assert_eq!(*case, documented);
    }
}

#[test]
fn ratio_expectations_use_truncation_not_rounding() {
    for (numerator, denominator, expected) in ratio_cases() {
        if denominator == 0 {
            continue;
        }

        let exact = numerator as u64 * 10_000 / denominator as u64;
        assert_eq!(u64::from(expected), exact, "{numerator}/{denominator}");
    }
}

#[test]
fn ratio_results_never_exceed_one_hundred_percent() {
    for (numerator, denominator, expected) in ratio_cases() {
        assert!(
            expected <= 10_000,
            "{numerator}/{denominator} produced {expected} bps"
        );
    }
}

#[test]
fn a_zero_denominator_ratio_is_zero() {
    let zero_denominator = ratio_cases()
        .into_iter()
        .find(|(_, denominator, _)| *denominator == 0)
        .expect("zero-denominator case");

    assert_eq!(zero_denominator, (5, 0, 0));
}

#[test]
fn an_equal_ratio_is_one_hundred_percent() {
    assert!(ratio_cases()
        .into_iter()
        .any(|(numerator, denominator, expected)| numerator == denominator && expected == 10_000));
}

#[test]
fn fixtures_are_stable_across_calls() {
    assert_eq!(mttr_cases(), mttr_cases());
    assert_eq!(ratio_cases(), ratio_cases());
}

#[test]
fn fixtures_are_owned_and_isolated_between_calls() {
    let mut first = mttr_cases();
    first.clear();

    assert_eq!(mttr_cases().len(), DOCUMENTED_CASES.len());
}

#[test]
fn case_names_are_unique_and_non_empty() {
    let cases = mttr_cases();
    let mut seen: Vec<String> = Vec::new();

    for case in &cases {
        assert!(!case.name.is_empty());
        let name = String::from(case.name);
        assert!(!seen.contains(&name), "duplicate case name: {name}");
        seen.push(name);
    }
}
