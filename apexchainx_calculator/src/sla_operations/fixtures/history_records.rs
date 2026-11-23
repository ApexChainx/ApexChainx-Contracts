//! History record fixtures covering met and violated outcomes.
//!
//! Amounts follow the calculator's convention: positive = reward, negative =
//! penalty.

use crate::sla_operations::reporting::history_export::HistoryRecord;
use alloc::{string::String, vec::Vec};

#[allow(clippy::too_many_arguments)]
fn record(
    outage_id: &str,
    severity: &str,
    status: &str,
    mttr: u32,
    threshold: u32,
    amount: i128,
    payment_type: &str,
    rating: &str,
) -> HistoryRecord {
    HistoryRecord {
        outage_id: String::from(outage_id),
        severity: String::from(severity),
        status: String::from(status),
        mttr_minutes: mttr,
        threshold_minutes: threshold,
        amount,
        payment_type: String::from(payment_type),
        rating: String::from(rating),
    }
}

/// A critical outage repaired in 5 of 15 minutes: reward.
pub fn met_record() -> HistoryRecord {
    record("hist-met", "critical", "met", 5, 15, 750, "rew", "top")
}

/// A critical outage repaired in 45 of 15 minutes: 30 minutes over at 100/min.
pub fn violated_record() -> HistoryRecord {
    record("hist-viol", "critical", "viol", 45, 15, -3_000, "pen", "poor")
}

/// A medium outage that meets its threshold exactly.
pub fn boundary_record() -> HistoryRecord {
    record("hist-edge", "medium", "met", 60, 60, 750, "rew", "good")
}

/// Three records in a fixed order: met, violated, boundary.
pub fn mixed_history() -> Vec<HistoryRecord> {
    alloc::vec![met_record(), violated_record(), boundary_record()]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn fixtures_cover_both_outcomes_consistently() {
        let h = mixed_history();
        assert_eq!(h.len(), 3);
        assert!(h.iter().any(|r| r.status == "viol" && r.amount < 0));
        assert!(h.iter().any(|r| r.status == "met" && r.amount > 0));
        for r in &h {
            assert_eq!(r.status == "viol", r.mttr_minutes > r.threshold_minutes);
        }
    }
}
