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
 *
 * [`pruneByMinAge`] is the direct mirror of the contract's
 * `prune_history_by_age` and is the one to use when reconciling with on-chain
 * state; `pruneByAge` and `pruneByWindow` are off-chain conveniences over the
 * same predicate. Verified against contract-recorded prunes by
 * `ts/parity/readSemanticsParity.test.ts`.
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

/**
 * Prunes by **relative age**, mirroring the contract's
 * `prune_history_by_age(min_age_seconds)`.
 *
 * The contract computes `cutoff = now.saturating_sub(min_age_seconds)` from the
 * *ledger* timestamp and keeps every entry with `recorded_at >= cutoff`. Two
 * details are easy to get wrong and are handled here explicitly:
 *
 *   - **`now` is the ledger timestamp, not wall-clock time.** Callers must pass
 *     the ledger timestamp they are reconciling against, or the mirror will
 *     disagree with the chain.
 *   - **The subtraction saturates.** A `min_age_seconds` larger than `now` —
 *     `u64::MAX` being the obvious case — clamps the cutoff to `0` and keeps
 *     everything, rather than underflowing into a huge cutoff that would prune
 *     the entire history.
 *
 * Both arguments are `bigint` because the contract's are `u64`, which exceeds
 * the range a JavaScript number represents exactly.
 */
export function pruneByMinAge(
  history: HistoryEntry[],
  now: bigint,
  minAgeSeconds: bigint,
): PruneResult {
  const cutoff = now > minAgeSeconds ? now - minAgeSeconds : 0n;
  const kept = history.filter((e) => BigInt(e.recordedAt) >= cutoff);
  return { kept, pruned: history.length - kept.length };
}
