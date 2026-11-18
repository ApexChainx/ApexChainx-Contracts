//! Deterministic severity weighting helpers for the SLA operations workspace.

/// Severity levels used by SLA analytics helpers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    /// Low severity.
    Low,
    /// Medium severity.
    Medium,
    /// High severity.
    High,
    /// Critical severity.
    Critical,
}

impl Severity {
    /// Returns the weight of this severity in basis points.
    #[must_use]
    pub const fn weight_bps(self) -> u32 {
        match self {
            Self::Low => 2_500,
            Self::Medium => 5_000,
            Self::High => 7_500,
            Self::Critical => 10_000,
        }
    }

    /// Returns a stable, lowercase label for this severity.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Applies a severity weight to a basis-point score.
#[must_use]
pub const fn weighted_score_bps(severity: Severity, score_bps: u32) -> u32 {
    (score_bps as u64 * severity.weight_bps() as u64 / 10_000) as u32
}

/// Returns the total weight of a severity sequence, saturating at [`u64::MAX`].
#[must_use]
pub fn total_weight_bps(severities: &[Severity]) -> u64 {
    let mut total = 0_u64;
    for &severity in severities {
        total = total.saturating_add(severity.weight_bps() as u64);
    }
    total
}

/// Returns the average severity weight in basis points.
///
/// Returns `None` for an empty sequence.
#[must_use]
pub fn average_weight_bps(severities: &[Severity]) -> Option<u32> {
    if severities.is_empty() {
        return None;
    }

    Some((total_weight_bps(severities) / severities.len() as u64) as u32)
}
