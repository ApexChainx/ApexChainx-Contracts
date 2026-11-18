//! Tests for `reporting::severity_distribution`.

use crate::sla_operations::reporting::severity_distribution::{
    build_distribution, total_count, SeverityCount,
};
use alloc::{string::String, vec::Vec};

fn sc(severity: &str, count: u32) -> SeverityCount {
    SeverityCount {
        severity: String::from(severity),
        count,
    }
}

#[test]
fn empty_input_gives_no_rows() {
    assert!(build_distribution(&[]).is_empty());
    assert_eq!(total_count(&[]), 0);
}

#[test]
fn rows_are_sorted_by_severity_name() {
    let rows = build_distribution(&[sc("low", 1), sc("critical", 1), sc("high", 1)]);
    let names: Vec<&str> = rows.iter().map(|r| r.severity.as_str()).collect();
    assert_eq!(names, alloc::vec!["critical", "high", "low"]);
}

#[test]
fn shares_are_computed_in_basis_points() {
    let rows = build_distribution(&[
        sc("critical", 10),
        sc("high", 20),
        sc("medium", 30),
        sc("low", 40),
    ]);
    let shares: Vec<(&str, u32)> = rows.iter().map(|r| (r.severity.as_str(), r.share_bps)).collect();
    assert_eq!(
        shares,
        alloc::vec![("critical", 1_000), ("high", 2_000), ("low", 4_000), ("medium", 3_000)]
    );
}

#[test]
fn a_single_severity_has_a_full_share() {
    let rows = build_distribution(&[sc("high", 7)]);
    assert_eq!(rows[0].share_bps, 10_000);
}

#[test]
fn shares_truncate_and_never_exceed_the_whole() {
    let rows = build_distribution(&[sc("a", 1), sc("b", 1), sc("c", 1)]);
    assert!(rows.iter().all(|r| r.share_bps == 3_333));
    let sum: u32 = rows.iter().map(|r| r.share_bps).sum();
    assert_eq!(sum, 9_999);
    assert!(sum <= 10_000);
}

#[test]
fn duplicate_severities_are_merged() {
    let rows = build_distribution(&[sc("high", 3), sc("low", 1), sc("high", 4)]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].severity, "high");
    assert_eq!(rows[0].count, 7);
    assert_eq!(rows[0].share_bps, 8_750);
}

#[test]
fn all_zero_counts_give_zero_shares() {
    let rows = build_distribution(&[sc("critical", 0), sc("low", 0)]);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.share_bps == 0));
}

#[test]
fn a_zero_count_row_is_kept_with_a_zero_share() {
    let rows = build_distribution(&[sc("high", 0), sc("low", 5)]);
    assert_eq!(rows[0].severity, "high");
    assert_eq!(rows[0].share_bps, 0);
    assert_eq!(rows[1].share_bps, 10_000);
}

#[test]
fn total_count_matches_the_merged_input() {
    let rows = build_distribution(&[sc("high", 3), sc("low", 1), sc("high", 4)]);
    assert_eq!(total_count(&rows), 8);
}

#[test]
fn merging_saturates_instead_of_overflowing() {
    let rows = build_distribution(&[sc("high", u32::MAX), sc("high", 5)]);
    assert_eq!(rows[0].count, u32::MAX);
}

#[test]
fn output_is_deterministic() {
    let input = [sc("b", 2), sc("a", 3)];
    assert_eq!(build_distribution(&input), build_distribution(&input));
}
