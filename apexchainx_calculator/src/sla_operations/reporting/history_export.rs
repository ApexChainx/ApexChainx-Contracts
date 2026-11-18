//! History export helpers.
//!
//! Transforms history-like records into flat, ordered rows that off-chain
//! consumers can serialise (CSV, JSON, ...). Rows keep the input order and carry
//! an absolute, zero-based `index`, so a page of history exports with the same
//! indexes it would have in a full export.
//!
//! Inputs are plain data: this module never reads contract state.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

/// Column names, in the order produced by [`row_fields`].
pub const EXPORT_COLUMNS: [&str; 9] = [
    "index",
    "outage_id",
    "severity",
    "status",
    "mttr_minutes",
    "threshold_minutes",
    "amount",
    "payment_type",
    "rating",
];

/// A history entry as supplied by the caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRecord {
    pub outage_id: String,
    pub severity: String,
    /// `"met"` or `"viol"`.
    pub status: String,
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
    /// Negative for penalties, positive for rewards.
    pub amount: i128,
    /// `"rew"` or `"pen"`.
    pub payment_type: String,
    pub rating: String,
}

/// One exportable row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportRow {
    /// Absolute zero-based position of the record in the full history.
    pub index: u32,
    pub record: HistoryRecord,
}

/// Convert a full history into rows, indexed from 0 in input order.
pub fn to_export_rows(records: &[HistoryRecord]) -> Vec<ExportRow> {
    to_export_rows_page(records, 0, u32::MAX)
}

/// Convert at most `limit` records starting at `offset` into rows.
///
/// Row indexes are absolute (`offset + position`). An `offset` past the end or
/// a `limit` of zero yields an empty vector.
pub fn to_export_rows_page(records: &[HistoryRecord], offset: u32, limit: u32) -> Vec<ExportRow> {
    let start = offset as usize;
    if start >= records.len() || limit == 0 {
        return Vec::new();
    }
    records[start..]
        .iter()
        .take(limit as usize)
        .enumerate()
        .map(|(i, record)| ExportRow {
            index: offset.saturating_add(i as u32),
            record: record.clone(),
        })
        .collect()
}

/// Render a row as strings in [`EXPORT_COLUMNS`] order.
pub fn row_fields(row: &ExportRow) -> Vec<String> {
    let r = &row.record;
    alloc::vec![
        row.index.to_string(),
        r.outage_id.clone(),
        r.severity.clone(),
        r.status.clone(),
        r.mttr_minutes.to_string(),
        r.threshold_minutes.to_string(),
        r.amount.to_string(),
        r.payment_type.clone(),
        r.rating.clone(),
    ]
}

/// One-line, human-readable summary of a row, e.g. `#0 out-1 critical viol 20/15`.
pub fn describe_row(row: &ExportRow) -> String {
    let r = &row.record;
    format!(
        "#{} {} {} {} {}/{}",
        row.index, r.outage_id, r.severity, r.status, r.mttr_minutes, r.threshold_minutes
    )
}

#[cfg(test)]
mod export_smoke_tests {
    use super::*;

    fn rec(id: &str) -> HistoryRecord {
        HistoryRecord {
            outage_id: String::from(id),
            severity: String::from("high"),
            status: String::from("met"),
            mttr_minutes: 1,
            threshold_minutes: 2,
            amount: 5,
            payment_type: String::from("rew"),
            rating: String::from("good"),
        }
    }

    #[test]
    fn pages_keep_absolute_indexes() {
        let all = alloc::vec![rec("a"), rec("b"), rec("c")];
        let page = to_export_rows_page(&all, 1, 5);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].index, 1);
        assert!(to_export_rows_page(&all, 3, 5).is_empty());
    }

    #[test]
    fn fields_match_the_column_count() {
        let rows = to_export_rows(&[rec("a")]);
        assert_eq!(row_fields(&rows[0]).len(), EXPORT_COLUMNS.len());
    }
}
