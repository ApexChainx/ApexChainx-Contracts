//! Deterministic anomaly-scenario fixtures for the SLA operations workspace.
//!
//! Each scenario encodes one intended anomalous SLA metric pattern (no traffic,
//! a complete violation, an extreme MTTR, a counter regression, a severity
//! concentration, or a zero threshold), plus one non-anomalous baseline.
//! Predicates are evaluated on the fixture data only; nothing here reads
//! contract state, storage, or events.

/// Pattern a scenario is meant to represent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnomalyPattern {
    /// Non-anomalous reference scenario.
    Baseline,
    /// No SLA calculations were recorded in the period.
    NoTraffic,
    /// Every calculation in the period was a violation.
    CompleteViolation,
    /// A representative MTTR at the maximum representable value.
    ExtremeMttr,
    /// A counter reading moved backwards between snapshots.
    CounterRegression,
    /// A strict majority of calculations were critical severity.
    SeverityConcentration,
    /// A threshold of zero minutes, which cannot describe a healthy SLA.
    ZeroThreshold,
}

impl AnomalyPattern {
    /// Stable, lowercase label for the pattern.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::NoTraffic => "no_traffic",
            Self::CompleteViolation => "complete_violation",
            Self::ExtremeMttr => "extreme_mttr",
            Self::CounterRegression => "counter_regression",
            Self::SeverityConcentration => "severity_concentration",
            Self::ZeroThreshold => "zero_threshold",
        }
    }
}

/// One anomaly scenario.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnomalyScenario {
    /// Stable scenario name.
    pub name: &'static str,
    /// Pattern this scenario is intended to exercise.
    pub pattern: AnomalyPattern,
    /// Number of SLA calculations in the period.
    pub total: u32,
    /// Number of calculations that violated their SLA.
    pub violations: u32,
    /// Number of calculations at critical severity.
    pub critical: u32,
    /// Representative MTTR in minutes.
    pub mttr_minutes: u32,
    /// Applicable threshold in minutes.
    pub threshold_minutes: u32,
    /// Counter reading before the period.
    pub previous_counter: u64,
    /// Counter reading after the period.
    pub current_counter: u64,
    /// Whether the scenario is expected to be flagged as anomalous.
    pub expect_anomalous: bool,
}

impl AnomalyScenario {
    /// Number of calculations that met their SLA, saturating at zero.
    #[must_use]
    pub const fn met(&self) -> u32 {
        self.total.saturating_sub(self.violations)
    }

    /// Whether no calculations were recorded.
    #[must_use]
    pub const fn has_no_traffic(&self) -> bool {
        self.total == 0
    }

    /// Whether at least one calculation ran and all of them violated.
    #[must_use]
    pub const fn has_complete_violation(&self) -> bool {
        self.total > 0 && self.violations >= self.total
    }

    /// Whether the representative MTTR is at its maximum representable value.
    #[must_use]
    pub const fn has_extreme_mttr(&self) -> bool {
        self.mttr_minutes == u32::MAX
    }

    /// Whether the counter moved backwards between snapshots.
    #[must_use]
    pub const fn has_counter_regression(&self) -> bool {
        self.current_counter < self.previous_counter
    }

    /// Whether critical calculations form a strict majority of the total.
    #[must_use]
    pub const fn has_severity_concentration(&self) -> bool {
        self.total > 0 && self.critical > self.total / 2
    }

    /// Whether the threshold is zero.
    #[must_use]
    pub const fn has_zero_threshold(&self) -> bool {
        self.threshold_minutes == 0
    }

    /// Whether any anomaly predicate matches this scenario.
    #[must_use]
    pub const fn is_anomalous(&self) -> bool {
        self.has_no_traffic()
            || self.has_complete_violation()
            || self.has_extreme_mttr()
            || self.has_counter_regression()
            || self.has_severity_concentration()
            || self.has_zero_threshold()
    }

    /// Number of anomaly predicates that match this scenario.
    #[must_use]
    pub const fn detected_pattern_count(&self) -> u8 {
        self.has_no_traffic() as u8
            + self.has_complete_violation() as u8
            + self.has_extreme_mttr() as u8
            + self.has_counter_regression() as u8
            + self.has_severity_concentration() as u8
            + self.has_zero_threshold() as u8
    }
}

const fn scenario(
    name: &'static str,
    pattern: AnomalyPattern,
    counts: (u32, u32, u32),
    mttr_and_threshold: (u32, u32),
    counters: (u64, u64),
    expect_anomalous: bool,
) -> AnomalyScenario {
    AnomalyScenario {
        name,
        pattern,
        total: counts.0,
        violations: counts.1,
        critical: counts.2,
        mttr_minutes: mttr_and_threshold.0,
        threshold_minutes: mttr_and_threshold.1,
        previous_counter: counters.0,
        current_counter: counters.1,
        expect_anomalous,
    }
}

/// All documented anomaly fixtures, baseline first.
#[must_use]
pub fn scenarios() -> alloc::vec::Vec<AnomalyScenario> {
    alloc::vec![
        scenario(
            "baseline",
            AnomalyPattern::Baseline,
            (10, 1, 2),
            (12, 15),
            (100, 120),
            false,
        ),
        scenario(
            "no_traffic",
            AnomalyPattern::NoTraffic,
            (0, 0, 0),
            (0, 15),
            (5, 5),
            true,
        ),
        scenario(
            "complete_violation",
            AnomalyPattern::CompleteViolation,
            (6, 6, 1),
            (90, 30),
            (6, 6),
            true,
        ),
        scenario(
            "extreme_mttr",
            AnomalyPattern::ExtremeMttr,
            (4, 2, 1),
            (u32::MAX, 120),
            (4, 4),
            true,
        ),
        scenario(
            "counter_regression",
            AnomalyPattern::CounterRegression,
            (8, 2, 2),
            (20, 30),
            (100, 99),
            true,
        ),
        scenario(
            "severity_concentration",
            AnomalyPattern::SeverityConcentration,
            (5, 1, 4),
            (10, 15),
            (5, 5),
            true,
        ),
        scenario(
            "zero_threshold",
            AnomalyPattern::ZeroThreshold,
            (3, 1, 1),
            (5, 0),
            (3, 3),
            true,
        ),
    ]
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn fixtures_are_deterministic_and_flags_match() {
        assert_eq!(scenarios(), scenarios());

        for s in scenarios() {
            assert_eq!(s.is_anomalous(), s.expect_anomalous, "{}", s.name);
            let expected_count = if s.pattern == AnomalyPattern::Baseline {
                0
            } else {
                1
            };
            assert_eq!(s.detected_pattern_count(), expected_count, "{}", s.name);
        }
    }
}
