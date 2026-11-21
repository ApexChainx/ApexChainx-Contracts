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

#[test]
fn metadata_error_is_copyable_and_comparable() {
    let error = MetadataError::InvalidRange;
    let copied = error;
    assert_eq!(error, copied);
    assert_eq!(copied, MetadataError::InvalidRange);
    assert_eq!(alloc::format!("{copied:?}"), "InvalidRange");
}

#[test]
fn generated_at_only_changes_its_own_pair() {
    let earlier = build_metadata(10, 20, 3, 100).expect("valid range");
    let later = build_metadata(10, 20, 3, 200).expect("valid range");

    let earlier_pairs = earlier.to_pairs();
    let later_pairs = later.to_pairs();
    assert_eq!(earlier_pairs.len(), later_pairs.len());

    for (index, (left, right)) in earlier_pairs.iter().zip(later_pairs.iter()).enumerate() {
        assert_eq!(left.0, right.0);
        if index == 4 {
            assert_ne!(left.1, right.1);
        } else {
            assert_eq!(left.1, right.1);
        }
    }
}

#[test]
fn maximum_record_count_is_rendered_without_overflow() {
    let metadata = build_metadata(0, 5, u32::MAX, u64::MAX).expect("valid range");
    let pairs = metadata.to_pairs();
    assert_eq!(pairs[3].1, u32::MAX.to_string());
    assert_eq!(pairs[4].1, u64::MAX.to_string());
    assert!(!metadata.is_empty());
}
