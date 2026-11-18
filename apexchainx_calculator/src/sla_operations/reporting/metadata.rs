//! Report metadata.
//!
//! Builds the header block of a report (range, record count, generated-at
//! marker, schema version) purely from its inputs. The "generated" marker is a
//! caller-supplied timestamp: this module never reads a clock, so the same
//! inputs always give the same metadata.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

/// Version of the report layout described by [`ReportMetadata`].
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// Why metadata could not be built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataError {
    /// `range_end` is before `range_start`.
    InvalidRange,
}

/// Header information for a report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportMetadata {
    /// Inclusive start of the covered range (unix seconds).
    pub range_start: u64,
    /// Inclusive end of the covered range (unix seconds).
    pub range_end: u64,
    pub record_count: u32,
    /// Caller-supplied generation timestamp (unix seconds).
    pub generated_at: u64,
    pub schema_version: u32,
}

impl ReportMetadata {
    /// Length of the covered range in seconds (`end - start`).
    pub fn duration_seconds(&self) -> u64 {
        self.range_end - self.range_start
    }

    /// Whether the report covers no records.
    pub fn is_empty(&self) -> bool {
        self.record_count == 0
    }

    /// Key/value pairs in a fixed order, ready for a header block.
    pub fn to_pairs(&self) -> Vec<(String, String)> {
        alloc::vec![
            (String::from("schema_version"), self.schema_version.to_string()),
            (String::from("range_start"), self.range_start.to_string()),
            (String::from("range_end"), self.range_end.to_string()),
            (String::from("record_count"), self.record_count.to_string()),
            (String::from("generated_at"), self.generated_at.to_string()),
        ]
    }
}

/// Build metadata for a report covering `[range_start, range_end]`.
///
/// A range where `range_start == range_end` is valid (a single instant).
pub fn build_metadata(
    range_start: u64,
    range_end: u64,
    record_count: u32,
    generated_at: u64,
) -> Result<ReportMetadata, MetadataError> {
    if range_end < range_start {
        return Err(MetadataError::InvalidRange);
    }
    Ok(ReportMetadata {
        range_start,
        range_end,
        record_count,
        generated_at,
        schema_version: REPORT_SCHEMA_VERSION,
    })
}

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn builds_deterministically_and_rejects_inverted_ranges() {
        let a = build_metadata(10, 70, 3, 100).unwrap();
        let b = build_metadata(10, 70, 3, 100).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.duration_seconds(), 60);
        assert_eq!(build_metadata(5, 4, 0, 0), Err(MetadataError::InvalidRange));
        assert!(build_metadata(5, 5, 0, 0).unwrap().is_empty());
    }

    #[test]
    fn pairs_are_in_a_fixed_order() {
        let keys: Vec<String> = build_metadata(1, 2, 3, 4)
            .unwrap()
            .to_pairs()
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(
            keys,
            alloc::vec!["schema_version", "range_start", "range_end", "record_count", "generated_at"]
        );
    }
}
