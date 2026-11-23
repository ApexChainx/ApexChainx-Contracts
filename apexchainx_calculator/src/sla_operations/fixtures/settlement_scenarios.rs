//! Reward / penalty settlement scenario fixtures.

use crate::sla_operations::reporting::settlement_summary::Settlement;
use alloc::{format, string::String, vec::Vec};

/// Build one settlement.
pub fn settlement(outage_id: &str, amount: i128) -> Settlement {
    Settlement {
        outage_id: String::from(outage_id),
        amount,
    }
}

/// Only rewards: 750 + 600.
pub fn rewards_only() -> Vec<Settlement> {
    alloc::vec![settlement("s-r1", 750), settlement("s-r2", 600)]
}

/// Only penalties: 3_000 + 500.
pub fn penalties_only() -> Vec<Settlement> {
    alloc::vec![settlement("s-p1", -3_000), settlement("s-p2", -500)]
}

/// Mixed: rewards 750, penalties 1_000, one neutral. Net is -250.
pub fn mixed() -> Vec<Settlement> {
    alloc::vec![
        settlement("s-m1", 750),
        settlement("s-m2", -1_000),
        settlement("s-m3", 0),
    ]
}

/// A reward and a penalty of identical magnitude: net is exactly 0.
pub fn net_zero() -> Vec<Settlement> {
    alloc::vec![settlement("s-z1", 400), settlement("s-z2", -400)]
}

/// `count` penalties of 10 each, ids `s-bulk-<i>`.
pub fn bulk_penalties(count: u32) -> Vec<Settlement> {
    (0..count)
        .map(|i| settlement(&format!("s-bulk-{i}"), -10))
        .collect()
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn scenarios_have_the_documented_totals() {
        let sum = |v: &[Settlement]| v.iter().map(|s| s.amount).sum::<i128>();
        assert_eq!(sum(&rewards_only()), 1_350);
        assert_eq!(sum(&penalties_only()), -3_500);
        assert_eq!(sum(&mixed()), -250);
        assert_eq!(sum(&net_zero()), 0);
        assert_eq!(bulk_penalties(4).len(), 4);
        assert_eq!(sum(&bulk_penalties(4)), -40);
    }
}
