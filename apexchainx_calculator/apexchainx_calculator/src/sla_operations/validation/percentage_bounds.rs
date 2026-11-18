
#![no_std]

//! Percentage and Ratio Validation Utilities
//!
//! Provides pure, deterministic validation and conversion helpers
//! for SLA percentage, ratio, threshold, and allocation operations.
//!
//! Rules:
//! - Percentages must be between 0 and 100 inclusive.
//! - Ratios must be finite and non-negative.
//! - Bounded values must satisfy minimum <= value <= maximum.
//! - Percentage allocations must not exceed 100%.
//! - Exact percentage allocations must total 100%, within tolerance.
//! - NaN and infinity are always rejected.
//!
//! This module does not access Soroban storage, contract events,
//! or contract state.

pub const MIN_PERCENTAGE: f64 = 0.0;
pub const MAX_PERCENTAGE: f64 = 100.0;
pub const MAX_BASIS_POINTS: u32 = 10_000;
pub const PERCENTAGE_EPSILON: f64 = 1e-9;

/// Errors produced by validation and conversion operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationError {
    NotANumber,
    InfiniteValue,
    BelowMinimum,
    AboveMaximum,
    InvalidBounds,
    PercentageSumExceeded,
    MustBePositive,
    EmptyValues,
}

/// Standard validation result.
pub type ValidationResult = Result<(), ValidationError>;

/// Checks whether a value is finite.
pub fn is_finite(value: f64) -> bool {
    !value.is_nan() && !value.is_infinite()
}

/// Validates a percentage in the inclusive range 0..=100.
pub fn validate_percentage(value: f64) -> ValidationResult {
    if value.is_nan() {
        return Err(ValidationError::NotANumber);
    }

    if value.is_infinite() {
        return Err(ValidationError::InfiniteValue);
    }

    if value < MIN_PERCENTAGE {
        return Err(ValidationError::BelowMinimum);
    }

    if value > MAX_PERCENTAGE {
        return Err(ValidationError::AboveMaximum);
    }

    Ok(())
}

/// Validates a percentage greater than zero.
pub fn validate_positive_percentage(value: f64) -> ValidationResult {
    validate_percentage(value)?;

    if value == 0.0 {
        return Err(ValidationError::MustBePositive);
    }

    Ok(())
}

/// Validates a percentage against inclusive custom boundaries.
pub fn validate_bounded_percentage(
    value: f64,
    minimum: f64,
    maximum: f64,
) -> ValidationResult {
    validate_percentage(value)?;
    validate_percentage(minimum)?;
    validate_percentage(maximum)?;

    if minimum > maximum {
        return Err(ValidationError::InvalidBounds);
    }

    if value < minimum {
        return Err(ValidationError::BelowMinimum);
    }

    if value > maximum {
        return Err(ValidationError::AboveMaximum);
    }

    Ok(())
}

/// Validates a non-negative finite ratio.
///
/// Ratios are not percentages, so values greater than 1.0 are allowed.
pub fn validate_ratio(value: f64) -> ValidationResult {
    if value.is_nan() {
        return Err(ValidationError::NotANumber);
    }

    if value.is_infinite() {
        return Err(ValidationError::InfiniteValue);
    }

    if value < 0.0 {
        return Err(ValidationError::BelowMinimum);
    }

    Ok(())
}

/// Validates a strictly positive ratio.
pub fn validate_positive_ratio(value: f64) -> ValidationResult {
    validate_ratio(value)?;

    if value == 0.0 {
        return Err(ValidationError::MustBePositive);
    }

    Ok(())
}

/// Validates a ratio against inclusive custom boundaries.
pub fn validate_bounded_ratio(
    value: f64,
    minimum: f64,
    maximum: f64,
) -> ValidationResult {
    validate_ratio(value)?;
    validate_ratio(minimum)?;
    validate_ratio(maximum)?;

    if minimum > maximum {
        return Err(ValidationError::InvalidBounds);
    }

    if value < minimum {
        return Err(ValidationError::BelowMinimum);
    }

    if value > maximum {
        return Err(ValidationError::AboveMaximum);
    }

    Ok(())
}

/// Validates that a collection of percentages does not exceed 100%.
pub fn validate_percentage_sum(values: &[f64]) -> ValidationResult {
    if values.is_empty() {
        return Err(ValidationError::EmptyValues);
    }

    let mut total = 0.0;

    for value in values {
        validate_percentage(*value)?;

        total += *value;

        if !is_finite(total) {
            return Err(ValidationError::PercentageSumExceeded);
        }
    }

    if total > MAX_PERCENTAGE + PERCENTAGE_EPSILON {
        return Err(ValidationError::PercentageSumExceeded);
    }

    Ok(())
}

/// Validates that percentages total exactly 100%, within tolerance.
pub fn validate_percentage_total(values: &[f64]) -> ValidationResult {
    validate_percentage_sum(values)?;

    let mut total = 0.0;

    for value in values {
        total += *value;
    }

    if (total - MAX_PERCENTAGE).abs() > PERCENTAGE_EPSILON {
        return Err(ValidationError::BelowMinimum);
    }

    Ok(())
}

