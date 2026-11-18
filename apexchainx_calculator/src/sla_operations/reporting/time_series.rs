//! Time-series report formatting.
//!
//! Takes bucketed SLA metrics, merges buckets that share a start time (summing
//! counters), sorts ascending by `bucket_start`, and computes each bucket's
//! violation rate in basis points. A bucket's `violations` is clamped to its
//! `total`.

use alloc::vec::Vec;

/// Raw metrics for one time bucket.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BucketMetric {
    /// Start of the bucket (unix seconds).
    pub bucket_start: u64,
    pub total: u32,
    pub violations: u32,
}

/// One formatted point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SeriesPoint {
    pub bucket_start: u64,
    pub total: u32,
    pub violations: u32,
    /// `violations * 10_000 / total`; `0` when `total == 0`.
    pub violation_rate_bps: u32,
}

/// Build the ordered series for `buckets`.
pub fn build_series(buckets: &[BucketMetric]) -> Vec<SeriesPoint> {
    let mut merged: Vec<BucketMetric> = Vec::new();
    for b in buckets {
        match merged.iter_mut().find(|m| m.bucket_start == b.bucket_start) {
            Some(m) => {
                m.total = m.total.saturating_add(b.total);
                m.violations = m.violations.saturating_add(b.violations);
            }
            None => merged.push(*b),
        }
    }
    merged.sort_by_key(|m| m.bucket_start);
    merged
        .into_iter()
        .map(|m| {
            let violations = m.violations.min(m.total);
            SeriesPoint {
                bucket_start: m.bucket_start,
                total: m.total,
                violations,
                violation_rate_bps: if m.total == 0 {
                    0
                } else {
                    (violations as u64 * 10_000 / m.total as u64) as u32
                },
            }
        })
        .collect()
}

/// Column-oriented view: `(bucket_starts, rates_bps)`, in series order.
pub fn rate_columns(points: &[SeriesPoint]) -> (Vec<u64>, Vec<u32>) {
    (
        points.iter().map(|p| p.bucket_start).collect(),
        points.iter().map(|p| p.violation_rate_bps).collect(),
    )
}

#[cfg(test)]
mod series_smoke_tests {
    use super::*;

    fn b(start: u64, total: u32, violations: u32) -> BucketMetric {
        BucketMetric { bucket_start: start, total, violations }
    }

    #[test]
    fn sorts_merges_and_rates() {
        let series = build_series(&[b(20, 10, 5), b(10, 10, 0), b(20, 10, 5)]);
        assert_eq!(series.len(), 2);
        assert_eq!(series[0].bucket_start, 10);
        assert_eq!((series[1].total, series[1].violation_rate_bps), (20, 5_000));
        assert_eq!(rate_columns(&series).0, alloc::vec![10, 20]);
    }
}
