//! Tests for `validation::aggregation_inputs`.

use crate::sla_operations::validation::aggregation_inputs::{
    validate_counters, validate_records, AggregationRecord, InputError, MAX_RECORDS,
};
use alloc::{format, string::String, vec::Vec};

fn rec<'a>(id: &'a str, severity: &'a str, mttr: u32, threshold: u32) -> AggregationRecord<'a> {
    AggregationRecord {
        outage_id: id,
        severity,
        mttr_minutes: mttr,
        threshold_minutes: threshold,
    }
}

#[test]
fn an_empty_batch_is_rejected() {
    assert_eq!(validate_records(&[]), Err(InputError::Empty));
}

#[test]
fn a_well_formed_batch_is_accepted() {
    let batch = [rec("a", "high", 10, 30), rec("b", "low", 500, 120)];
    assert_eq!(validate_records(&batch), Ok(()));
}

#[test]
fn a_violating_record_is_still_valid_input() {
    // mttr above threshold is a *result*, not an input error.
    assert_eq!(validate_records(&[rec("a", "high", 999, 30)]), Ok(()));
}

#[test]
fn empty_outage_id_is_rejected() {
    assert_eq!(
        validate_records(&[rec("", "high", 1, 1)]),
        Err(InputError::EmptyOutageId)
    );
}

#[test]
fn empty_severity_is_rejected() {
    assert_eq!(
        validate_records(&[rec("a", "", 1, 1)]),
        Err(InputError::EmptySeverity)
    );
}

#[test]
fn zero_threshold_is_rejected() {
    assert_eq!(
        validate_records(&[rec("a", "high", 1, 0)]),
        Err(InputError::ZeroThreshold)
    );
}

#[test]
fn duplicate_outage_ids_are_rejected() {
    let batch = [rec("a", "high", 1, 1), rec("b", "high", 1, 1), rec("a", "low", 1, 1)];
    assert_eq!(validate_records(&batch), Err(InputError::DuplicateOutageId));
}

#[test]
fn the_same_id_in_different_case_is_not_a_duplicate() {
    let batch = [rec("A", "high", 1, 1), rec("a", "high", 1, 1)];
    assert_eq!(validate_records(&batch), Ok(()));
}

#[test]
fn field_errors_are_reported_before_duplicates() {
    let batch = [rec("a", "high", 1, 1), rec("a", "", 1, 1)];
    assert_eq!(validate_records(&batch), Err(InputError::EmptySeverity));
}

#[test]
fn the_first_offending_record_decides_the_error() {
    let batch = [rec("a", "", 1, 1), rec("", "high", 1, 1)];
    assert_eq!(validate_records(&batch), Err(InputError::EmptySeverity));
}

#[test]
fn batch_size_limit_is_inclusive() {
    let ids: Vec<String> = (0..MAX_RECORDS + 1).map(|i| format!("id-{i}")).collect();
    let at_limit: Vec<AggregationRecord<'_>> =
        ids[..MAX_RECORDS].iter().map(|id| rec(id, "high", 1, 1)).collect();
    assert_eq!(validate_records(&at_limit), Ok(()));

    let over: Vec<AggregationRecord<'_>> = ids.iter().map(|id| rec(id, "high", 1, 1)).collect();
    assert_eq!(validate_records(&over), Err(InputError::TooManyRecords));
}

#[test]
fn counters_accept_violations_up_to_the_total() {
    assert_eq!(validate_counters(0, 0), Ok(()));
    assert_eq!(validate_counters(5, 0), Ok(()));
    assert_eq!(validate_counters(5, 5), Ok(()));
    assert_eq!(validate_counters(u32::MAX, u32::MAX), Ok(()));
}

#[test]
fn counters_reject_violations_above_the_total() {
    assert_eq!(validate_counters(5, 6), Err(InputError::ViolationsExceedTotal));
    assert_eq!(validate_counters(0, 1), Err(InputError::ViolationsExceedTotal));
    assert_eq!(
        validate_counters(0, u32::MAX),
        Err(InputError::ViolationsExceedTotal)
    );
}