/// Validates an SLA threshold greater than zero and at most 100%.
pub fn validate_sla_threshold(threshold: f64) -> ValidationResult {
    validate_positive_percentage(threshold)
}

/// Validates a non-negative percentage adjustment with a maximum.
pub fn validate_percentage_adjustment(
    adjustment: f64,
    maximum_adjustment: f64,
) -> ValidationResult {
    validate_percentage(adjustment)?;
    validate_percentage(maximum_adjustment)?;

    if adjustment > maximum_adjustment {
        return Err(ValidationError::AboveMaximum);
    }

    Ok(())
}

/// Converts a percentage to a normalized ratio.
///
/// 50% becomes 0.5.
pub fn percentage_to_ratio(
    percentage: f64,
) -> Result<f64, ValidationError> {
    validate_percentage(percentage)?;

    Ok(percentage / MAX_PERCENTAGE)
}

/// Converts a normalized ratio to a percentage.
///
/// Only ratios between 0 and 1 are accepted.
pub fn ratio_to_percentage(
    ratio: f64,
) -> Result<f64, ValidationError> {
    validate_bounded_ratio(ratio, 0.0, 1.0)?;

    Ok(ratio * MAX_PERCENTAGE)
}

/// Converts a percentage into basis points.
///
/// 1% = 100 basis points.
/// 100% = 10,000 basis points.
pub fn percentage_to_basis_points(
    percentage: f64,
) -> Result<u32, ValidationError> {
    validate_percentage(percentage)?;

    Ok((percentage * 100.0 + 0.5) as u32)
}

/// Converts basis points into a percentage.
pub fn basis_points_to_percentage(
    basis_points: u32,
) -> Result<f64, ValidationError> {
    if basis_points > MAX_BASIS_POINTS {
        return Err(ValidationError::AboveMaximum);
    }

    Ok(basis_points as f64 / 100.0)
}

/// Converts a normalized ratio into basis points.
pub fn ratio_to_basis_points(
    ratio: f64,
) -> Result<u32, ValidationError> {
    validate_bounded_ratio(ratio, 0.0, 1.0)?;

    Ok((ratio * MAX_BASIS_POINTS as f64 + 0.5) as u32)
}

/// Converts basis points into a normalized ratio.
pub fn basis_points_to_ratio(
    basis_points: u32,
) -> Result<f64, ValidationError> {
    if basis_points > MAX_BASIS_POINTS {
        return Err(ValidationError::AboveMaximum);
    }

    Ok(basis_points as f64 / MAX_BASIS_POINTS as f64)
}

/// Boolean convenience helper for bounded percentages.
pub fn is_percentage_in_bounds(
    value: f64,
    minimum: f64,
    maximum: f64,
) -> bool {
    validate_bounded_percentage(value, minimum, maximum).is_ok()
}

/// Boolean convenience helper for bounded ratios.
pub fn is_ratio_in_bounds(
    value: f64,
    minimum: f64,
    maximum: f64,
) -> bool {
    validate_bounded_ratio(value, minimum, maximum).is_ok()
}

