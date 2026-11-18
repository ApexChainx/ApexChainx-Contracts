//! Reusable invariant checks for report outputs.
//!
//! Each check returns `Ok(())` when the invariant holds and a specific
//! [`InvariantError`] otherwise, so tests and callers can assert on the exact
//! property that broke.

/// Which invariant failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvariantError {
    /// A basis-point value is above `10_000`.
    BpsOutOfRange,
    /// The parts do not add up to the stated total.
    CountMismatch,
    /// Shares add up to more than `10_000` bps.
    SharesExceedWhole,
    /// `net != rewards - penalties` (or the subtraction overflows).
    NetMismatch,
    /// A sequence is not strictly ascending.
    NotStrictlyAscending,
}

/// A basis-point value must be at most `10_000` (100%).
pub fn check_bps(value: u32) -> Result<(), InvariantError> {
    if value > 10_000 {
        Err(InvariantError::BpsOutOfRange)
    } else {
        Ok(())
    }
}

/// `parts` must sum to exactly `total` (summed in `u64`, so it cannot overflow).
pub fn check_counts_add_up(parts: &[u32], total: u32) -> Result<(), InvariantError> {
    let sum: u64 = parts.iter().map(|p| *p as u64).sum();
    if sum == total as u64 {
        Ok(())
    } else {
        Err(InvariantError::CountMismatch)
    }
}

/// Shares (bps) must not add up to more than `10_000`. Truncated shares may
/// legitimately add up to *less*.
pub fn check_shares_within_whole(shares: &[u32]) -> Result<(), InvariantError> {
    let sum: u64 = shares.iter().map(|s| *s as u64).sum();
    if sum > 10_000 {
        Err(InvariantError::SharesExceedWhole)
    } else {
        Ok(())
    }
}

/// `net` must equal `rewards - penalties` exactly.
pub fn check_net(rewards: i128, penalties: i128, net: i128) -> Result<(), InvariantError> {
    match rewards.checked_sub(penalties) {
        Some(expected) if expected == net => Ok(()),
        _ => Err(InvariantError::NetMismatch),
    }
}

/// `values` must be strictly ascending (no duplicates, no decreases). Empty and
/// single-element slices pass.
pub fn check_strictly_ascending(values: &[u64]) -> Result<(), InvariantError> {
    if values.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(InvariantError::NotStrictlyAscending)
    }
}
