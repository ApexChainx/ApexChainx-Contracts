//! Deterministic tests for `reporting::metadata` (#996).
//!
//! These tests exercise the isolated SLA operations workspace helper only. They
//! do not touch contract storage, events, or entrypoints, and they use no clock:
//! every timestamp is supplied by the test itself.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::sla_operations::reporting::metadata::{
    build_metadata, MetadataError, ReportMetadata, REPORT_SCHEMA_VERSION,
};

fn sample_metadata() -> ReportMetadata {
    build_metadata(100, 700, 12, 900).expect("valid sample range")
}

#[test]
fn identical_inputs_produce_identical_metadata() {
    let first = build_metadata(100, 700, 12, 900).expect("first build");
    let second = build_metadata(100, 700, 12, 900).expect("second build");
    assert_eq!(first, second);
}

#[test]
fn inverted_range_is_rejected() {
    assert_eq!(
        build_metadata(701, 700, 1, 900),
        Err(MetadataError::InvalidRange)
    );
}

#[test]
fn all_fields_come_from_the_call_inputs() {
    let metadata = sample_metadata();
    assert_eq!(metadata.range_start, 100);
    assert_eq!(metadata.range_end, 700);
    assert_eq!(metadata.record_count, 12);
    assert_eq!(metadata.generated_at, 900);
    assert_eq!(metadata.schema_version, REPORT_SCHEMA_VERSION);
}

#[test]
fn schema_version_is_stable_for_every_build() {
    let first = build_metadata(0, 1, 0, 0).expect("valid range");
    let second = build_metadata(u64::MAX, u64::MAX, u32::MAX, u64::MAX).expect("single instant");
    assert_eq!(first.schema_version, REPORT_SCHEMA_VERSION);
    assert_eq!(second.schema_version, REPORT_SCHEMA_VERSION);
    assert_eq!(REPORT_SCHEMA_VERSION, 1);
}

#[test]
fn pairs_have_a_fixed_key_order() {
    let keys: Vec<String> = sample_metadata()
        .to_pairs()
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    assert_eq!(
        keys,
        alloc::vec![
            "schema_version",
            "range_start",
            "range_end",
            "record_count",
            "generated_at"
        ]
    );
}

#[test]
fn pairs_contain_the_exact_input_values_as_decimal_strings() {
    let values: Vec<String> = sample_metadata()
        .to_pairs()
        .into_iter()
        .map(|(_, value)| value)
        .collect();
    assert_eq!(
        values,
        alloc::vec![
            REPORT_SCHEMA_VERSION.to_string(),
            "100",
            "700",
            "12",
            "900"
        ]
    );
}

#[test]
fn pairs_are_stable_across_repeated_calls() {
    let metadata = sample_metadata();
    assert_eq!(metadata.to_pairs(), metadata.to_pairs());
}

#[test]
fn duration_is_the_difference_between_range_bounds() {
    assert_eq!(sample_metadata().duration_seconds(), 600);
    assert_eq!(
        build_metadata(0, u64::MAX, 1, 0)
            .expect("valid range")
            .duration_seconds(),
        u64::MAX
    );
}

#[test]
fn single_instant_ranges_have_zero_duration() {
    for instant in [0_u64, 1_u64, 1_700_000_000_u64, u64::MAX] {
        let metadata = build_metadata(instant, instant, 1, instant).expect("single instant");
        assert_eq!(metadata.duration_seconds(), 0);
    }
}

#[test]
fn emptiness_depends_only_on_record_count() {
    assert!(build_metadata(0, 0, 0, 0).expect("valid range").is_empty());
    assert!(!build_metadata(0, 0, 1, 0).expect("valid range").is_empty());
    assert!(!sample_metadata().is_empty());
}
