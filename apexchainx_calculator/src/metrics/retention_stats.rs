//! Types describing history retention metrics.

use soroban_sdk::contracttype;

#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryRetentionMetrics {
    /// Protocol version used to compute these metrics.
    pub protocol_version: u32,

    /// Configured maximum history entries.
    pub retention_limit: u32,

    /// Number of retained history entries.
    pub retained_entries: u32,

    /// Number of removed history entries.
    pub pruned_entries: u32,

    /// Total history entries ever observed.
    pub total_entries: u32,

    /// Retention ratio in basis points.
    ///
    /// Example:
    /// 10000 = 100%
    /// 7500 = 75%
    pub retention_ratio_bps: u32,
}
