//! Deterministic MTTR classification helpers for the SLA operations workspace.

/// Classification band for a mean-time-to-repair measurement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MttrBand {
    /// The measurement is at or below the warning threshold.
    Healthy,
    /// The measurement is above the warning threshold but within the breach threshold.
    Warning,
    /// The measurement is above the breach threshold.
    Breached,
}

/// Result of comparing an MTTR value with warning and breach thresholds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MttrClassification {
    /// Selected classification band.
    pub band: MttrBand,
    /// Whether the warning threshold was exceeded.
    pub exceeded_warning: bool,
    /// Whether the breach threshold was exceeded.
    pub exceeded_breach: bool,
    /// Amount by which the warning threshold was exceeded, or zero.
    pub warning_margin: u64,
    /// Amount by which the breach threshold was exceeded, or zero.
    pub breach_margin: u64,
}

/// Classifies `mttr_secs` against ascending warning and breach thresholds.
///
/// Returns `None` when `warning_secs` is greater than `breach_secs` or when
/// `breach_secs` is zero, because no meaningful classification can be derived.
#[must_use]
pub const fn classify_mttr(
    mttr_secs: u64,
    warning_secs: u64,
    breach_secs: u64,
) -> Option<MttrClassification> {
    if warning_secs > breach_secs || breach_secs == 0 {
        return None;
    }

    let exceeded_warning = mttr_secs > warning_secs;
    let exceeded_breach = mttr_secs > breach_secs;

    let band = if exceeded_breach {
        MttrBand::Breached
    } else if exceeded_warning {
        MttrBand::Warning
    } else {
        MttrBand::Healthy
    };

    Some(MttrClassification {
        band,
        exceeded_warning,
        exceeded_breach,
        warning_margin: mttr_secs.saturating_sub(warning_secs),
        breach_margin: mttr_secs.saturating_sub(breach_secs),
    })
}

/// Returns `true` when `mttr_secs` is at or below `threshold_secs`.
#[must_use]
pub const fn meets_mttr_threshold(mttr_secs: u64, threshold_secs: u64) -> bool {
    mttr_secs <= threshold_secs
}

/// Returns a stable, lowercase label for `band`.
#[must_use]
pub const fn band_label(band: MttrBand) -> &'static str {
    match band {
        MttrBand::Healthy => "healthy",
        MttrBand::Warning => "warning",
        MttrBand::Breached => "breached",
    }
}
