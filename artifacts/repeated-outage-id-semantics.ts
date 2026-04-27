/**
 * SC-032: Repeated outage-ID semantics for recompute and history retrieval.
 * Documents and tests the contract's intended behaviour when the same outage ID
 * is submitted multiple times (replay / recompute workflows).
 */

export interface SlaResult {
  outageId: string;
  severity: string;
  mttr: number;
  slaMet: boolean;
  score: number;
  calculatedAt: number;
}

type HistoryStore = Map<string, SlaResult[]>;

/** Simulates contract history append — latest entry wins for "get_latest". */
export function recordResult(store: HistoryStore, result: SlaResult): void {
  const existing = store.get(result.outageId) ?? [];
  store.set(result.outageId, [...existing, result]);
}

export function getLatest(store: HistoryStore, outageId: string): SlaResult | undefined {
  const entries = store.get(outageId);
  return entries ? entries[entries.length - 1] : undefined;
}

export function getHistory(store: HistoryStore, outageId: string): SlaResult[] {
  return store.get(outageId) ?? [];
}

// --- tests ---

function assert(cond: boolean, msg: string): void {
  if (!cond) throw new Error(`FAIL: ${msg}`);
}

function makeResult(outageId: string, score: number, t: number): SlaResult {
  return { outageId, severity: "high", mttr: 100, slaMet: score >= 70, score, calculatedAt: t };
}

if (require.main === module) {
  const store: HistoryStore = new Map();

  // First calculation
  recordResult(store, makeResult("INC-001", 60, 1000));
  assert(getLatest(store, "INC-001")?.score === 60, "initial score should be 60");
  assert(getHistory(store, "INC-001").length === 1, "history length should be 1");

  // Recompute — same outage ID, different score
  recordResult(store, makeResult("INC-001", 85, 2000));
  assert(getLatest(store, "INC-001")?.score === 85, "latest should reflect recompute");
  assert(getHistory(store, "INC-001").length === 2, "history should retain both entries");

  // Second recompute
  recordResult(store, makeResult("INC-001", 95, 3000));
  assert(getLatest(store, "INC-001")?.score === 95, "latest should be most recent");
  assert(getHistory(store, "INC-001").length === 3, "full history preserved");

  // Unrelated outage unaffected
  assert(getLatest(store, "INC-002") === undefined, "unknown outage returns undefined");

  console.log("All repeated-outage-ID semantics tests passed ✓");
}
