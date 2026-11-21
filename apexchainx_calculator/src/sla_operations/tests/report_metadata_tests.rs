//! Deterministic tests for `reporting::metadata` (#996).
//!
//! These tests exercise the isolated SLA operations workspace helper only. They
//! do not touch contract storage, events, or entrypoints, and they use no clock:
//! every timestamp is supplied by the test itself.

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
