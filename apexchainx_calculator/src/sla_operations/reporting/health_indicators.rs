//! Operational health indicators derived from SLA metrics.
//!
//! Inputs are plain counters. If `violations` exceeds `total` the value is
//! clamped to `total` so a bad counter can never produce a negative compliance
//! rate. Rates are basis points, truncated toward zero.

/// Overall health level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthLevel {
    /// No calculations in the period.
    NoData,
    /// Compliance of at least [`HEALTHY_MIN_BPS`].
    Healthy,
    /// Compliance of at least [`WARNING_MIN_BPS`] but below healthy.
    Warning,
    /// Compliance below [`WARNING_MIN_BPS`].
    Critical,
}

/// Minimum compliance (bps) for [`HealthLevel::Healthy`] (95%).
pub const HEALTHY_MIN_BPS: u32 = 9_500;
/// Minimum compliance (bps) for [`HealthLevel::Warning`] (90%).
pub const WARNING_MIN_BPS: u32 = 9_000;

/// Raw counters for one period.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlaMetrics {
    pub total: u32,
    pub violations: u32,
    /// Sum of MTTR across all calculations, in minutes.
    pub total_mttr_minutes: u64,
}

/// Derived indicators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthIndicators {
    pub level: HealthLevel,
    /// `(total - violations) * 10_000 / total`; `0` when there is no data.
    pub compliance_bps: u32,
    /// `violations * 10_000 / total`; `0` when there is no data.
    pub violation_bps: u32,
    /// Mean MTTR in whole minutes (truncated); `0` when there is no data.
    pub avg_mttr_minutes: u32,
}

/// Derive health indicators from `metrics`.
pub fn derive(metrics: SlaMetrics) -> HealthIndicators {
    if metrics.total == 0 {
        return HealthIndicators {
            level: HealthLevel::NoData,
            compliance_bps: 0,
            violation_bps: 0,
            avg_mttr_minutes: 0,
        };
    }
    let total = metrics.total as u64;
    let violations = metrics.violations.min(metrics.total) as u64;
    let violation_bps = (violations * 10_000 / total) as u32;
    let compliance_bps = ((total - violations) * 10_000 / total) as u32;
    let avg = metrics.total_mttr_minutes / total;
    let level = if compliance_bps >= HEALTHY_MIN_BPS {
        HealthLevel::Healthy
    } else if compliance_bps >= WARNING_MIN_BPS {
        HealthLevel::Warning
    } else {
        HealthLevel::Critical
    };
    HealthIndicators {
        level,
        compliance_bps,
        violation_bps,
        avg_mttr_minutes: avg.min(u32::MAX as u64) as u32,
    }
}

#[cfg(test)]
mod health_tests {
    use super::*;

    fn m(total: u32, violations: u32, mttr: u64) -> SlaMetrics {
        SlaMetrics { total, violations, total_mttr_minutes: mttr }
    }

    #[test]
    fn levels_follow_the_thresholds() {
        assert_eq!(derive(m(100, 5, 0)).level, HealthLevel::Healthy);
        assert_eq!(derive(m(100, 6, 0)).level, HealthLevel::Warning);
        assert_eq!(derive(m(100, 10, 0)).level, HealthLevel::Warning);
        assert_eq!(derive(m(100, 11, 0)).level, HealthLevel::Critical);
        assert_eq!(derive(m(0, 0, 0)).level, HealthLevel::NoData);
    }

    #[test]
    fn violations_are_clamped_and_mttr_averaged() {
        let d = derive(m(4, 99, 10));
        assert_eq!((d.compliance_bps, d.violation_bps), (0, 10_000));
        assert_eq!(d.avg_mttr_minutes, 2);
    }
}
