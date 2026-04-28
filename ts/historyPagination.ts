/**
 * SC-010 — Paginated history read helpers (#130)
 *
 * Provides stable, offset-based pagination over an append-only SLA history
 * array.  Ordering is insertion order (oldest first).
 *
 * Cursor semantics:
 *   - `offset` is the 0-based index of the first record to return.
 *   - `limit`  is the maximum number of records per page (capped at MAX_PAGE).
 *   - A page with fewer than `limit` items signals end-of-history.
 *   - An empty page (offset >= total) is valid and returns [].
 */

export interface HistoryEntry {
  id: string;
  outageId: string;
  severity: string;
  mttr: number;
  slaMetPct: number;
  recordedAt: number;
}

export interface HistoryPage {
  entries: HistoryEntry[];
  offset: number;
  total: number;
  hasMore: boolean;
}

const MAX_PAGE = 50;

/**
 * Returns a bounded page of history entries.
 *
 * @param history - full append-only history array (oldest first)
 * @param offset  - 0-based start index
 * @param limit   - desired page size (clamped to MAX_PAGE)
 */
export function getHistoryPage(
  history: HistoryEntry[],
  offset: number,
  limit: number,
): HistoryPage {
  const safeOffset = Math.max(0, Math.floor(offset));
  const safeLimit  = Math.min(MAX_PAGE, Math.max(1, Math.floor(limit)));
  const slice      = history.slice(safeOffset, safeOffset + safeLimit);

  return {
    entries: slice,
    offset:  safeOffset,
    total:   history.length,
    hasMore: safeOffset + slice.length < history.length,
  };
}

// ---------------------------------------------------------------------------
// Quick self-test
// ---------------------------------------------------------------------------
if (require.main === module) {
  const history: HistoryEntry[] = Array.from({ length: 7 }, (_, i) => ({
    id: `e${i}`, outageId: `o${i % 3}`, severity: "high",
    mttr: 60, slaMetPct: 100, recordedAt: i,
  }));

  const p1 = getHistoryPage(history, 0, 3);
  console.assert(p1.entries.length === 3 && p1.hasMore, "page 1");

  const p2 = getHistoryPage(history, 3, 3);
  console.assert(p2.entries.length === 3 && p2.hasMore, "page 2");

  const p3 = getHistoryPage(history, 6, 3);
  console.assert(p3.entries.length === 1 && !p3.hasMore, "last page");

  const p4 = getHistoryPage(history, 99, 3);
  console.assert(p4.entries.length === 0 && !p4.hasMore, "past end");

  console.log("pagination OK — total:", p1.total);
}
