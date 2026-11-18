//! Tests for `reporting::telemetry_snapshot`.

use crate::sla_operations::reporting::telemetry_snapshot::{
    format_bps, format_snapshot, render_lines, SeverityCounters,
};
use alloc::string::String;

fn c(severity: &str, calculations: u32, violations: u32) -> SeverityCounters {
    SeverityCounters {
        severity: String::from(severity),
        calculations,
        violations,
    }
}

#[test]
fn format_bps_always_shows_two_decimals() {
    assert_eq!(format_bps(0), "0.00%");
    assert_eq!(format_bps(5), "0.05%");
    assert_eq!(format_bps(2_050), "20.50%");
    assert_eq!(format_bps(10_000), "100.00%");
}

#[test]
fn empty_snapshot_has_no_rows_and_zero_rate() {
    let report = format_snapshot(&[]);
    assert!(report.rows.is_empty());
    assert_eq!(report.total_calculations, 0);
    assert_eq!(report.overall_rate_bps, 0);
    assert!(render_lines(&report).is_empty());
}

#[test]
fn rows_are_sorted_by_severity_name() {
    let report = format_snapshot(&[c("low", 1, 0), c("critical", 1, 0), c("high", 1, 0)]);
    let names: alloc::vec::Vec<&str> = report.rows.iter().map(|r| r.severity.as_str()).collect();
    assert_eq!(names, alloc::vec!["critical", "high", "low"]);
}

#[test]
fn per_row_rate_truncates_toward_zero() {
    let report = format_snapshot(&[c("high", 3, 1)]);
    // 1 / 3 = 33.33...% -> 3333 bps
    assert_eq!(report.rows[0].violation_rate_bps, 3_333);
}

#[test]
fn zero_calculations_gives_a_zero_rate_not_a_division_error() {
    let report = format_snapshot(&[c("high", 0, 0)]);
    assert_eq!(report.rows[0].violation_rate_bps, 0);
    assert_eq!(report.overall_rate_bps, 0);
}

#[test]
fn violations_above_calculations_are_clamped() {
    let report = format_snapshot(&[c("critical", 2, 5)]);
    assert_eq!(report.rows[0].violations, 2);
    assert_eq!(report.rows[0].violation_rate_bps, 10_000);
    assert_eq!(report.total_violations, 2);
}

#[test]
fn totals_and_overall_rate_span_all_rows() {
    let report = format_snapshot(&[c("low", 40, 1), c("critical", 10, 3), c("high", 20, 2)]);
    assert_eq!(report.total_calculations, 70);
    assert_eq!(report.total_violations, 6);
    // 6 / 70 = 8.57...% -> 857 bps
    assert_eq!(report.overall_rate_bps, 857);
}

#[test]
fn rendered_lines_follow_the_documented_format() {
    let report = format_snapshot(&[c("high", 10, 2)]);
    assert_eq!(
        render_lines(&report),
        alloc::vec![String::from("high: 10 calculations, 2 violations (20.00%)")]
    );
}

#[test]
fn same_input_gives_the_same_report() {
    let input = [c("a", 3, 1), c("b", 9, 9)];
    assert_eq!(format_snapshot(&input), format_snapshot(&input));
}
