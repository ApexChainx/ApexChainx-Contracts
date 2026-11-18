//! Outage analytics report builders.
//!
//! Aggregates outage records overall and per severity. Output ordering is
//! deterministic: severity sections are sorted by severity name (byte order);
//! ties for the longest outage resolve to the earliest record in the input.

use alloc::{string::String, vec::Vec};

/// One outage as supplied by the caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutageRecord {
    pub outage_id: String,
    pub severity: String,
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
}

/// Aggregate figures for a group of outages. All zeros for an empty group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutageAggregate {
    pub count: u32,
    pub total_mttr_minutes: u64,
    pub min_mttr_minutes: u32,
    pub max_mttr_minutes: u32,
    /// Outages whose MTTR is strictly greater than their threshold.
    pub over_threshold: u32,
}

/// Aggregate for one severity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeveritySection {
    pub severity: String,
    pub aggregate: OutageAggregate,
}

fn fold<'a, I: Iterator<Item = &'a OutageRecord>>(records: I) -> OutageAggregate {
    let mut agg = OutageAggregate {
        count: 0,
        total_mttr_minutes: 0,
        min_mttr_minutes: 0,
        max_mttr_minutes: 0,
        over_threshold: 0,
    };
    for r in records {
        if agg.count == 0 {
            agg.min_mttr_minutes = r.mttr_minutes;
            agg.max_mttr_minutes = r.mttr_minutes;
        } else {
            agg.min_mttr_minutes = agg.min_mttr_minutes.min(r.mttr_minutes);
            agg.max_mttr_minutes = agg.max_mttr_minutes.max(r.mttr_minutes);
        }
        agg.count = agg.count.saturating_add(1);
        agg.total_mttr_minutes = agg.total_mttr_minutes.saturating_add(r.mttr_minutes as u64);
        if r.mttr_minutes > r.threshold_minutes {
            agg.over_threshold = agg.over_threshold.saturating_add(1);
        }
    }
    agg
}

/// Aggregate every record.
pub fn aggregate(records: &[OutageRecord]) -> OutageAggregate {
    fold(records.iter())
}

/// Aggregate per severity, sorted by severity name.
pub fn sections_by_severity(records: &[OutageRecord]) -> Vec<SeveritySection> {
    let mut severities: Vec<&str> = Vec::new();
    for r in records {
        if !severities.contains(&r.severity.as_str()) {
            severities.push(r.severity.as_str());
        }
    }
    severities.sort_unstable();
    severities
        .into_iter()
        .map(|severity| SeveritySection {
            severity: String::from(severity),
            aggregate: fold(records.iter().filter(|r| r.severity == severity)),
        })
        .collect()
}

/// The outage with the largest MTTR; the first one wins ties.
pub fn longest_outage(records: &[OutageRecord]) -> Option<&OutageRecord> {
    let mut best: Option<&OutageRecord> = None;
    for r in records {
        match best {
            Some(b) if b.mttr_minutes >= r.mttr_minutes => {}
            _ => best = Some(r),
        }
    }
    best
}

#[cfg(test)]
mod analytics_smoke_tests {
    use super::*;

    fn rec(id: &str, severity: &str, mttr: u32, threshold: u32) -> OutageRecord {
        OutageRecord {
            outage_id: String::from(id),
            severity: String::from(severity),
            mttr_minutes: mttr,
            threshold_minutes: threshold,
        }
    }

    #[test]
    fn aggregates_and_groups() {
        let records = [rec("a", "low", 10, 5), rec("b", "high", 2, 5), rec("c", "low", 4, 5)];
        let agg = aggregate(&records);
        assert_eq!((agg.count, agg.over_threshold, agg.max_mttr_minutes), (3, 1, 10));
        let sections = sections_by_severity(&records);
        assert_eq!(sections[0].severity, "high");
        assert_eq!(sections[1].aggregate.count, 2);
        assert_eq!(longest_outage(&records).unwrap().outage_id, "a");
    }
}
