//! Deterministic violation-rate helpers for the SLA operations workspace.

/// Basis-point denominator used by the rate helpers.
pub const BASIS_POINTS_DENOMINATOR: u32 = 10_000;

/// A validated violation rate expressed in basis points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViolationRate {
    /// Number of observed violations.
    pub violated: u64,
    /// Total number of observations.
    pub total: u64,
    /// Violations as basis points of the total.
    pub basis_points: u32,
}

/// Computes `violated / total` in basis points.
///
/// Returns `None` when `total` is zero or when `violated` exceeds `total`,
/// because neither case represents a valid rate.
#[must_use]
pub const fn violation_rate_bps(violated: u64, total: u64) -> Option<u32> {
    if total == 0 || violated > total {
        return None;
    }

    Some(((violated as u128 * BASIS_POINTS_DENOMINATOR as u128) / total as u128) as u32)
}

/// Computes the complementary compliance rate in basis points.
#[must_use]
pub const fn compliance_rate_bps(violated: u64, total: u64) -> Option<u32> {
    match violation_rate_bps(violated, total) {
        Some(rate) => Some(BASIS_POINTS_DENOMINATOR - rate),
        None => None,
    }
}

/// Builds a validated [`ViolationRate`] from raw counts.
#[must_use]
pub const fn violation_rate(violated: u64, total: u64) -> Option<ViolationRate> {
    match violation_rate_bps(violated, total) {
        Some(basis_points) => Some(ViolationRate {
            violated,
            total,
            basis_points,
        }),
        None => None,
    }
}

/// Returns whether a violation rate stays within `maximum_bps`.
///
/// Returns `None` when the supplied counts do not form a valid rate.
#[must_use]
pub const fn within_violation_budget(
    violated: u64,
    total: u64,
    maximum_bps: u32,
) -> Option<bool> {
    match violation_rate_bps(violated, total) {
        Some(rate) => Some(rate <= maximum_bps),
        None => None,
    }
}
