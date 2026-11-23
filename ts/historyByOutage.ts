/**
 * SC-012 — History query by outage identifier (#132)
 *
 * Off-chain systems can retrieve all SLA history entries for a specific
 * outage ID without scanning the full list themselves.
 *
 * Off-chain mirror of the contract's `get_history_by_outage` and
 * `get_latest_by_outage` (`apexchainx_calculator/src/history.rs`).
 *
 * Behaviour:
 *   - Returns entries in insertion order (oldest first) — deterministic.
 *   - Returns an empty array when no entries match (zero-match case).
 *   - Handles repeated outage IDs correctly (many-match case).
 *
 * Verified against contract-recorded lookups by
 * `ts/parity/readSemanticsParity.test.ts`.
 */

export interface HistoryEntry {
  id: string;
  outageId: string;
  severity: string;
  mttr: number;
  slaMetPct: number;
  recordedAt: number;
}

export interface OutageQueryResult {
  outageId: string;
  entries: HistoryEntry[];
  count: number;
}

/**
 * Returns all history entries matching `outageId`, preserving insertion order.
 *
 * @param history  - full append-only history array
 * @param outageId - identifier to filter by
 */
export function getHistoryByOutage(
  history: HistoryEntry[],
  outageId: string,
): OutageQueryResult {
  const entries = history.filter((e) => e.outageId === outageId);
  return { outageId, entries, count: entries.length };
}

/**
 * Returns the most recent entry for `outageId`, mirroring the contract's
 * `get_latest_by_outage`.
 *
 * "Most recent" is the **last matching entry in insertion order**, not the one
 * with the largest `recordedAt`. The distinction matters: history is
 * append-only and never reordered, so the contract scans forward and keeps the
 * last match rather than comparing timestamps. Sorting by `recordedAt` would
 * disagree whenever two results share a ledger timestamp.
 *
 * Returns `null` when nothing matches — the contract returns `None`.
 */
export function getLatestByOutage(
  history: HistoryEntry[],
  outageId: string,
): HistoryEntry | null {
  let latest: HistoryEntry | null = null;
  for (const entry of history) {
    if (entry.outageId === outageId) {
      latest = entry;
    }
  }
  return latest;
}
