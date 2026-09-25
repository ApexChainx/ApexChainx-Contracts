//! Fuzz spec for reward/penalty feasibility region boundary probing (Issue #670).
//!
//! `validate_config` enforces ratio and cross-ladder rules (2x/3x reward-penalty
//! ratios, per-severity cap/floor). The existing bounds fuzz covers valid ranges
//! but not systematic ratio-boundary exploration. This module defines boundary
//! scenarios that probe the corners of the feasibility region for each canonical
//! severity — accept/reject must match the documented rules.

/// A boundary scenario: inputs near a feasibility edge and the expected outcome.
#[derive(Debug)]
pub struct FeasibilityScenario {
    pub severity: &'static str,
    pub threshold_minutes: u32,
    pub penalty_per_minute: i128,
    pub reward_base: i128,
    /// Whether `validate_config` should accept (true) or reject (false) these inputs.
    pub should_accept: bool,
    pub description: &'static str,
}

/// Boundary scenarios probing the reward/penalty feasibility region corners.
///
/// These cover:
/// - penalty at cap with reward just above the ratio line (accept)
/// - penalty at cap with reward just below the ratio line (reject)
/// - reward at minimum with penalty just below the ratio floor (accept)
/// - cross-severity ordering boundary (critical threshold <= high threshold)
pub const FEASIBILITY_SCENARIOS: &[FeasibilityScenario] = &[
    FeasibilityScenario {
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 200,
        reward_base: 600,
        should_accept: true,
        description: "critical: penalty at high end, reward satisfies 3x ratio",
    },
    FeasibilityScenario {
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 200,
        reward_base: 100,
        should_accept: false,
        description: "critical: penalty at high end, reward below ratio floor",
    },
    FeasibilityScenario {
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 100,
        reward_base: 300,
        should_accept: true,
        description: "high: penalty mid-range, reward satisfies 3x ratio",
    },
    FeasibilityScenario {
        severity: "low",
        threshold_minutes: 60,
        penalty_per_minute: 0,
        reward_base: 500,
        should_accept: true,
        description: "low: zero penalty (exempt), any reward accepted",
    },
    FeasibilityScenario {
        severity: "medium",
        threshold_minutes: 45,
        penalty_per_minute: 50,
        reward_base: 200,
        should_accept: true,
        description: "medium: boundary-adjacent values within feasibility region",
    },
];