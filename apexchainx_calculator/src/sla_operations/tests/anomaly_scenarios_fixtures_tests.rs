//! Tests for `fixtures::anomaly_scenarios`.
//!
//! Each fixture is expected to exercise exactly one documented anomaly pattern.
//! Assertions are literal so an accidental overlap between predicates (for
//! example, a "complete violation" fixture that also trips the severity
//! concentration rule) fails loudly instead of silently changing the fixture's
//! meaning.

use crate::sla_operations::fixtures::anomaly_scenarios::{scenarios, AnomalyPattern, AnomalyScenario};
use alloc::{string::String, vec, vec::Vec};

type DocumentedScenario = (
    &'static str,
    AnomalyPattern,
    u32,
    u32,
    u32,
    u32,
    u32,
    u64,
    u64,
    bool,
);

const DOCUMENTED_SCENARIOS: [DocumentedScenario; 7] = [
    (
        "baseline",
        AnomalyPattern::Baseline,
        10,
        1,
        2,
        12,
        15,
        100,
        120,
        false,
    ),
    (
        "no_traffic",
        AnomalyPattern::NoTraffic,
        0,
        0,
        0,
        0,
        15,
        5,
        5,
        true,
    ),
    (
        "complete_violation",
        AnomalyPattern::CompleteViolation,
        6,
        6,
        1,
        90,
        30,
        6,
        6,
        true,
    ),
    (
        "extreme_mttr",
        AnomalyPattern::ExtremeMttr,
        4,
        2,
        1,
        u32::MAX,
        120,
        4,
        4,
        true,
    ),
    (
        "counter_regression",
        AnomalyPattern::CounterRegression,
        8,
        2,
        2,
        20,
        30,
        100,
        99,
        true,
    ),
    (
        "severity_concentration",
        AnomalyPattern::SeverityConcentration,
        5,
        1,
        4,
        10,
        15,
        5,
        5,
        true,
    ),
    (
        "zero_threshold",
        AnomalyPattern::ZeroThreshold,
        3,
        1,
        1,
        5,
        0,
        3,
        3,
        true,
    ),
];

fn names() -> Vec<&'static str> {
    scenarios().iter().map(|s| s.name).collect()
}

fn find_scenario(name: &str) -> AnomalyScenario {
    scenarios()
        .into_iter()
        .find(|s| s.name == name)
        .expect("documented anomaly scenario")
}

fn pattern_matches(scenario: &AnomalyScenario, pattern: AnomalyPattern) -> bool {
    match pattern {
        AnomalyPattern::Baseline => false,
        AnomalyPattern::NoTraffic => scenario.has_no_traffic(),
        AnomalyPattern::CompleteViolation => scenario.has_complete_violation(),
        AnomalyPattern::ExtremeMttr => scenario.has_extreme_mttr(),
        AnomalyPattern::CounterRegression => scenario.has_counter_regression(),
        AnomalyPattern::SeverityConcentration => scenario.has_severity_concentration(),
        AnomalyPattern::ZeroThreshold => scenario.has_zero_threshold(),
    }
}

#[test]
fn scenarios_have_the_documented_names_in_order() {
    assert_eq!(
        names(),
        vec![
            "baseline",
            "no_traffic",
            "complete_violation",
            "extreme_mttr",
            "counter_regression",
            "severity_concentration",
            "zero_threshold",
        ]
    );
}

#[test]
fn scenarios_carry_the_documented_values() {
    let all = scenarios();

    assert_eq!(all.len(), DOCUMENTED_SCENARIOS.len());
    for (
        scenario,
        (name, pattern, total, violations, critical, mttr, threshold, previous, current, anomalous),
    ) in all.iter().zip(DOCUMENTED_SCENARIOS)
    {
        assert_eq!(scenario.name, name);
        assert_eq!(scenario.pattern, pattern, "{name}");
        assert_eq!(scenario.total, total, "{name}");
        assert_eq!(scenario.violations, violations, "{name}");
        assert_eq!(scenario.critical, critical, "{name}");
        assert_eq!(scenario.mttr_minutes, mttr, "{name}");
        assert_eq!(scenario.threshold_minutes, threshold, "{name}");
        assert_eq!(scenario.previous_counter, previous, "{name}");
        assert_eq!(scenario.current_counter, current, "{name}");
        assert_eq!(scenario.expect_anomalous, anomalous, "{name}");
    }
}

#[test]
fn every_scenario_flags_exactly_its_documented_pattern() {
    for scenario in scenarios() {
        match scenario.pattern {
            AnomalyPattern::Baseline => {
                assert_eq!(scenario.detected_pattern_count(), 0, "{}", scenario.name);
                assert!(!scenario.is_anomalous(), "{}", scenario.name);
            }
            pattern => {
                assert!(
                    pattern_matches(&scenario, pattern),
                    "{} does not trip its pattern",
                    scenario.name
                );
                assert_eq!(
                    scenario.detected_pattern_count(),
                    1,
                    "{} trips more than one pattern",
                    scenario.name
                );
                assert!(scenario.is_anomalous(), "{}", scenario.name);
            }
        }
    }
}

