//! Period-over-period metric deltas.
//!
//! All arithmetic is saturating or checked, so extreme inputs never panic.
//! Percentages are expressed in basis points (1 bp = 0.01%) and truncate toward
//! zero; they are `None` when the previous value is zero (no meaningful base).

use alloc::{string::String, vec::Vec};

/// Direction of a change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

/// Change of one named metric between two periods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricDelta {
    pub name: String,
    pub previous: i128,
    pub current: i128,
    /// `current - previous`, saturating.
    pub absolute: i128,
    /// `absolute * 10_000 / previous`, truncated; `None` if `previous == 0`
    /// or the intermediate product overflows.
    pub percent_bps: Option<i128>,
}

impl MetricDelta {
    /// Direction of the change.
    pub fn trend(&self) -> Trend {
        match self.absolute {
            0 => Trend::Flat,
            n if n > 0 => Trend::Up,
            _ => Trend::Down,
        }
    }
}

/// Compute the delta for one metric.
pub fn compute_delta(name: &str, previous: i128, current: i128) -> MetricDelta {
    let absolute = current.saturating_sub(previous);
    let percent_bps = if previous == 0 {
        None
    } else {
        absolute
            .checked_mul(10_000)
            .and_then(|scaled| scaled.checked_div(previous))
    };
    MetricDelta {
        name: String::from(name),
        previous,
        current,
        absolute,
        percent_bps,
    }
}

/// Compute deltas for `(name, previous, current)` triples, preserving order.
pub fn compute_deltas(metrics: &[(&str, i128, i128)]) -> Vec<MetricDelta> {
    metrics
        .iter()
        .map(|(name, previous, current)| compute_delta(name, *previous, *current))
        .collect()
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn computes_absolute_percent_and_trend() {
        let up = compute_delta("m", 200, 250);
        assert_eq!((up.absolute, up.percent_bps, up.trend()), (50, Some(2_500), Trend::Up));
        let down = compute_delta("m", 200, 150);
        assert_eq!((down.absolute, down.percent_bps, down.trend()), (-50, Some(-2_500), Trend::Down));
        assert_eq!(compute_delta("m", 7, 7).trend(), Trend::Flat);
    }

    #[test]
    fn zero_base_and_overflow_have_no_percentage() {
        assert_eq!(compute_delta("m", 0, 10).percent_bps, None);
        assert_eq!(compute_delta("m", 1, i128::MAX).percent_bps, None);
    }
}
