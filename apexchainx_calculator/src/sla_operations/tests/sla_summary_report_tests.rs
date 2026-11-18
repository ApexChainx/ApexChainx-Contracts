//! Tests for `reporting::sla_summary`.

use crate::sla_operations::reporting::sla_summary::{build_summary, SlaOutcome};
use alloc::string::String;

fn o(met: bool, mttr: u32) -> SlaOutcome {
    SlaOutcome {
        severity: String::from("high"),
        met,
        mttr_minutes: mttr,
    }
}

#[test]
fn empty_input_is_all_zeros_including_compliance() {
    let s = build_summary(&[]);
    assert_eq!((s.total, s.met, s.violated), (0, 0, 0));
    assert_eq!(s.compliance_bps, 0);
    assert_eq!((s.total_mttr_minutes, s.avg_mttr_minutes, s.worst_mttr_minutes), (0, 0, 0));
}

#[test]
fn all_met_is_full_compliance() {
    let s = build_summary(&[o(true, 5), o(true, 10)]);
    assert_eq!((s.met, s.violated), (2, 0));
    assert_eq!(s.compliance_bps, 10_000);
}

#[test]
fn all_violated_is_zero_compliance() {
    let s = build_summary(&[o(false, 50), o(false, 70)]);
    assert_eq!((s.met, s.violated), (0, 2));
    assert_eq!(s.compliance_bps, 0);
}

#[test]
fn mixed_outcomes_report_counts_and_rate() {
    let s = build_summary(&[o(true, 5), o(false, 45), o(true, 15), o(true, 15)]);
    assert_eq!((s.total, s.met, s.violated), (4, 3, 1));
    assert_eq!(s.compliance_bps, 7_500);
}

#[test]
fn compliance_truncates_toward_zero() {
    // 1 of 3 met = 33.33...% -> 3333
    let s = build_summary(&[o(true, 1), o(false, 1), o(false, 1)]);
    assert_eq!(s.compliance_bps, 3_333);
}

#[test]
fn mttr_total_average_and_worst() {
    let s = build_summary(&[o(true, 10), o(true, 20), o(false, 45)]);
    assert_eq!(s.total_mttr_minutes, 75);
    assert_eq!(s.avg_mttr_minutes, 25);
    assert_eq!(s.worst_mttr_minutes, 45);
}

#[test]
fn average_truncates_toward_zero() {
    let s = build_summary(&[o(true, 1), o(true, 2)]);
    assert_eq!(s.avg_mttr_minutes, 1);
}

#[test]
fn met_plus_violated_equals_total() {
    let s = build_summary(&[o(true, 1), o(false, 2), o(false, 3), o(true, 4), o(true, 5)]);
    assert_eq!(s.met + s.violated, s.total);
}

#[test]
fn large_mttr_values_do_not_overflow() {
    let s = build_summary(&[o(false, u32::MAX), o(false, u32::MAX)]);
    assert_eq!(s.total_mttr_minutes, 2 * u32::MAX as u64);
    assert_eq!(s.avg_mttr_minutes, u32::MAX);
    assert_eq!(s.worst_mttr_minutes, u32::MAX);
}

#[test]
fn same_input_gives_the_same_summary() {
    let input = [o(true, 3), o(false, 9)];
    assert_eq!(build_summary(&input), build_summary(&input));
}
