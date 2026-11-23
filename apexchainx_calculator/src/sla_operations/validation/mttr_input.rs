//! Validation helpers for MTTR and duration inputs.

/// Errors produced while validating MTTR inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MttrInputError {
    /// The minimum bound is greater than the maximum bound.
    InvertedRange,
    /// The minimum and maximum bounds are equal.
    EmptyRange,
    /// The duration is below the allowed minimum.
    BelowMinimum,
    /// The duration exceeds the allowed maximum.
    AboveMaximum,
    /// The duration is zero where a positive value is required.
    ZeroDuration,
}

/// An inclusive MTTR range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MttrInputBounds {
    /// Inclusive minimum duration in seconds.
    pub minimum: u64,
    /// Inclusive maximum duration in seconds.
    pub maximum: u64,
}

impl MttrInputBounds {
    /// Builds a validated MTTR range.
    pub const fn new(minimum: u64, maximum: u64) -> Result<Self, MttrInputError> {
        if minimum > maximum {
            Err(MttrInputError::InvertedRange)
        } else if minimum == maximum {
            Err(MttrInputError::EmptyRange)
        } else {
            Ok(Self { minimum, maximum })
        }
    }
}

/// Rejects zero MTTR durations.
#[must_use]
pub const fn validate_positive_mttr(duration_secs: u64) -> Result<(), MttrInputError> {
    if duration_secs == 0 {
        Err(MttrInputError::ZeroDuration)
    } else {
        Ok(())
    }
}

/// Validates an MTTR duration against inclusive bounds.
#[must_use]
pub const fn validate_mttr_range(
    duration_secs: u64,
    minimum_secs: u64,
    maximum_secs: u64,
) -> Result<(), MttrInputError> {
    match MttrInputBounds::new(minimum_secs, maximum_secs) {
        Ok(bounds) => validate_mttr_bounds(duration_secs, bounds),
        Err(error) => Err(error),
    }
}

/// Validates an MTTR duration against already-validated bounds.
#[must_use]
pub const fn validate_mttr_bounds(
    duration_secs: u64,
    bounds: MttrInputBounds,
) -> Result<(), MttrInputError> {
    if duration_secs < bounds.minimum {
        Err(MttrInputError::BelowMinimum)
    } else if duration_secs > bounds.maximum {
        Err(MttrInputError::AboveMaximum)
    } else {
        Ok(())
    }
}

/// Returns whether an MTTR duration is positive and within inclusive bounds.
#[must_use]
pub const fn is_valid_mttr(
    duration_secs: u64,
    minimum_secs: u64,
    maximum_secs: u64,
) -> bool {
    if duration_secs == 0 {
        return false;
    }

    duration_secs >= minimum_secs && duration_secs <= maximum_secs && minimum_secs < maximum_secs
}
