//! Tests for `reporting::outage_analytics`.

use crate::sla_operations::reporting::outage_analytics::{
    aggregate, longest_outage, sections_by_severity, OutageAggregate, OutageRecord,
};
use alloc::string::String;

fn rec(id: &str, severity: &str, mttr: u32, threshold: u32) -> OutageRecord {
    OutageRecord {
        outage_id: String::from(id),
        severity: String::from(severity),
        mttr_minutes: mttr,
        threshold_minutes: threshold,
    }
}

#[test]
fn empty_input_aggregates_to_all_zeros() {
    let agg = aggregate(&[]);
    assert_eq!(
        agg,
        OutageAggregate {
            count: 0,
            total_mttr_minutes: 0,
            min_mttr_minutes: 0,
            max_mttr_minutes: 0,
            over_threshold: 0,
        }
    );
    assert!(sections_by_severity(&[]).is_empty());
    assert!(longest_outage(&[]).is_none());
}

#[test]
fn aggregate_counts_totals_min_max_and_overages() {
    let records = [
        rec("a", "high", 10, 30),
        rec("b", "high", 45, 30),
        rec("c", "critical", 15, 15),
        rec("d", "critical", 16, 15),
    ];
    let agg = aggregate(&records);
    assert_eq!(agg.count, 4);
    assert_eq!(agg.total_mttr_minutes, 86);
    assert_eq!(agg.min_mttr_minutes, 10);
    assert_eq!(agg.max_mttr_minutes, 45);
    // 45 > 30 and 16 > 15 are over; exactly-at-threshold (15 vs 15) is not.
    assert_eq!(agg.over_threshold, 2);
}

#[test]
fn single_record_sets_min_and_max_to_its_value() {
    let agg = aggregate(&[rec("a", "low", 7, 120)]);
    assert_eq!((agg.min_mttr_minutes, agg.max_mttr_minutes), (7, 7));
    assert_eq!(agg.over_threshold, 0);
}

#[test]
fn sections_are_sorted_by_severity_and_aggregate_independently() {
    let records = [
        rec("a", "low", 100, 120),
        rec("b", "critical", 20, 15),
        rec("c", "low", 130, 120),
        rec("d", "high", 5, 30),
    ];
    let sections = sections_by_severity(&records);
    let names: alloc::vec::Vec<&str> = sections.iter().map(|s| s.severity.as_str()).collect();
    assert_eq!(names, alloc::vec!["critical", "high", "low"]);
    assert_eq!(sections[0].aggregate.count, 1);
    assert_eq!(sections[0].aggregate.over_threshold, 1);
    assert_eq!(sections[2].aggregate.count, 2);
    assert_eq!(sections[2].aggregate.total_mttr_minutes, 230);
    assert_eq!(sections[2].aggregate.over_threshold, 1);
}

#[test]
fn section_counts_add_up_to_the_overall_count() {
    let records = [
        rec("a", "low", 1, 2),
        rec("b", "high", 1, 2),
        rec("c", "low", 1, 2),
    ];
    let total: u32 = sections_by_severity(&records)
        .iter()
        .map(|s| s.aggregate.count)
        .sum();
    assert_eq!(total, aggregate(&records).count);
}

#[test]
fn longest_outage_prefers_the_first_on_ties() {
    let records = [
        rec("first", "high", 50, 30),
        rec("second", "high", 50, 30),
        rec("short", "high", 5, 30),
    ];
    assert_eq!(longest_outage(&records).unwrap().outage_id, "first");
}

#[test]
fn output_is_deterministic_for_the_same_input() {
    let records = [rec("a", "low", 3, 4), rec("b", "high", 9, 4)];
    assert_eq!(sections_by_severity(&records), sections_by_severity(&records));
}

#[test]
fn totals_saturate_instead_of_overflowing() {
    let records = [rec("a", "low", u32::MAX, 1), rec("b", "low", u32::MAX, 1)];
    assert_eq!(aggregate(&records).total_mttr_minutes, 2 * u32::MAX as u64);
}
