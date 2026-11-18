//! Deterministic golden-vector comparison helpers for the SLA operations workspace.

/// Comparison outcome for a golden vector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoldenVectorResult {
    /// The actual value matches the expected value.
    Match,
    /// The actual value differs from the expected value.
    Mismatch {
        /// Absolute difference between expected and actual values.
        delta: u64,
    },
}

/// A named expected value paired with an observed value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoldenVector {
    /// Stable name used to identify the vector.
    pub name: &'static str,
    /// Expected deterministic value.
    pub expected: u64,
    /// Observed value.
    pub actual: u64,
}

impl GoldenVector {
    /// Builds a golden vector from a name, expected value, and observed value.
    #[must_use]
    pub const fn new(name: &'static str, expected: u64, actual: u64) -> Self {
        Self {
            name,
            expected,
            actual,
        }
    }

    /// Returns the absolute difference between expected and actual values.
    #[must_use]
    pub const fn delta(&self) -> u64 {
        golden_delta(self.expected, self.actual)
    }

    /// Returns whether the observed value matches the expected value.
    #[must_use]
    pub const fn is_match(&self) -> bool {
        self.expected == self.actual
    }

    /// Returns the comparison outcome for this vector.
    #[must_use]
    pub const fn result(&self) -> GoldenVectorResult {
        compare_golden(self.expected, self.actual)
    }
}

/// Returns the absolute difference between two golden-vector values.
#[must_use]
pub const fn golden_delta(expected: u64, actual: u64) -> u64 {
    if actual >= expected {
        actual - expected
    } else {
        expected - actual
    }
}

/// Compares an expected value with an observed value.
#[must_use]
pub const fn compare_golden(expected: u64, actual: u64) -> GoldenVectorResult {
    if expected == actual {
        GoldenVectorResult::Match
    } else {
        GoldenVectorResult::Mismatch {
            delta: golden_delta(expected, actual),
        }
    }
}

/// Returns whether every supplied vector matches its expected value.
#[must_use]
pub fn all_vectors_match(vectors: &[GoldenVector]) -> bool {
    for vector in vectors {
        if !vector.is_match() {
            return false;
        }
    }

    true
}
