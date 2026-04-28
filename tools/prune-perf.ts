// SC-047: Prune-performance regression coverage on large history datasets
// Validates that prune operations stay within acceptable time bounds as history grows.

const PRUNE_BUDGET_MS = 200;

interface HistoryEntry {
  id: number;
  severity: "critical" | "high" | "medium" | "low";
  mttr: number;
  timestamp: number;
}

function generateHistory(size: number): HistoryEntry[] {
  const severities: HistoryEntry["severity"][] = ["critical", "high", "medium", "low"];
  return Array.from({ length: size }, (_, i) => ({
    id: i,
    severity: severities[i % severities.length],
    mttr: 10 + (i % 300),
    timestamp: Date.now() - i * 1000,
  }));
}

function pruneOlderThan(history: HistoryEntry[], cutoffMs: number): HistoryEntry[] {
  return history.filter((e) => e.timestamp >= cutoffMs);
}

function measurePrune(size: number, retainCount: number): number {
  const history = generateHistory(size);
  const cutoff = history[history.length - retainCount]?.timestamp ?? 0;
  const start = performance.now();
  pruneOlderThan(history, cutoff);
  return performance.now() - start;
}

function runPruneRegressionSuite() {
  const cases = [
    { size: 500, retain: 100 },
    { size: 1000, retain: 200 },
    { size: 2000, retain: 500 },
  ];

  let passed = 0;
  for (const { size, retain } of cases) {
    const elapsed = measurePrune(size, retain);
    const ok = elapsed < PRUNE_BUDGET_MS;
    console.log(`prune(${size} → ${retain}): ${elapsed.toFixed(2)}ms [${ok ? "PASS" : "FAIL"}]`);
    if (ok) passed++;
  }

  console.log(`\n${passed}/${cases.length} prune regression checks passed.`);
  if (passed < cases.length) process.exit(1);
}

runPruneRegressionSuite();
