//! Deterministic counter delta helpers for the SLA operations workspace.

/// Direction of a counter change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterMovement {
    /// The current value is greater than the previous value.
    Increased,
    /// The current value is less than the previous value.
    Decreased,
    /// The current and previous values are equal.
    Unchanged,
}

/// A single counter comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterDelta {
    /// Direction of the change.
    pub movement: CounterMovement,
    /// Absolute change between the two values.
    pub magnitude: u64,
    /// Signed difference between current and previous values.
    pub net: i128,
}

/// Aggregate counters produced from a sequence of counter comparisons.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CounterSummary {
    /// Number of counters that increased.
    pub increases: u64,
    /// Number of counters that decreased.
    pub decreases: u64,
    /// Number of counters that did not change.
    pub unchanged: u64,
    /// Sum of all increases, saturated at [`u64::MAX`].
    pub growth_total: u64,
    /// Sum of all decreases, saturated at [`u64::MAX`].
    pub decline_total: u64,
}

/// Compares two counter readings.
#[must_use]
pub const fn counter_delta(current: u64, previous: u64) -> CounterDelta {
    let movement = if current > previous {
        CounterMovement::Increased
    } else if current < previous {
        CounterMovement::Decreased
    } else {
        CounterMovement::Unchanged
    };

    let magnitude = current.abs_diff(previous);

    CounterDelta {
        movement,
        magnitude,
        net: current as i128 - previous as i128,
    }
}

/// Returns the growth from `previous` to `current`, saturating at zero.
#[must_use]
pub const fn saturating_growth(current: u64, previous: u64) -> u64 {
    current.saturating_sub(previous)
}

/// Returns the decline from `previous` to `current`, saturating at zero.
#[must_use]
pub const fn saturating_decline(current: u64, previous: u64) -> u64 {
    previous.saturating_sub(current)
}

/// Summarizes `(previous, current)` counter pairs without retaining storage.
#[must_use]
pub fn summarize_counters(samples: &[(u64, u64)]) -> CounterSummary {
    let mut summary = CounterSummary::default();

    for &(previous, current) in samples {
        match counter_delta(current, previous).movement {
            CounterMovement::Increased => {
                summary.increases = summary.increases.saturating_add(1);
                summary.growth_total = summary
                    .growth_total
                    .saturating_add(saturating_growth(current, previous));
            }
            CounterMovement::Decreased => {
                summary.decreases = summary.decreases.saturating_add(1);
                summary.decline_total = summary
                    .decline_total
                    .saturating_add(saturating_decline(current, previous));
            }
            CounterMovement::Unchanged => {
                summary.unchanged = summary.unchanged.saturating_add(1);
            }
        }
    }

    summary
}