#[test]
fn every_documented_pattern_is_covered_once() {
    let all = scenarios();

    for pattern in [
        AnomalyPattern::Baseline,
        AnomalyPattern::NoTraffic,
        AnomalyPattern::CompleteViolation,
        AnomalyPattern::ExtremeMttr,
        AnomalyPattern::CounterRegression,
        AnomalyPattern::SeverityConcentration,
        AnomalyPattern::ZeroThreshold,
    ] {
        assert_eq!(
            all.iter().filter(|s| s.pattern == pattern).count(),
            1,
            "{}",
            pattern.label()
        );
    }
}

#[test]
fn the_baseline_scenario_is_not_anomalous() {
    let baseline = find_scenario("baseline");

    assert!(!baseline.has_no_traffic());
    assert!(!baseline.has_complete_violation());
    assert!(!baseline.has_extreme_mttr());
    assert!(!baseline.has_counter_regression());
    assert!(!baseline.has_severity_concentration());
    assert!(!baseline.has_zero_threshold());
    assert!(!baseline.is_anomalous());
    assert_eq!(baseline.detected_pattern_count(), 0);
}

#[test]
fn the_no_traffic_scenario_has_no_calculations() {
    let scenario = find_scenario("no_traffic");

    assert_eq!(scenario.total, 0);
    assert!(scenario.has_no_traffic());
    assert_eq!(scenario.met(), 0);
    assert!(!scenario.has_complete_violation());
}

#[test]
fn the_complete_violation_scenario_has_no_met_calculations() {
    let scenario = find_scenario("complete_violation");

    assert!(scenario.total > 0);
    assert_eq!(scenario.violations, scenario.total);
    assert_eq!(scenario.met(), 0);
    assert!(scenario.has_complete_violation());
}

#[test]
fn the_extreme_mttr_scenario_uses_the_maximum_value() {
    let scenario = find_scenario("extreme_mttr");

    assert_eq!(scenario.mttr_minutes, u32::MAX);
    assert!(scenario.has_extreme_mttr());
    assert!(scenario.mttr_minutes > scenario.threshold_minutes);
}

#[test]
fn the_counter_regression_scenario_moves_backwards() {
    let scenario = find_scenario("counter_regression");

    assert!(scenario.current_counter < scenario.previous_counter);
    assert!(scenario.has_counter_regression());
}

#[test]
fn the_severity_concentration_scenario_has_a_critical_majority() {
    let scenario = find_scenario("severity_concentration");

    assert!(scenario.critical > scenario.total / 2);
    assert!(scenario.has_severity_concentration());
}

#[test]
fn the_zero_threshold_scenario_has_a_zero_threshold() {
    let scenario = find_scenario("zero_threshold");

    assert_eq!(scenario.threshold_minutes, 0);
    assert!(scenario.has_zero_threshold());
}

#[test]
fn met_saturates_when_violations_exceed_the_total() {
    let mut scenario = find_scenario("baseline");
    scenario.total = 2;
    scenario.violations = 5;

    assert_eq!(scenario.met(), 0);
    assert!(scenario.has_complete_violation());
}

#[test]
fn scenarios_are_stable_and_owned_across_calls() {
    assert_eq!(scenarios(), scenarios());

    let mut first = scenarios();
    first.clear();
    assert_eq!(scenarios().len(), DOCUMENTED_SCENARIOS.len());
}

#[test]
fn scenario_names_are_unique_and_non_empty() {
    let mut seen: Vec<String> = Vec::new();

    for scenario in scenarios() {
        assert!(!scenario.name.is_empty());
        let name = String::from(scenario.name);
        assert!(!seen.contains(&name), "duplicate scenario name: {name}");
        seen.push(name);
    }
}

#[test]
fn pattern_labels_are_stable_and_lowercase() {
    assert_eq!(AnomalyPattern::Baseline.label(), "baseline");
    assert_eq!(AnomalyPattern::NoTraffic.label(), "no_traffic");
    assert_eq!(AnomalyPattern::CompleteViolation.label(), "complete_violation");
    assert_eq!(AnomalyPattern::ExtremeMttr.label(), "extreme_mttr");
    assert_eq!(AnomalyPattern::CounterRegression.label(), "counter_regression");
    assert_eq!(
        AnomalyPattern::SeverityConcentration.label(),
        "severity_concentration"
    );
    assert_eq!(AnomalyPattern::ZeroThreshold.label(), "zero_threshold");
}