// ================================================================
// UNIT TESTS
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_percentages() {
        assert_eq!(validate_percentage(0.0), Ok(()));
        assert_eq!(validate_percentage(25.0), Ok(()));
        assert_eq!(validate_percentage(50.0), Ok(()));
        assert_eq!(validate_percentage(100.0), Ok(()));
    }

    #[test]
    fn test_invalid_percentages() {
        assert_eq!(
            validate_percentage(-1.0),
            Err(ValidationError::BelowMinimum)
        );

        assert_eq!(
            validate_percentage(101.0),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_nan_and_infinity() {
        assert_eq!(
            validate_percentage(f64::NAN),
            Err(ValidationError::NotANumber)
        );

        assert_eq!(
            validate_percentage(f64::INFINITY),
            Err(ValidationError::InfiniteValue)
        );

        assert_eq!(
            validate_ratio(f64::NEG_INFINITY),
            Err(ValidationError::InfiniteValue)
        );
    }

    #[test]
    fn test_positive_percentage() {
        assert_eq!(validate_positive_percentage(1.0), Ok(()));

        assert_eq!(
            validate_positive_percentage(0.0),
            Err(ValidationError::MustBePositive)
        );
    }

    #[test]
    fn test_bounded_percentage() {
        assert_eq!(
            validate_bounded_percentage(50.0, 10.0, 90.0),
            Ok(())
        );

        assert_eq!(
            validate_bounded_percentage(5.0, 10.0, 90.0),
            Err(ValidationError::BelowMinimum)
        );

        assert_eq!(
            validate_bounded_percentage(95.0, 10.0, 90.0),
            Err(ValidationError::AboveMaximum)
        );

        assert_eq!(
            validate_bounded_percentage(50.0, 90.0, 10.0),
            Err(ValidationError::InvalidBounds)
        );
    }

    #[test]
    fn test_valid_ratios() {
        assert_eq!(validate_ratio(0.0), Ok(()));
        assert_eq!(validate_ratio(0.5), Ok(()));
        assert_eq!(validate_ratio(1.0), Ok(()));
        assert_eq!(validate_ratio(2.5), Ok(()));
    }

    #[test]
    fn test_invalid_ratios() {
        assert_eq!(
            validate_ratio(-0.5),
            Err(ValidationError::BelowMinimum)
        );
    }

    #[test]
    fn test_positive_ratio() {
        assert_eq!(validate_positive_ratio(0.5), Ok(()));

        assert_eq!(
            validate_positive_ratio(0.0),
            Err(ValidationError::MustBePositive)
        );
    }

    #[test]
    fn test_bounded_ratio() {
        assert_eq!(
            validate_bounded_ratio(0.5, 0.0, 1.0),
            Ok(())
        );

        assert_eq!(
            validate_bounded_ratio(2.0, 0.0, 1.0),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_percentage_sum() {
        assert_eq!(
            validate_percentage_sum(&[20.0, 30.0, 50.0]),
            Ok(())
        );

        assert_eq!(
            validate_percentage_sum(&[60.0, 50.0]),
            Err(ValidationError::PercentageSumExceeded)
        );
    }

    #[test]
    fn test_empty_percentage_sum() {
        assert_eq!(
            validate_percentage_sum(&[]),
            Err(ValidationError::EmptyValues)
        );
    }

    #[test]
    fn test_percentage_total() {
        assert_eq!(
            validate_percentage_total(&[25.0, 25.0, 50.0]),
            Ok(())
        );

        assert_eq!(
            validate_percentage_total(&[25.0, 25.0]),
            Err(ValidationError::BelowMinimum)
        );
    }

    #[test]
    fn test_percentage_sum_tolerance() {
        assert_eq!(
            validate_percentage_sum(&[
                33.333333333,
                33.333333333,
                33.333333334,
            ]),
            Ok(())
        );
    }

    #[test]
    fn test_sla_threshold() {
        assert_eq!(validate_sla_threshold(99.9), Ok(()));

        assert_eq!(
            validate_sla_threshold(0.0),
            Err(ValidationError::MustBePositive)
        );

        assert_eq!(
            validate_sla_threshold(101.0),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_percentage_adjustment() {
        assert_eq!(
            validate_percentage_adjustment(10.0, 20.0),
            Ok(())
        );

        assert_eq!(
            validate_percentage_adjustment(30.0, 20.0),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_percentage_to_ratio() {
        assert_eq!(percentage_to_ratio(0.0), Ok(0.0));
        assert_eq!(percentage_to_ratio(50.0), Ok(0.5));
        assert_eq!(percentage_to_ratio(100.0), Ok(1.0));

        assert_eq!(
            percentage_to_ratio(101.0),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_ratio_to_percentage() {
        assert_eq!(ratio_to_percentage(0.0), Ok(0.0));
        assert_eq!(ratio_to_percentage(0.5), Ok(50.0));
        assert_eq!(ratio_to_percentage(1.0), Ok(100.0));

        assert_eq!(
            ratio_to_percentage(1.5),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_percentage_to_basis_points() {
        assert_eq!(percentage_to_basis_points(0.0), Ok(0));
        assert_eq!(percentage_to_basis_points(1.0), Ok(100));
        assert_eq!(percentage_to_basis_points(50.0), Ok(5000));
        assert_eq!(percentage_to_basis_points(100.0), Ok(10000));
    }

    #[test]
    fn test_basis_points_to_percentage() {
        assert_eq!(basis_points_to_percentage(0), Ok(0.0));
        assert_eq!(basis_points_to_percentage(100), Ok(1.0));
        assert_eq!(basis_points_to_percentage(5000), Ok(50.0));
        assert_eq!(basis_points_to_percentage(10000), Ok(100.0));

        assert_eq!(
            basis_points_to_percentage(10001),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_ratio_to_basis_points() {
        assert_eq!(ratio_to_basis_points(0.0), Ok(0));
        assert_eq!(ratio_to_basis_points(0.5), Ok(5000));
        assert_eq!(ratio_to_basis_points(1.0), Ok(10000));
    }

    #[test]
    fn test_basis_points_to_ratio() {
        assert_eq!(basis_points_to_ratio(0), Ok(0.0));
        assert_eq!(basis_points_to_ratio(5000), Ok(0.5));
        assert_eq!(basis_points_to_ratio(10000), Ok(1.0));

        assert_eq!(
            basis_points_to_ratio(10001),
            Err(ValidationError::AboveMaximum)
        );
    }

    #[test]
    fn test_boolean_helpers() {
        assert!(is_percentage_in_bounds(50.0, 0.0, 100.0));
        assert!(!is_percentage_in_bounds(110.0, 0.0, 100.0));

        assert!(is_ratio_in_bounds(0.5, 0.0, 1.0));
        assert!(!is_ratio_in_bounds(2.0, 0.0, 1.0));
    }

    #[test]
    fn test_deterministic_conversion() {
        let first = percentage_to_ratio(75.0).unwrap();
        let second = percentage_to_ratio(75.0).unwrap();

        assert_eq!(first, second);
    }
}
