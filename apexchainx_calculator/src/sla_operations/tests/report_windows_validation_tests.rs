//! Tests for `validation::report_windows`.

use crate::sla_operations::validation::report_windows::{
    validate_buckets, validate_window, WindowError, MAX_BUCKETS, MAX_WINDOW_SECONDS,
};

#[test]
fn a_one_second_window_is_valid_and_reports_its_length() {
    assert_eq!(validate_window(100, 101), Ok(1));
}

#[test]
fn inverted_range_is_rejected() {
    assert_eq!(validate_window(100, 99), Err(WindowError::InvertedRange));
    assert_eq!(validate_window(u64::MAX, 0), Err(WindowError::InvertedRange));
}

#[test]
fn zero_length_window_is_rejected_as_empty() {
    assert_eq!(validate_window(100, 100), Err(WindowError::EmptyRange));
    assert_eq!(validate_window(0, 0), Err(WindowError::EmptyRange));
}

#[test]
fn the_maximum_window_is_accepted_and_one_more_second_is_not() {
    assert_eq!(validate_window(0, MAX_WINDOW_SECONDS), Ok(MAX_WINDOW_SECONDS));
    assert_eq!(
        validate_window(0, MAX_WINDOW_SECONDS + 1),
        Err(WindowError::TooLong)
    );
}

#[test]
fn the_window_length_not_the_absolute_time_is_what_is_limited() {
    let start = 1_700_000_000;
    assert_eq!(validate_window(start, start + 3_600), Ok(3_600));
}

#[test]
fn window_errors_take_precedence_over_bucket_errors() {
    assert_eq!(validate_buckets(10, 5, 0), Err(WindowError::InvertedRange));
    assert_eq!(validate_buckets(5, 5, 0), Err(WindowError::EmptyRange));
}

#[test]
fn zero_bucket_size_is_rejected() {
    assert_eq!(validate_buckets(0, 100, 0), Err(WindowError::ZeroBucket));
}

#[test]
fn bucket_larger_than_the_window_is_rejected() {
    assert_eq!(
        validate_buckets(0, 100, 101),
        Err(WindowError::BucketLargerThanWindow)
    );
}

#[test]
fn bucket_equal_to_the_window_gives_a_single_bucket() {
    assert_eq!(validate_buckets(0, 100, 100), Ok(1));
}

#[test]
fn bucket_count_rounds_up() {
    assert_eq!(validate_buckets(0, 100, 30), Ok(4));
    assert_eq!(validate_buckets(0, 100, 50), Ok(2));
    assert_eq!(validate_buckets(0, 100, 1), Ok(100));
}

#[test]
fn the_maximum_bucket_count_is_accepted_and_one_more_is_not() {
    assert_eq!(validate_buckets(0, MAX_BUCKETS, 1), Ok(MAX_BUCKETS));
    assert_eq!(
        validate_buckets(0, MAX_BUCKETS + 1, 1),
        Err(WindowError::TooManyBuckets)
    );
}

#[test]
fn a_rounded_up_bucket_count_can_exceed_the_limit() {
    // 1_001 seconds in 1_000 buckets of 1s is fine for 1_000s, but a window of
    // 1_001s at 1s needs 1_001 buckets.
    assert_eq!(
        validate_buckets(0, 1_001, 1),
        Err(WindowError::TooManyBuckets)
    );
}
