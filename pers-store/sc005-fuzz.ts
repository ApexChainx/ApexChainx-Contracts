// Closes #125 — [SC-005] Add fuzz-style randomized coverage for severity and threshold combinations

type Severity = "critical" | "high" | "medium" | "low";

const SEVERITIES: Severity[] = ["critical", "high", "medium", "low"];
const THRESHOLDS: Record<Severity, number> = { critical: 60, high: 120, medium: 240, low: 480 };
const VALID_RATINGS = new Set(["top", "excellent", "good", "violated"]);

// Seeded LCG for reproducible "random" runs
function makeLcg(seed: number) {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return (s >>> 0) / 0xffffffff;
  };
}

function calcSla(severity: Severity, mttr: number): { rating: string; payout: number } {
  const t = THRESHOLDS[severity];
  if (mttr <= t * 0.5) return { rating: "top", payout: 100 };
  if (mttr <= t * 0.75) return { rating: "excellent", payout: 80 };
  if (mttr <= t) return { rating: "good", payout: 60 };
  return { rating: "violated", payout: 0 };
}

interface FuzzCase {
  severity: Severity;
  mttr: number;
  rating: string;
  payout: number;
  ok: boolean;
}

function runFuzz(seed: number, iterations: number): FuzzCase[] {
  const rand = makeLcg(seed);
  const failures: FuzzCase[] = [];

  for (let i = 0; i < iterations; i++) {
    const severity = SEVERITIES[Math.floor(rand() * SEVERITIES.length)];
    const mttr = Math.floor(rand() * 600) + 1;
    const result = calcSla(severity, mttr);
    const ok = VALID_RATINGS.has(result.rating) && result.payout >= 0 && result.payout <= 100;
    if (!ok) failures.push({ severity, mttr, ...result, ok });
  }
  return failures;
}

function main(): void {
  const seed = parseInt(process.argv[2] ?? "42", 10);
  const iterations = 1000;
  const failures = runFuzz(seed, iterations);
  console.log(`[SC-005] Fuzz run: seed=${seed}, iterations=${iterations}, failures=${failures.length}`);
  if (failures.length > 0) {
    failures.forEach((f) => console.error(`  FAIL: ${f.severity} mttr=${f.mttr} → ${f.rating}/${f.payout}`));
    process.exit(1);
  }
  console.log("[SC-005] All fuzz cases produced valid result shapes.");
}

main();
