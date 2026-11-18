//! Tests for `reporting::time_series`.

use crate::sla_operations::reporting::time_series::{
    build_series, rate_columns, BucketMetric,
};
use alloc::vec::Vec;

fn b(start: u64, total: u32, violations: u32) -> BucketMetric {
    BucketMetric {
        bucket_start: start,
        total,
        violations,
    }
}

#[test]
fn empty_input_gives_an_empty_series() {
    assert!(build_series(&[]).is_empty());
    let (starts, rates) = rate_columns(&[]);
    assert!(starts.is_empty() && rates.is_empty());
}

#[test]
fn points_are_sorted_ascending_by_bucket_start() {
    let series = build_series(&[b(300, 1, 0), b(100, 1, 0), b(200, 1, 0)]);
    let starts: Vec<u64> = series.iter().map(|p| p.bucket_start).collect();
    assert_eq!(starts, alloc::vec![100, 200, 300]);
}

#[test]
fn rates_are_basis_points_truncated() {
    let series = build_series(&[b(0, 20, 0), b(1, 20, 2), b(2, 3, 1), b(3, 20, 10)]);
    let rates: Vec<u32> = series.iter().map(|p| p.violation_rate_bps).collect();
    assert_eq!(rates, alloc::vec![0, 1_000, 3_333, 5_000]);
}

#[test]
fn an_empty_bucket_has_a_zero_rate() {
    let series = build_series(&[b(0, 0, 0)]);
    assert_eq!(series[0].violation_rate_bps, 0);
    assert_eq!(series[0].total, 0);
}

#[test]
fn buckets_with_the_same_start_are_merged() {
    let series = build_series(&[b(0, 10, 1), b(60, 10, 1), b(60, 10, 3)]);
    assert_eq!(series.len(), 2);
    assert_eq!(series[1].total, 20);
    assert_eq!(series[1].violations, 4);
    assert_eq!(series[1].violation_rate_bps, 2_000);
}

#[test]
fn violations_above_total_are_clamped() {
    let series = build_series(&[b(0, 2, 9)]);
    assert_eq!(series[0].violations, 2);
    assert_eq!(series[0].violation_rate_bps, 10_000);
}

#[test]
fn input_order_does_not_change_the_output() {
    let forward = [b(1, 4, 1), b(2, 5, 0), b(3, 6, 6)];
    let backward = [b(3, 6, 6), b(2, 5, 0), b(1, 4, 1)];
    assert_eq!(build_series(&forward), build_series(&backward));
}

#[test]
fn rate_columns_follow_series_order() {
    let series = build_series(&[b(20, 10, 5), b(10, 10, 0)]);
    let (starts, rates) = rate_columns(&series);
    assert_eq!(starts, alloc::vec![10, 20]);
    assert_eq!(rates, alloc::vec![0, 5_000]);
}

#[test]
fn counters_saturate_when_merging() {
    let series = build_series(&[b(0, u32::MAX, 0), b(0, 10, 0)]);
    assert_eq!(series[0].total, u32::MAX);
}
