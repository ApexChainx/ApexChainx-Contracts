//! Deterministic compliance-score helpers for the SLA operations workspace.

/// Basis-point denominator used by the score helpers.
pub const BASIS_POINTS_DENOMINATOR: u32 = 10_000;

/// A compliance score derived from met and violated counts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComplianceScore {
    /// Number of met obligations.
    pub met: u64,
    /// Number of violated obligations.
    pub violated: u64,
    /// Met obligations as basis points of the total.
    pub basis_points: u32,
}

/// Computes the compliance score in basis points.
///
/// Returns `None` when both counts are zero.
#[must_use]
pub const fn compliance_score_bps(met: u64, violated: u64) -> Option<u32> {
    let total = met as u128 + violated as u128;

    if total == 0 {
        return None;
    }

    Some(((met as u128 * BASIS_POINTS_DENOMINATOR as u128) / total) as u32)
}

/// Computes the non-compliance score in basis points.
#[must_use]
pub const fn non_compliance_score_bps(met: u64, violated: u64) -> Option<u32> {
    match compliance_score_bps(met, violated) {
        Some(score) => Some(BASIS_POINTS_DENOMINATOR - score),
        None => None,
    }
}

/// Builds a validated [`ComplianceScore`] from raw counts.
#[must_use]
pub const fn compliance_score(met: u64, violated: u64) -> Option<ComplianceScore> {
    match compliance_score_bps(met, violated) {
        Some(basis_points) => Some(ComplianceScore {
            met,
            violated,
            basis_points,
        }),
        None => None,
    }
}

/// Returns whether the compliance score meets `target_bps`.
///
/// Returns `None` when both counts are zero.
#[must_use]
pub const fn meets_compliance_target(
    met: u64,
    violated: u64,
    target_bps: u32,
) -> Option<bool> {
    match compliance_score_bps(met, violated) {
        Some(score) => Some(score >= target_bps),
        None => None,
    }
}
