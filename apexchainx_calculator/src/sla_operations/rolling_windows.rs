//! Rolling-window aggregate helpers for ordered SLA metric samples.

/// Aggregate values for one bounded window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowAggregate {
    /// Sum of samples in the window.
    pub sum: u64,
    /// Number of samples in the window.
    pub count: usize,
    /// Integer average of the window, or `None` for an empty window.
    pub average: Option<u64>,
}

fn window_bounds(samples: &[u64], start: usize, count: usize) -> Option<(usize, usize)> {
    if count == 0 {
        return None;
    }

    let end = start.checked_add(count)?;
    if end > samples.len() {
        return None;
    }

    Some((start, end))
}

/// Sums `count` samples beginning at `start`, saturating at [`u64::MAX`].
#[must_use]
pub fn window_sum(samples: &[u64], start: usize, count: usize) -> Option<u64> {
    let (start, end) = window_bounds(samples, start, count)?;
    let mut total = 0_u64;

    for &sample in &samples[start..end] {
        total = total.saturating_add(sample);
    }

    Some(total)
}

/// Returns the integer average of a bounded window.
#[must_use]
pub fn window_average(samples: &[u64], start: usize, count: usize) -> Option<u64> {
    let sum = window_sum(samples, start, count)?;
    Some(sum / count as u64)
}

/// Returns the maximum sample in a bounded window.
#[must_use]
pub fn window_max(samples: &[u64], start: usize, count: usize) -> Option<u64> {
    let (start, end) = window_bounds(samples, start, count)?;
    let mut maximum = samples[start];

    for &sample in &samples[start + 1..end] {
        if sample > maximum {
            maximum = sample;
        }
    }

    Some(maximum)
}

/// Returns the minimum sample in a bounded window.
#[must_use]
pub fn window_min(samples: &[u64], start: usize, count: usize) -> Option<u64> {
    let (start, end) = window_bounds(samples, start, count)?;
    let mut minimum = samples[start];

    for &sample in &samples[start + 1..end] {
        if sample < minimum {
            minimum = sample;
        }
    }

    Some(minimum)
}

/// Builds a complete aggregate for a bounded window.
#[must_use]
pub fn aggregate_window(samples: &[u64], start: usize, count: usize) -> Option<WindowAggregate> {
    let sum = window_sum(samples, start, count)?;

    Some(WindowAggregate {
        sum,
        count,
        average: Some(sum / count as u64),
    })
}
