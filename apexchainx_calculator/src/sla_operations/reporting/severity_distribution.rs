//! Severity distribution report.
//!
//! Merges counts per severity (duplicate severities are summed), sorts rows by
//! severity name, and reports each severity's share in basis points. Shares are
//! truncated, so they can sum to slightly less than `10_000`; they are all `0`
//! when the total count is zero.

use alloc::{string::String, vec::Vec};

/// A count for one severity (the same severity may appear more than once).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeverityCount {
    pub severity: String,
    pub count: u32,
}

/// One row of the distribution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionRow {
    pub severity: String,
    pub count: u32,
    /// `count * 10_000 / total`, truncated.
    pub share_bps: u32,
}

/// Build the distribution for `counts`.
pub fn build_distribution(counts: &[SeverityCount]) -> Vec<DistributionRow> {
    let mut merged: Vec<(String, u32)> = Vec::new();
    for c in counts {
        match merged.iter_mut().find(|(s, _)| *s == c.severity) {
            Some((_, n)) => *n = n.saturating_add(c.count),
            None => merged.push((c.severity.clone(), c.count)),
        }
    }
    merged.sort_by(|a, b| a.0.cmp(&b.0));
    let total: u64 = merged.iter().map(|(_, n)| *n as u64).sum();
    merged
        .into_iter()
        .map(|(severity, count)| DistributionRow {
            severity,
            count,
            share_bps: if total == 0 {
                0
            } else {
                (count as u64 * 10_000 / total) as u32
            },
        })
        .collect()
}

/// Sum of all counts in `rows`.
pub fn total_count(rows: &[DistributionRow]) -> u64 {
    rows.iter().map(|r| r.count as u64).sum()
}

#[cfg(test)]
mod distribution_smoke_tests {
    use super::*;

    fn sc(severity: &str, count: u32) -> SeverityCount {
        SeverityCount { severity: String::from(severity), count }
    }

    #[test]
    fn merges_duplicates_and_computes_shares() {
        let rows = build_distribution(&[sc("low", 1), sc("high", 3), sc("high", 4)]);
        assert_eq!(rows[0].severity, "high");
        assert_eq!((rows[0].count, rows[0].share_bps), (7, 8_750));
        assert_eq!(total_count(&rows), 8);
    }

    #[test]
    fn zero_total_gives_zero_shares() {
        assert_eq!(build_distribution(&[sc("a", 0)])[0].share_bps, 0);
    }
}
