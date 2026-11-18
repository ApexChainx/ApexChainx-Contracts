//! Compliance / violation report builder.
//!
//! An entry is a *violation* when `mttr_minutes > threshold_minutes` (meeting
//! the threshold exactly is compliant). Violation lines keep input order;
//! [`worst_first`] gives a deterministic ranking.

use alloc::{string::String, vec::Vec};

/// One checked outage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceEntry {
    pub outage_id: String,
    pub severity: String,
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
}

/// A violating outage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViolationLine {
    pub outage_id: String,
    pub severity: String,
    /// `mttr_minutes - threshold_minutes`.
    pub overage_minutes: u32,
}

/// The report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComplianceReport {
    pub checked: u32,
    pub compliant: u32,
    pub violations: Vec<ViolationLine>,
    /// `compliant * 10_000 / checked`; `0` when nothing was checked.
    pub compliance_bps: u32,
}

/// Build the report for `entries`.
pub fn build_report(entries: &[ComplianceEntry]) -> ComplianceReport {
    let mut violations = Vec::new();
    for e in entries {
        if e.mttr_minutes > e.threshold_minutes {
            violations.push(ViolationLine {
                outage_id: e.outage_id.clone(),
                severity: e.severity.clone(),
                overage_minutes: e.mttr_minutes - e.threshold_minutes,
            });
        }
    }
    let checked = entries.len().min(u32::MAX as usize) as u32;
    let compliant = checked.saturating_sub(violations.len().min(u32::MAX as usize) as u32);
    let compliance_bps = if checked == 0 {
        0
    } else {
        (compliant as u64 * 10_000 / checked as u64) as u32
    };
    ComplianceReport {
        checked,
        compliant,
        violations,
        compliance_bps,
    }
}

/// Violations ranked by overage (largest first); ties broken by `outage_id`.
pub fn worst_first(report: &ComplianceReport) -> Vec<ViolationLine> {
    let mut lines = report.violations.clone();
    lines.sort_by(|a, b| {
        b.overage_minutes
            .cmp(&a.overage_minutes)
            .then_with(|| a.outage_id.cmp(&b.outage_id))
    });
    lines
}

#[cfg(test)]
mod compliance_smoke_tests {
    use super::*;

    fn e(id: &str, mttr: u32, threshold: u32) -> ComplianceEntry {
        ComplianceEntry {
            outage_id: String::from(id),
            severity: String::from("high"),
            mttr_minutes: mttr,
            threshold_minutes: threshold,
        }
    }

    #[test]
    fn at_threshold_is_compliant_and_over_is_not() {
        let r = build_report(&[e("a", 30, 30), e("b", 31, 30)]);
        assert_eq!((r.checked, r.compliant), (2, 1));
        assert_eq!(r.violations[0].outage_id, "b");
        assert_eq!(r.violations[0].overage_minutes, 1);
        assert_eq!(r.compliance_bps, 5_000);
    }

    #[test]
    fn worst_first_ranks_by_overage() {
        let r = build_report(&[e("a", 40, 30), e("b", 90, 30)]);
        assert_eq!(worst_first(&r)[0].outage_id, "b");
    }
}
