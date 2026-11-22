//! Tests for `fixtures::sla_inputs`.
//!
//! The fixtures are pure builders, so every test compares against literal
//! expected values and never touches contract storage or the test `Env`.

use crate::sla_operations::fixtures::sla_inputs::{
    input, inputs_for, met_input, sample_inputs, violated_input, SlaInput,
};
use alloc::{string::String, vec, vec::Vec};

/// Critical severity threshold used by the SLA calculator, in minutes.
const CRITICAL_THRESHOLD_MINUTES: u32 = 15;

fn ids(inputs: &[SlaInput]) -> Vec<String> {
    inputs.iter().map(|i| i.outage_id.clone()).collect()
}

fn mttrs(inputs: &[SlaInput]) -> Vec<u32> {
    inputs.iter().map(|i| i.mttr_minutes).collect()
}

fn count_for(inputs: &[SlaInput], severity: &str) -> usize {
    inputs.iter().filter(|i| i.severity == severity).count()
}

#[test]
fn input_builder_keeps_every_field_verbatim() {
    let built = input("outage-42", "high", 30);

    assert_eq!(built.outage_id, "outage-42");
    assert_eq!(built.severity, "high");
    assert_eq!(built.mttr_minutes, 30);
}

#[test]
fn input_builder_accepts_empty_strings_and_extreme_mttrs() {
    let empty = input("", "", 0);
    assert_eq!(empty.outage_id, "");
    assert_eq!(empty.severity, "");
    assert_eq!(empty.mttr_minutes, 0);

    let largest = input("outage-max", "low", u32::MAX);
    assert_eq!(largest.mttr_minutes, u32::MAX);
}

#[test]
fn named_fixtures_are_stable_across_calls() {
    assert_eq!(met_input(), met_input());
    assert_eq!(violated_input(), violated_input());
}

#[test]
fn met_and_violated_fixtures_sit_on_opposite_sides_of_the_threshold() {
    let met = met_input();
    let violated = violated_input();

    assert_eq!(met.severity, "critical");
    assert_eq!(violated.severity, "critical");
    assert!(met.mttr_minutes <= CRITICAL_THRESHOLD_MINUTES);
    assert!(violated.mttr_minutes > CRITICAL_THRESHOLD_MINUTES);
    assert_ne!(met.outage_id, violated.outage_id);
}

#[test]
fn inputs_for_derives_ids_from_severity_and_position() {
    let batch = inputs_for("high", &[10, 20, 30]);

    assert_eq!(
        ids(&batch),
        vec![
            String::from("high-0"),
            String::from("high-1"),
            String::from("high-2"),
        ]
    );
    assert!(batch.iter().all(|i| i.severity == "high"));
    assert_eq!(mttrs(&batch), vec![10, 20, 30]);
}

#[test]
fn inputs_for_preserves_boundary_mttr_values() {
    let batch = inputs_for("low", &[0, u32::MAX]);

    assert_eq!(mttrs(&batch), vec![0, u32::MAX]);
    assert_eq!(ids(&batch), vec![String::from("low-0"), String::from("low-1")]);
}

#[test]
fn inputs_for_empty_slice_yields_no_inputs() {
    let batch = inputs_for("critical", &[]);

    assert!(batch.is_empty());
}

#[test]
fn sample_inputs_covers_every_canonical_severity() {
    let inputs = sample_inputs();

    assert_eq!(inputs.len(), 8);
    assert_eq!(count_for(&inputs, "critical"), 3);
    assert_eq!(count_for(&inputs, "high"), 2);
    assert_eq!(count_for(&inputs, "medium"), 2);
    assert_eq!(count_for(&inputs, "low"), 1);
}

#[test]
fn sample_inputs_has_unique_ids() {
    let inputs = sample_inputs();
    let mut seen: Vec<String> = Vec::new();

    for outage_id in ids(&inputs) {
        assert!(!seen.contains(&outage_id), "duplicate outage id: {outage_id}");
        seen.push(outage_id);
    }
}

#[test]
fn sample_inputs_is_deterministic() {
    assert_eq!(sample_inputs(), sample_inputs());
}

#[test]
fn fixture_vectors_are_owned_and_isolated_between_calls() {
    let mut first = sample_inputs();
    first.push(input("extra", "critical", 1));
    first[0].mttr_minutes = 999;

    let second = sample_inputs();
    assert_eq!(second.len(), 8);
    assert_eq!(second[0].mttr_minutes, 5);
    assert!(second.iter().all(|i| i.outage_id != "extra"));
}
