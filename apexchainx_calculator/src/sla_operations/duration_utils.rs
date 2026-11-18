//! Duration normalization and boundary helpers for SLA outage records.

/// Errors produced while normalizing outage durations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DurationError {
    /// The end timestamp precedes the start timestamp.
    EndBeforeStart,
}

/// An inclusive-exclusive outage interval in seconds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DurationWindow {
    /// Interval start in seconds.
    pub start: u64,
    /// Interval end in seconds.
    pub end: u64,
}

impl DurationWindow {
    /// Builds a normalized duration window.
    pub const fn new(start: u64, end: u64) -> Result<Self, DurationError> {
        if end < start {
            Err(DurationError::EndBeforeStart)
        } else {
            Ok(Self { start, end })
        }
    }

    /// Returns the interval length in seconds.
    #[must_use]
    pub const fn duration(&self) -> u64 {
        self.end - self.start
    }

    /// Returns whether this interval overlaps `other`.
    #[must_use]
    pub const fn overlaps(&self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Returns `end - start`, or `None` when the interval is inverted.
#[must_use]
pub const fn normalize_duration(start: u64, end: u64) -> Option<u64> {
    end.checked_sub(start)
}

/// Caps `duration_secs` at `maximum_secs`.
#[must_use]
pub const fn clamp_duration(duration_secs: u64, maximum_secs: u64) -> u64 {
    if duration_secs > maximum_secs {
        maximum_secs
    } else {
        duration_secs
    }
}

/// Sums durations without overflowing, saturating at [`u64::MAX`].
#[must_use]
pub fn total_duration(durations: &[u64]) -> u64 {
    let mut total = 0_u64;
    for &duration in durations {
        total = total.saturating_add(duration);
    }
    total
}
