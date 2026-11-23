//! Telemetry counter fixtures.

use crate::sla_operations::reporting::telemetry_snapshot::SeverityCounters;
use alloc::{string::String, vec::Vec};

/// Build one counter row.
pub fn counters(severity: &str, calculations: u32, violations: u32) -> SeverityCounters {
    SeverityCounters {
        severity: String::from(severity),
        calculations,
        violations,
    }
}

/// Representative snapshot (deliberately not sorted by severity).
pub fn typical_snapshot() -> Vec<SeverityCounters> {
    alloc::vec![
        counters("low", 40, 1),
        counters("critical", 10, 3),
        counters("high", 20, 2),
        counters("medium", 30, 0),
    ]
}

/// No activity at all.
pub fn empty_snapshot() -> Vec<SeverityCounters> {
    Vec::new()
}

/// A severity with calculations but no violations, and one with neither.
pub fn quiet_snapshot() -> Vec<SeverityCounters> {
    alloc::vec![counters("low", 25, 0), counters("high", 0, 0)]
}

/// Counters whose `violations` exceed `calculations` (malformed input).
pub fn inconsistent_snapshot() -> Vec<SeverityCounters> {
    alloc::vec![counters("critical", 2, 5)]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn snapshots_have_the_documented_shape() {
        assert_eq!(typical_snapshot().len(), 4);
        assert!(empty_snapshot().is_empty());
        assert!(quiet_snapshot().iter().all(|c| c.violations == 0));
        assert!(inconsistent_snapshot().iter().all(|c| c.violations > c.calculations));
    }
}
