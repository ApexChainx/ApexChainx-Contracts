/**
 * SC-012 — History query by outage identifier (#132)
 *
 * Off-chain systems can retrieve all SLA history entries for a specific
 * outage ID without scanning the full list themselves.
 *
 * Behaviour:
 *   - Returns entries in insertion order (oldest first) — deterministic.
 *   - Returns an empty array when no entries match (zero-match case).
 *   - Handles repeated outage IDs correctly (many-match case).
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

// ---------------------------------------------------------------------------
// Quick self-test
// ---------------------------------------------------------------------------
if (require.main === module) {
  const history: HistoryEntry[] = [
    { id: "e0", outageId: "OUT-1", severity: "critical", mttr: 30,  slaMetPct: 100, recordedAt: 1 },
    { id: "e1", outageId: "OUT-2", severity: "high",     mttr: 90,  slaMetPct: 80,  recordedAt: 2 },
    { id: "e2", outageId: "OUT-1", severity: "critical", mttr: 45,  slaMetPct: 100, recordedAt: 3 },
    { id: "e3", outageId: "OUT-1", severity: "medium",   mttr: 200, slaMetPct: 60,  recordedAt: 4 },
  ];

  const r0 = getHistoryByOutage(history, "OUT-NONE");
  console.assert(r0.count === 0, "zero match");

  const r1 = getHistoryByOutage(history, "OUT-2");
  console.assert(r1.count === 1, "one match");

  const r2 = getHistoryByOutage(history, "OUT-1");
  console.assert(r2.count === 3, "many matches");
  console.assert(r2.entries[0].id === "e0", "insertion order preserved");

  console.log("outage query OK — OUT-1 entries:", r2.count);
}
