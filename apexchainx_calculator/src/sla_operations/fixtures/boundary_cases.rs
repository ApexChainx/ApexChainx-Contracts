//! Boundary-value fixtures for MTTR / threshold comparisons.
//!
//! The SLA is met when `mttr_minutes <= threshold_minutes`.

use alloc::vec::Vec;

/// One boundary scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundaryCase {
    pub name: &'static str,
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
    /// Whether the SLA is met (`mttr <= threshold`).
    pub expect_met: bool,
}

const fn case(name: &'static str, mttr: u32, threshold: u32) -> BoundaryCase {
    BoundaryCase {
        name,
        mttr_minutes: mttr,
        threshold_minutes: threshold,
        expect_met: mttr <= threshold,
    }
}

/// MTTR/threshold edge cases.
pub fn mttr_cases() -> Vec<BoundaryCase> {
    alloc::vec![
        case("zero_mttr", 0, 15),
        case("one_under", 14, 15),
        case("exactly_at_threshold", 15, 15),
        case("one_over", 16, 15),
        case("min_threshold_met", 1, 1),
        case("min_threshold_violated", 2, 1),
        case("max_mttr", u32::MAX, 15),
        case("max_threshold", 15, u32::MAX),
        case("both_max", u32::MAX, u32::MAX),
    ]
}

/// Ratio edge cases as `(numerator, denominator, expected_bps)` with bps =
/// `numerator * 10_000 / denominator` (truncated) and `0` for a zero denominator.
pub fn ratio_cases() -> Vec<(u32, u32, u32)> {
    alloc::vec![
        (0, 10, 0),
        (1, 3, 3_333),
        (2, 3, 6_666),
        (10, 10, 10_000),
        (5, 0, 0),
    ]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn expectations_match_the_rule() {
        for c in mttr_cases() {
            assert_eq!(c.expect_met, c.mttr_minutes <= c.threshold_minutes, "{}", c.name);
        }
        for (n, d, expected) in ratio_cases() {
            let got = if d == 0 { 0 } else { (n as u64 * 10_000 / d as u64) as u32 };
            assert_eq!(got, expected);
        }
    }
}
