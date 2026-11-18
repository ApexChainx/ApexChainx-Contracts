//! Settlement summary builder.
//!
//! Splits signed settlement amounts into rewards (positive) and penalties
//! (negative) and reports totals. Penalty totals are reported as positive
//! magnitudes. Arithmetic saturates, so extreme inputs cannot panic; a
//! saturated total is not an exact sum.

use alloc::string::String;

/// One settlement. Positive = reward, negative = penalty, zero = neutral.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settlement {
    pub outage_id: String,
    pub amount: i128,
}

/// Aggregated settlement figures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SettlementSummary {
    /// Sum of positive amounts.
    pub rewards_total: i128,
    /// Sum of the magnitudes of negative amounts.
    pub penalties_total: i128,
    /// `rewards_total - penalties_total`, saturating.
    pub net: i128,
    pub reward_count: u32,
    pub penalty_count: u32,
    pub zero_count: u32,
}

/// Build the summary for `settlements`.
pub fn build_settlement_summary(settlements: &[Settlement]) -> SettlementSummary {
    let mut rewards_total = 0i128;
    let mut penalties_total = 0i128;
    let mut reward_count = 0u32;
    let mut penalty_count = 0u32;
    let mut zero_count = 0u32;
    for s in settlements {
        if s.amount > 0 {
            rewards_total = rewards_total.saturating_add(s.amount);
            reward_count = reward_count.saturating_add(1);
        } else if s.amount < 0 {
            penalties_total = penalties_total.saturating_add(s.amount.saturating_neg());
            penalty_count = penalty_count.saturating_add(1);
        } else {
            zero_count = zero_count.saturating_add(1);
        }
    }
    SettlementSummary {
        rewards_total,
        penalties_total,
        net: rewards_total.saturating_sub(penalties_total),
        reward_count,
        penalty_count,
        zero_count,
    }
}

#[cfg(test)]
mod settlement_smoke_tests {
    use super::*;

    #[test]
    fn splits_rewards_and_penalties() {
        let input = [
            Settlement { outage_id: String::from("a"), amount: 100 },
            Settlement { outage_id: String::from("b"), amount: -30 },
            Settlement { outage_id: String::from("c"), amount: 0 },
        ];
        let s = build_settlement_summary(&input);
        assert_eq!((s.rewards_total, s.penalties_total, s.net), (100, 30, 70));
        assert_eq!((s.reward_count, s.penalty_count, s.zero_count), (1, 1, 1));
    }
}
