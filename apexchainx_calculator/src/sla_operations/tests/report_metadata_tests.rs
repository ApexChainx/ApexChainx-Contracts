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
