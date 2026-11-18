//! Tests for `reporting::compliance_report`.

use crate::sla_operations::reporting::compliance_report::{
    build_report, worst_first, ComplianceEntry,
};
use alloc::{string::String, vec::Vec};

fn e(id: &str, severity: &str, mttr: u32, threshold: u32) -> ComplianceEntry {
    ComplianceEntry {
        outage_id: String::from(id),
        severity: String::from(severity),
        mttr_minutes: mttr,
        threshold_minutes: threshold,
    }
}

#[test]
fn empty_input_reports_nothing_checked() {
    let r = build_report(&[]);
    assert_eq!((r.checked, r.compliant, r.compliance_bps), (0, 0, 0));
    assert!(r.violations.is_empty());
}

#[test]
fn meeting_the_threshold_exactly_is_compliant() {
    let r = build_report(&[e("a", "high", 30, 30)]);
    assert!(r.violations.is_empty());
    assert_eq!(r.compliant, 1);
    assert_eq!(r.compliance_bps, 10_000);
}

#[test]
fn one_minute_over_is_a_violation_with_overage_one() {
    let r = build_report(&[e("a", "high", 31, 30)]);
    assert_eq!(r.violations.len(), 1);
    assert_eq!(r.violations[0].overage_minutes, 1);
    assert_eq!(r.compliance_bps, 0);
}

#[test]
fn violation_lines_keep_input_order_and_carry_identity() {
    let r = build_report(&[
        e("z", "low", 200, 120),
        e("ok", "low", 10, 120),
        e("a", "critical", 20, 15),
    ]);
    let ids: Vec<&str> = r.violations.iter().map(|v| v.outage_id.as_str()).collect();
    assert_eq!(ids, alloc::vec!["z", "a"]);
    assert_eq!(r.violations[0].severity, "low");
    assert_eq!(r.violations[0].overage_minutes, 80);
}

#[test]
fn counts_and_rate_are_consistent() {
    let r = build_report(&[
        e("a", "high", 40, 30),
        e("b", "high", 10, 30),
        e("c", "high", 10, 30),
    ]);
    assert_eq!(r.checked, 3);
    assert_eq!(r.compliant + r.violations.len() as u32, r.checked);
    // 2 of 3 compliant = 66.66...% -> 6666 bps
    assert_eq!(r.compliance_bps, 6_666);
}

#[test]
fn worst_first_ranks_by_overage_then_outage_id() {
    let r = build_report(&[
        e("b", "high", 40, 30),
        e("a", "high", 40, 30),
        e("c", "high", 100, 30),
        e("d", "high", 31, 30),
    ]);
    let lines = worst_first(&r);
    let ranked: Vec<&str> = lines.iter().map(|v| v.outage_id.as_str()).collect();
    assert_eq!(ranked, alloc::vec!["c", "a", "b", "d"]);
}

#[test]
fn worst_first_does_not_change_the_report() {
    let r = build_report(&[e("b", "high", 40, 30), e("a", "high", 99, 30)]);
    let before = r.clone();
    let _ = worst_first(&r);
    assert_eq!(r, before);
}

#[test]
fn extreme_mttr_does_not_overflow() {
    let r = build_report(&[e("a", "low", u32::MAX, 1)]);
    assert_eq!(r.violations[0].overage_minutes, u32::MAX - 1);
}
