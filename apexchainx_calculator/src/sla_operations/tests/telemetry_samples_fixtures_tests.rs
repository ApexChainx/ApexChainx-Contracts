//! Tests for `fixtures::telemetry_samples`.
//!
//! The fixtures are pure counter data. Tests assert their documented shape and
//! that the telemetry snapshot formatter consumes them consistently (sorting,
//! clamping and rate math). No contract storage or `Env` is involved.

use crate::sla_operations::fixtures::telemetry_samples::{
    counters, empty_snapshot, inconsistent_snapshot, quiet_snapshot, typical_snapshot,
};
use crate::sla_operations::reporting::telemetry_snapshot::{
    format_bps, format_snapshot, render_lines, SeverityCounters,
};
use alloc::{vec, vec::Vec};

fn find<'a>(snapshot: &'a [SeverityCounters], severity: &str) -> Option<&'a SeverityCounters> {
    snapshot.iter().find(|c| c.severity == severity)
}

#[test]
fn counters_builder_keeps_every_field_verbatim() {
    let row = counters("high", 20, 2);

    assert_eq!(row.severity, "high");
    assert_eq!(row.calculations, 20);
    assert_eq!(row.violations, 2);
}

#[test]
fn counters_builder_accepts_zero_and_maximum_values() {
    let zero = counters("low", 0, 0);
    assert_eq!((zero.calculations, zero.violations), (0, 0));

    let largest = counters("critical", u32::MAX, u32::MAX);
    assert_eq!((largest.calculations, largest.violations), (u32::MAX, u32::MAX));
}

#[test]
fn typical_snapshot_has_its_documented_shape() {
    let snapshot = typical_snapshot();

    assert_eq!(snapshot.len(), 4);
    assert_eq!(
        snapshot,
        alloc::vec![
            counters("low", 40, 1),
            counters("critical", 10, 3),
            counters("high", 20, 2),
            counters("medium", 30, 0),
        ]
    );
}

#[test]
fn typical_snapshot_values_are_looked_up_by_severity() {
    let snapshot = typical_snapshot();

    let critical = find(&snapshot, "critical").expect("critical row");
    assert_eq!((critical.calculations, critical.violations), (10, 3));

    let medium = find(&snapshot, "medium").expect("medium row");
    assert_eq!((medium.calculations, medium.violations), (30, 0));
}

#[test]
fn quiet_snapshot_has_a_quiet_and_an_idle_severity() {
    let snapshot = quiet_snapshot();

    assert!(snapshot.iter().any(|c| c.calculations > 0 && c.violations == 0));
    assert!(snapshot.iter().any(|c| c.calculations == 0 && c.violations == 0));
}

#[test]
fn inconsistent_snapshot_is_malformed_by_design() {
    let snapshot = inconsistent_snapshot();

    assert!(!snapshot.is_empty());
    assert!(snapshot.iter().all(|c| c.violations > c.calculations));
}

#[test]
fn snapshots_are_stable_across_calls() {
    assert_eq!(typical_snapshot(), typical_snapshot());
    assert_eq!(quiet_snapshot(), quiet_snapshot());
    assert_eq!(inconsistent_snapshot(), inconsistent_snapshot());
    assert_eq!(empty_snapshot(), empty_snapshot());
}

#[test]
fn empty_snapshot_has_no_rows() {
    assert!(empty_snapshot().is_empty());
}

#[test]
fn snapshots_are_owned_and_isolated_between_calls() {
    let mut first = typical_snapshot();
    first.clear();

    assert_eq!(typical_snapshot().len(), 4);
}

#[test]
fn typical_snapshot_formats_with_sorted_severities() {
    let report = format_snapshot(&typical_snapshot());
    let severities: Vec<&str> = report.rows.iter().map(|r| r.severity.as_str()).collect();

    assert_eq!(severities, vec!["critical", "high", "low", "medium"]);
}

#[test]
fn typical_snapshot_totals_and_overall_rate_are_consistent() {
    let report = format_snapshot(&typical_snapshot());

    assert_eq!(report.total_calculations, 100);
    assert_eq!(report.total_violations, 6);
    assert_eq!(report.overall_rate_bps, 600);
}

#[test]
fn quiet_snapshot_has_no_violation_rate() {
    let report = format_snapshot(&quiet_snapshot());
    let idle = report
        .rows
        .iter()
        .find(|r| r.calculations == 0)
        .expect("idle row");

    assert_eq!(idle.violation_rate_bps, 0);
    assert_eq!(report.total_violations, 0);
    assert_eq!(report.overall_rate_bps, 0);
}

#[test]
fn inconsistent_snapshot_is_clamped_by_the_formatter() {
    let report = format_snapshot(&inconsistent_snapshot());

    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].violations, 2);
    assert_eq!(report.rows[0].violation_rate_bps, 10_000);
    assert_eq!(report.total_violations, 2);
}

#[test]
fn empty_snapshot_formats_to_an_empty_report() {
    let report = format_snapshot(&empty_snapshot());

    assert!(report.rows.is_empty());
    assert_eq!(
        (
            report.total_calculations,
            report.total_violations,
            report.overall_rate_bps
        ),
        (0, 0, 0)
    );
    assert!(render_lines(&report).is_empty());
}

#[test]
fn typical_snapshot_renders_human_readable_lines() {
    let lines = render_lines(&format_snapshot(&typical_snapshot()));

    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "critical: 10 calculations, 3 violations (30.00%)");
    assert_eq!(lines[3], "medium: 30 calculations, 0 violations (0.00%)");
}

#[test]
fn format_bps_covers_zero_full_and_fractional_rates() {
    assert_eq!(format_bps(0), "0.00%");
    assert_eq!(format_bps(50), "0.50%");
    assert_eq!(format_bps(2_050), "20.50%");
    assert_eq!(format_bps(10_000), "100.00%");
}

#[test]
fn fixtures_use_only_canonical_severities() {
    for snapshot in [typical_snapshot(), quiet_snapshot(), inconsistent_snapshot()] {
        for row in snapshot {
            assert!(
                ["critical", "high", "medium", "low"].contains(&row.severity.as_str()),
                "unexpected severity: {}",
                row.severity
            );
        }
    }
}
