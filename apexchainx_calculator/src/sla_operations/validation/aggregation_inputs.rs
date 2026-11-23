//! Aggregation input validation.
//!
//! Checks record batches and counters before they are fed to the reporting
//! builders. Problems are reported in the order listed on [`InputError`]; for
//! record batches the first offending record (in input order) decides the
//! error, except duplicate ids, which are checked last.

use alloc::vec::Vec;

/// Most records accepted in one batch.
pub const MAX_RECORDS: usize = 10_000;

/// Why an input was rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    /// No records supplied.
    Empty,
    /// More than [`MAX_RECORDS`] records.
    TooManyRecords,
    /// A record has an empty outage id.
    EmptyOutageId,
    /// A record has an empty severity.
    EmptySeverity,
    /// A record has `threshold_minutes == 0`.
    ZeroThreshold,
    /// Two records share an outage id.
    DuplicateOutageId,
    /// `violations > total`.
    ViolationsExceedTotal,
}

/// A borrowed record to validate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AggregationRecord<'a> {
    pub outage_id: &'a str,
    pub severity: &'a str,
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
}

/// Validate a batch of records.
pub fn validate_records(records: &[AggregationRecord<'_>]) -> Result<(), InputError> {
    if records.is_empty() {
        return Err(InputError::Empty);
    }
    if records.len() > MAX_RECORDS {
        return Err(InputError::TooManyRecords);
    }
    for r in records {
        if r.outage_id.is_empty() {
            return Err(InputError::EmptyOutageId);
        }
        if r.severity.is_empty() {
            return Err(InputError::EmptySeverity);
        }
        if r.threshold_minutes == 0 {
            return Err(InputError::ZeroThreshold);
        }
    }
    let mut ids: Vec<&str> = records.iter().map(|r| r.outage_id).collect();
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(InputError::DuplicateOutageId);
    }
    Ok(())
}

/// Validate a `(total, violations)` counter pair.
pub fn validate_counters(total: u32, violations: u32) -> Result<(), InputError> {
    if violations > total {
        return Err(InputError::ViolationsExceedTotal);
    }
    Ok(())
}
