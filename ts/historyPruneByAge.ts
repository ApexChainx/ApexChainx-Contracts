/**
 * SC-014 — Prune-by-age / pruning-window semantics (#134)
 *
 * Extends the existing count-based pruning with an age-oriented strategy.
 * Operators can prune all entries older than a given ledger timestamp
 * (absolute cutoff) or keep only entries within a rolling window
 * (relative to the newest entry).
 *
 * Both modes are deterministic: given the same history and parameters
 * they always produce the same result.
 */

export interface HistoryEntry {
  id: string;
  outageId: string;
  severity: string;
  mttr: number;
  slaMetPct: number;
  recordedAt: number; // ledger timestamp
}

export interface PruneResult {
  kept: HistoryEntry[];
  pruned: number;
}

/**
 * Prune entries with `recordedAt` strictly before `cutoffTimestamp`.
 */
export function pruneByAge(
  history: HistoryEntry[],
  cutoffTimestamp: number,
): PruneResult {
  const kept = history.filter((e) => e.recordedAt >= cutoffTimestamp);
  return { kept, pruned: history.length - kept.length };
}

/**
 * Keep only entries within `windowSize` ledger ticks of the newest entry.
 * If history is empty the result is an empty array.
 */
export function pruneByWindow(
  history: HistoryEntry[],
  windowSize: number,
): PruneResult {
  if (history.length === 0) return { kept: [], pruned: 0 };
  const newest  = Math.max(...history.map((e) => e.recordedAt));
  const cutoff  = newest - windowSize;
  return pruneByAge(history, cutoff);
}

// ---------------------------------------------------------------------------
// Quick self-test
// ---------------------------------------------------------------------------
if (require.main === module) {
  const history: HistoryEntry[] = [
    { id: "e0", outageId: "o1", severity: "high", mttr: 60, slaMetPct: 100, recordedAt: 100 },
    { id: "e1", outageId: "o2", severity: "high", mttr: 60, slaMetPct: 100, recordedAt: 200 },
    { id: "e2", outageId: "o3", severity: "high", mttr: 60, slaMetPct: 100, recordedAt: 300 },
  ];

  const r1 = pruneByAge(history, 200);
  console.assert(r1.pruned === 1 && r1.kept.length === 2, "prune-by-age");

  const r2 = pruneByWindow(history, 150);
  console.assert(r2.kept.length === 2 && r2.pruned === 1, "prune-by-window");

  const r3 = pruneByWindow([], 100);
  console.assert(r3.kept.length === 0, "empty history");

  console.log("prune-by-age OK, pruned:", r1.pruned);
  console.log("prune-by-window OK, kept:", r2.kept.length);
}
