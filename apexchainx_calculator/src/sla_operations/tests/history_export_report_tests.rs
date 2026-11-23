//! Tests for `reporting::history_export`.

use crate::sla_operations::reporting::history_export::{
    describe_row, row_fields, to_export_rows, to_export_rows_page, HistoryRecord, EXPORT_COLUMNS,
};
use alloc::{string::String, vec::Vec};

fn rec(id: &str, status: &str, mttr: u32, amount: i128) -> HistoryRecord {
    HistoryRecord {
        outage_id: String::from(id),
        severity: String::from("critical"),
        status: String::from(status),
        mttr_minutes: mttr,
        threshold_minutes: 15,
        amount,
        payment_type: String::from(if amount < 0 { "pen" } else { "rew" }),
        rating: String::from("good"),
    }
}

fn history(n: u32) -> Vec<HistoryRecord> {
    (0..n)
        .map(|i| rec(&alloc::format!("out-{i}"), "met", i, 100))
        .collect()
}

#[test]
fn empty_history_exports_no_rows() {
    assert!(to_export_rows(&[]).is_empty());
}

#[test]
fn rows_keep_input_order_and_zero_based_indexes() {
    let rows = to_export_rows(&history(3));
    let indexes: Vec<u32> = rows.iter().map(|r| r.index).collect();
    assert_eq!(indexes, alloc::vec![0, 1, 2]);
    assert_eq!(rows[2].record.outage_id, "out-2");
}

#[test]
fn page_uses_absolute_indexes() {
    let rows = to_export_rows_page(&history(10), 4, 3);
    let indexes: Vec<u32> = rows.iter().map(|r| r.index).collect();
    assert_eq!(indexes, alloc::vec![4, 5, 6]);
    assert_eq!(rows[0].record.outage_id, "out-4");
}

#[test]
fn page_is_truncated_at_the_end_of_history() {
    let rows = to_export_rows_page(&history(5), 3, 10);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].index, 4);
}

#[test]
fn offset_past_the_end_and_zero_limit_yield_nothing() {
    assert!(to_export_rows_page(&history(5), 5, 3).is_empty());
    assert!(to_export_rows_page(&history(5), 99, 3).is_empty());
    assert!(to_export_rows_page(&history(5), 0, 0).is_empty());
}

#[test]
fn pages_concatenate_to_the_full_export() {
    let h = history(7);
    let mut joined = to_export_rows_page(&h, 0, 3);
    joined.extend(to_export_rows_page(&h, 3, 3));
    joined.extend(to_export_rows_page(&h, 6, 3));
    assert_eq!(joined, to_export_rows(&h));
}

#[test]
fn row_fields_match_the_column_order_and_arity() {
    let rows = to_export_rows(&[rec("out-x", "viol", 45, -3_000)]);
    let fields = row_fields(&rows[0]);
    assert_eq!(fields.len(), EXPORT_COLUMNS.len());
    assert_eq!(
        fields,
        alloc::vec![
            "0", "out-x", "critical", "viol", "45", "15", "-3000", "pen", "good"
        ]
    );
    assert_eq!(EXPORT_COLUMNS[0], "index");
    assert_eq!(EXPORT_COLUMNS[6], "amount");
}

#[test]
fn describe_row_is_a_stable_single_line() {
    let rows = to_export_rows(&[rec("out-x", "viol", 45, -3_000)]);
    assert_eq!(describe_row(&rows[0]), "#0 out-x critical viol 45/15");
}

#[test]
fn export_is_deterministic() {
    let h = history(4);
    assert_eq!(to_export_rows(&h), to_export_rows(&h));
}
