//! SLA summary report builder.
//!
//! Folds a slice of per-outage outcomes into one summary. Pure and
//! deterministic: no state is read or written. Integer division truncates
//! toward zero. For an empty input every figure is `0` (there is nothing to
//! be compliant *with*, so `compliance_bps` is `0`, not `10_000`).

use alloc::string::String;

/// One outage's SLA outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlaOutcome {
    pub severity: String,
    /// `true` when the SLA was met.
    pub met: bool,
    pub mttr_minutes: u32,
}

/// Aggregated figures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlaSummary {
    pub total: u32,
    pub met: u32,
    pub violated: u32,
    /// `met * 10_000 / total`, truncated; `0` when `total == 0`.
    pub compliance_bps: u32,
    pub total_mttr_minutes: u64,
    /// `total_mttr_minutes / total`, truncated; `0` when `total == 0`.
    pub avg_mttr_minutes: u32,
    /// Largest single MTTR; `0` when `total == 0`.
    pub worst_mttr_minutes: u32,
}

/// Build the summary for `outcomes`.
pub fn build_summary(outcomes: &[SlaOutcome]) -> SlaSummary {
    let mut met = 0u32;
    let mut violated = 0u32;
    let mut total_mttr = 0u64;
    let mut worst = 0u32;
    for outcome in outcomes {
        if outcome.met {
            met = met.saturating_add(1);
        } else {
            violated = violated.saturating_add(1);
        }
        total_mttr = total_mttr.saturating_add(outcome.mttr_minutes as u64);
        worst = worst.max(outcome.mttr_minutes);
    }
    let total = met.saturating_add(violated);
    let (compliance_bps, avg) = if total == 0 {
        (0, 0)
    } else {
        (
            (met as u64 * 10_000 / total as u64) as u32,
            (total_mttr / total as u64).min(u32::MAX as u64) as u32,
        )
    };
    SlaSummary {
        total,
        met,
        violated,
        compliance_bps,
        total_mttr_minutes: total_mttr,
        avg_mttr_minutes: avg,
        worst_mttr_minutes: worst,
    }
}

#[cfg(test)]
mod summary_smoke_tests {
    use super::*;

    fn o(met: bool, mttr: u32) -> SlaOutcome {
        SlaOutcome { severity: String::from("high"), met, mttr_minutes: mttr }
    }

    #[test]
    fn summarises_counts_rate_and_mttr() {
        let s = build_summary(&[o(true, 10), o(false, 40)]);
        assert_eq!((s.total, s.met, s.violated), (2, 1, 1));
        assert_eq!(s.compliance_bps, 5_000);
        assert_eq!((s.avg_mttr_minutes, s.worst_mttr_minutes), (25, 40));
    }

    #[test]
    fn empty_input_is_all_zeros() {
        assert_eq!(build_summary(&[]).compliance_bps, 0);
    }
}
