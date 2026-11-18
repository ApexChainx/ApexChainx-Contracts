//! Intentionally invalid inputs for validation tests.
//!
//! Each fixture names *why* it is invalid, so a test can assert on the specific
//! rejection rather than on "some error".

use alloc::{string::String, vec::Vec};

/// An invalid report window `(start, end)` and the reason it is invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidWindow {
    pub start: u64,
    pub end: u64,
    pub reason: &'static str,
}

/// Invalid windows: inverted, empty, and longer than 366 days.
pub fn invalid_windows() -> Vec<InvalidWindow> {
    alloc::vec![
        InvalidWindow { start: 100, end: 50, reason: "inverted" },
        InvalidWindow { start: 100, end: 100, reason: "empty" },
        InvalidWindow { start: 0, end: 366 * 24 * 60 * 60 + 1, reason: "too_long" },
    ]
}

/// An invalid `(total, violations)` counter pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidCounters {
    pub total: u32,
    pub violations: u32,
}

/// Counters where violations exceed the total.
pub fn invalid_counters() -> Vec<InvalidCounters> {
    alloc::vec![
        InvalidCounters { total: 0, violations: 1 },
        InvalidCounters { total: 5, violations: 6 },
        InvalidCounters { total: 0, violations: u32::MAX },
    ]
}

/// An owned record with a deliberate defect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidRecord {
    pub outage_id: String,
    pub severity: String,
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
    pub reason: &'static str,
}

fn bad(id: &str, severity: &str, mttr: u32, threshold: u32, reason: &'static str) -> InvalidRecord {
    InvalidRecord {
        outage_id: String::from(id),
        severity: String::from(severity),
        mttr_minutes: mttr,
        threshold_minutes: threshold,
        reason,
    }
}

/// Records with an empty id, an empty severity, or a zero threshold.
pub fn invalid_records() -> Vec<InvalidRecord> {
    alloc::vec![
        bad("", "high", 10, 30, "empty_outage_id"),
        bad("inv-2", "", 10, 30, "empty_severity"),
        bad("inv-3", "high", 10, 0, "zero_threshold"),
    ]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn every_fixture_is_actually_invalid() {
        assert!(invalid_windows().iter().all(|w| w.end <= w.start || w.end - w.start > 366 * 86_400));
        assert!(invalid_counters().iter().all(|c| c.violations > c.total));
        assert!(invalid_records()
            .iter()
            .all(|r| r.outage_id.is_empty() || r.severity.is_empty() || r.threshold_minutes == 0));
    }
}
