// SC-047: Prune-performance regression harness for large history datasets.
// Ensures prune stays within a time budget as entry count scales.

const BUDGET_MS = 150;

type Severity = "critical" | "high" | "medium" | "low";

interface Entry {
  id: number;
  severity: Severity;
  mttr: number;
  ts: number;
}

const SEV: Severity[] = ["critical", "high", "medium", "low"];

function makeHistory(n: number): Entry[] {
  return Array.from({ length: n }, (_, i) => ({
    id: i,
    severity: SEV[i % 4],
    mttr: 5 + (i % 480),
    ts: 1_700_000_000_000 - i * 60_000,
  }));
}

function prune(entries: Entry[], keepAfter: number): Entry[] {
  return entries.filter((e) => e.ts >= keepAfter);
}

function bench(size: number, keepRatio: number): { ms: number; remaining: number } {
  const history = makeHistory(size);
  const cutoff = history[Math.floor(size * (1 - keepRatio))]?.ts ?? 0;
  const t0 = performance.now();
  const result = prune(history, cutoff);
  return { ms: performance.now() - t0, remaining: result.length };
}

const cases = [
  { size: 500, keep: 0.4 },
  { size: 1_000, keep: 0.3 },
  { size: 5_000, keep: 0.2 },
];

let failures = 0;
for (const c of cases) {
  const { ms, remaining } = bench(c.size, c.keep);
  const pass = ms < BUDGET_MS;
  console.log(`[${pass ? "OK" : "FAIL"}] prune ${c.size} → ${remaining} entries in ${ms.toFixed(2)}ms`);
  if (!pass) failures++;
}

if (failures) { console.error(`${failures} benchmark(s) exceeded ${BUDGET_MS}ms budget`); process.exit(1); }
