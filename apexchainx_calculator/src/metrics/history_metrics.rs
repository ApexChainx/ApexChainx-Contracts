//! Metric calculation logic for history retention.

use super::retention_stats::HistoryRetentionMetrics;

/// Builds a deterministic metrics snapshot.
///
/// This helper performs no storage mutations and is safe to call
/// from read-only contract endpoints.
pub fn build_history_metrics(
    protocol_version: u32,
    retention_limit: u32,
    retained_entries: u32,
    pruned_entries: u32,
) -> HistoryRetentionMetrics {
    let total_entries = retained_entries + pruned_entries;

    let retention_ratio_bps = if total_entries == 0 {
        0
    } else {
        retained_entries * 10_000 / total_entries
    };

    HistoryRetentionMetrics {
        protocol_version,
        retention_limit,
        retained_entries,
        pruned_entries,
        total_entries,
        retention_ratio_bps,
    }
}
