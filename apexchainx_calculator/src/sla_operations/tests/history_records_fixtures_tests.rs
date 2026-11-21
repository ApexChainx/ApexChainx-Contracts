//! Tests for `fixtures::history_records`.
//!
//! The fixtures are pure data, so these tests assert literal field values and
//! the invariants the calculator relies on (status vs. MTTR, amount sign vs.
//! payment type). No contract storage or `Env` is used.

use crate::sla_operations::fixtures::history_records::{
    boundary_record, met_record, mixed_history, violated_record,
};
use crate::sla_operations::reporting::history_export::{to_export_rows, HistoryRecord};
use alloc::{string::String, vec::Vec};

/// Penalty per minute over the threshold, as used by the fixture.
const PENALTY_PER_MINUTE: i128 = 100;

fn ids(records: &[HistoryRecord]) -> Vec<String> {
    records.iter().map(|r| r.outage_id.clone()).collect()
}

#[test]
fn met_record_has_its_documented_fields() {
    let record = met_record();

    assert_eq!(record.outage_id, "hist-met");
    assert_eq!(record.severity, "critical");
    assert_eq!(record.status, "met");
    assert_eq!(record.mttr_minutes, 5);
    assert_eq!(record.threshold_minutes, 15);
    assert_eq!(record.amount, 750);
    assert_eq!(record.payment_type, "rew");
    assert_eq!(record.rating, "top");
}

#[test]
fn violated_record_has_its_documented_fields() {
    let record = violated_record();

    assert_eq!(record.outage_id, "hist-viol");
    assert_eq!(record.severity, "critical");
    assert_eq!(record.status, "viol");
    assert_eq!(record.mttr_minutes, 45);
    assert_eq!(record.threshold_minutes, 15);
    assert_eq!(record.amount, -3_000);
    assert_eq!(record.payment_type, "pen");
    assert_eq!(record.rating, "poor");
}

#[test]
fn boundary_record_meets_its_threshold_exactly() {
    let record = boundary_record();

    assert_eq!(record.mttr_minutes, record.threshold_minutes);
    assert_eq!(record.status, "met");
    assert!(record.amount > 0);
    assert_eq!(record.payment_type, "rew");
}

#[test]
fn violation_penalty_matches_minutes_over_threshold() {
    let record = violated_record();
    let over = i128::from(record.mttr_minutes - record.threshold_minutes);

    assert_eq!(record.amount, -over * PENALTY_PER_MINUTE);
}

#[test]
fn history_fixtures_are_stable_across_calls() {
    assert_eq!(met_record(), met_record());
    assert_eq!(violated_record(), violated_record());
    assert_eq!(boundary_record(), boundary_record());
    assert_eq!(mixed_history(), mixed_history());
}

#[test]
fn mixed_history_keeps_its_documented_order() {
    let records = mixed_history();

    assert_eq!(records.len(), 3);
    assert_eq!(
        records,
        alloc::vec![met_record(), violated_record(), boundary_record()]
    );
}

#[test]
fn mixed_history_ids_are_unique() {
    let records = mixed_history();
    let mut seen: Vec<String> = Vec::new();

    for outage_id in ids(&records) {
        assert!(!seen.contains(&outage_id), "duplicate outage id: {outage_id}");
        seen.push(outage_id);
    }
}

#[test]
fn every_fixture_keeps_status_consistent_with_the_mttr_rule() {
    for record in mixed_history() {
        let over_threshold = record.mttr_minutes > record.threshold_minutes;
        assert_eq!(record.status == "viol", over_threshold, "{}", record.outage_id);
        assert_eq!(record.status == "met", !over_threshold, "{}", record.outage_id);
    }
}

#[test]
fn every_fixture_keeps_amount_sign_consistent_with_payment_type() {
    for record in mixed_history() {
        match record.payment_type.as_str() {
            "rew" => assert!(record.amount > 0, "{}", record.outage_id),
            "pen" => assert!(record.amount < 0, "{}", record.outage_id),
            other => panic!("unexpected payment type: {other}"),
        }
    }
}

#[test]
fn mixed_history_covers_both_outcomes() {
    let records = mixed_history();

    assert!(records.iter().any(|r| r.status == "met"));
    assert!(records.iter().any(|r| r.status == "viol"));
}

#[test]
fn fixtures_are_owned_and_isolated_between_calls() {
    let mut first = mixed_history();
    first.clear();

    assert_eq!(mixed_history().len(), 3);
}

#[test]
fn mixed_history_exports_in_order_with_absolute_indexes() {
    let records = mixed_history();
    let rows = to_export_rows(&records);

    assert_eq!(rows.len(), records.len());
    for (position, row) in rows.iter().enumerate() {
        assert_eq!(row.index, position as u32);
        assert_eq!(row.record, records[position]);
    }
}
