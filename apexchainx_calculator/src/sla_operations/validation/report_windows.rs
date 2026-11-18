//! Report window validation.
//!
//! A report window is the half-open-in-spirit range `[start, end]` (unix
//! seconds) a report covers, optionally split into fixed-size buckets.
//! Validation is pure and returns the first problem found, checked in the order
//! listed on [`WindowError`].

/// Longest window a report may cover: 366 days.
pub const MAX_WINDOW_SECONDS: u64 = 366 * 24 * 60 * 60;
/// Most buckets a window may be split into.
pub const MAX_BUCKETS: u64 = 1_000;

/// Why a window or bucket size was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowError {
    /// `end < start`.
    InvertedRange,
    /// `end == start` (zero-length window).
    EmptyRange,
    /// Longer than [`MAX_WINDOW_SECONDS`].
    TooLong,
    /// `bucket_seconds == 0`.
    ZeroBucket,
    /// `bucket_seconds` exceeds the window length.
    BucketLargerThanWindow,
    /// The window would need more than [`MAX_BUCKETS`] buckets.
    TooManyBuckets,
}

/// Validate `[start, end]` and return its length in seconds.
pub fn validate_window(start: u64, end: u64) -> Result<u64, WindowError> {
    if end < start {
        return Err(WindowError::InvertedRange);
    }
    if end == start {
        return Err(WindowError::EmptyRange);
    }
    let duration = end - start;
    if duration > MAX_WINDOW_SECONDS {
        return Err(WindowError::TooLong);
    }
    Ok(duration)
}

/// Validate a window plus bucket size and return the number of buckets
/// (`ceil(duration / bucket_seconds)`).
pub fn validate_buckets(start: u64, end: u64, bucket_seconds: u64) -> Result<u64, WindowError> {
    let duration = validate_window(start, end)?;
    if bucket_seconds == 0 {
        return Err(WindowError::ZeroBucket);
    }
    if bucket_seconds > duration {
        return Err(WindowError::BucketLargerThanWindow);
    }
    let buckets = duration.div_ceil(bucket_seconds);
    if buckets > MAX_BUCKETS {
        return Err(WindowError::TooManyBuckets);
    }
    Ok(buckets)
}
