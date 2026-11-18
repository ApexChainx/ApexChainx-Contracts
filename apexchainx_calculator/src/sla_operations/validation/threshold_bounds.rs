//! Validation helpers for SLA threshold bound constraints.

/// Errors produced while validating threshold bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThresholdBoundsError {
    /// The lower bound is greater than the upper bound.
    NonAscending,
    /// The lower and upper bounds are equal.
    EmptySpan,
    /// A supplied value is below the lower bound.
    BelowLowerBound,
    /// A supplied value is above the upper bound.
    AboveUpperBound,
}

/// A validated inclusive threshold range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThresholdBounds {
    /// Inclusive lower bound.
    pub lower: u64,
    /// Inclusive upper bound.
    pub upper: u64,
}

impl ThresholdBounds {
    /// Builds a validated inclusive range.
    pub const fn new(lower: u64, upper: u64) -> Result<Self, ThresholdBoundsError> {
        match validate_threshold_bounds(lower, upper) {
            Ok(()) => Ok(Self { lower, upper }),
            Err(error) => Err(error),
        }
    }

    /// Returns whether `value` falls inside the inclusive range.
    #[must_use]
    pub const fn contains(&self, value: u64) -> bool {
        value >= self.lower && value <= self.upper
    }
}

/// Validates that bounds form a non-empty ascending inclusive range.
#[must_use]
pub const fn validate_threshold_bounds(
    lower: u64,
    upper: u64,
) -> Result<(), ThresholdBoundsError> {
    if lower > upper {
        Err(ThresholdBoundsError::NonAscending)
    } else if lower == upper {
        Err(ThresholdBoundsError::EmptySpan)
    } else {
        Ok(())
    }
}

/// Validates that `value` lies within already-validated `bounds`.
#[must_use]
pub const fn validate_threshold_value(
    bounds: ThresholdBounds,
    value: u64,
) -> Result<(), ThresholdBoundsError> {
    if value < bounds.lower {
        Err(ThresholdBoundsError::BelowLowerBound)
    } else if value > bounds.upper {
        Err(ThresholdBoundsError::AboveUpperBound)
    } else {
        Ok(())
    }
}
