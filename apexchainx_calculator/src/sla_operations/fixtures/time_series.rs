//! Deterministic time-series metric fixtures.
//!
//! Bucket starts are multiples of [`BUCKET_SECONDS`] from [`BASE_TIME`], so
//! fixtures never depend on a clock.

use crate::sla_operations::reporting::time_series::BucketMetric;
use alloc::vec::Vec;

/// First bucket start used by the fixtures (unix seconds).
pub const BASE_TIME: u64 = 1_700_000_000;
/// Bucket width used by the fixtures (one hour).
pub const BUCKET_SECONDS: u64 = 3_600;

/// Build the bucket at position `index` (0 = [`BASE_TIME`]).
pub fn bucket(index: u64, total: u32, violations: u32) -> BucketMetric {
    BucketMetric {
        bucket_start: BASE_TIME + index * BUCKET_SECONDS,
        total,
        violations,
    }
}

/// Four buckets with a rising violation rate: 0%, 10%, 25%, 50%.
pub fn rising_trend() -> Vec<BucketMetric> {
    alloc::vec![
        bucket(0, 20, 0),
        bucket(1, 20, 2),
        bucket(2, 20, 5),
        bucket(3, 20, 10),
    ]
}

/// Constant 10% rate over four buckets.
pub fn flat_trend() -> Vec<BucketMetric> {
    (0..4).map(|i| bucket(i, 10, 1)).collect()
}

/// The rising trend delivered newest-first, to exercise ordering.
pub fn out_of_order() -> Vec<BucketMetric> {
    let mut v = rising_trend();
    v.reverse();
    v
}

/// Two entries share bucket 1 and should merge.
pub fn duplicate_bucket() -> Vec<BucketMetric> {
    alloc::vec![bucket(0, 10, 1), bucket(1, 10, 1), bucket(1, 10, 3)]
}

/// A bucket with no activity.
pub fn empty_bucket() -> Vec<BucketMetric> {
    alloc::vec![bucket(0, 0, 0)]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn fixtures_are_aligned_and_shaped_as_documented() {
        assert_eq!(rising_trend().len(), 4);
        assert!(rising_trend()
            .windows(2)
            .all(|w| w[1].bucket_start - w[0].bucket_start == BUCKET_SECONDS));
        assert!(out_of_order()[0].bucket_start > out_of_order()[3].bucket_start);
        assert_eq!(duplicate_bucket()[1].bucket_start, duplicate_bucket()[2].bucket_start);
        assert_eq!(empty_bucket()[0].total, 0);
    }
}
