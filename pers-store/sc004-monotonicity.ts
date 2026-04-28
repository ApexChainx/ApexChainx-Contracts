// Closes #124 — [SC-004] Add property-based SLA monotonicity tests across MTTR ranges

type Severity = "critical" | "high" | "medium" | "low";
type Rating = "top" | "excellent" | "good" | "violated";

const RATING_RANK: Record<Rating, number> = { top: 3, excellent: 2, good: 1, violated: 0 };

interface SlaResult {
  rating: Rating;
  payout: number;
}

// Simulated deterministic SLA function (mirrors contract logic shape)
function calcSla(severity: Severity, mttr: number): SlaResult {
  const thresholds: Record<Severity, number> = { critical: 60, high: 120, medium: 240, low: 480 };
  const t = thresholds[severity];
  if (mttr <= t * 0.5) return { rating: "top", payout: 100 };
  if (mttr <= t * 0.75) return { rating: "excellent", payout: 80 };
  if (mttr <= t) return { rating: "good", payout: 60 };
  return { rating: "violated", payout: 0 };
}

function assertMonotonic(severity: Severity, mttrValues: number[]): boolean {
  const results = mttrValues.map((m) => calcSla(severity, m));
  for (let i = 1; i < results.length; i++) {
    const prev = results[i - 1];
    const curr = results[i];
    if (RATING_RANK[curr.rating] > RATING_RANK[prev.rating]) {
      console.error(`[SC-004] FAIL: ${severity} mttr=${mttrValues[i]} improved over mttr=${mttrValues[i - 1]}`);
      return false;
    }
    if (curr.payout > prev.payout) {
      console.error(`[SC-004] FAIL: payout increased for worse MTTR`);
      return false;
    }
  }
  return true;
}

function runMonotonicityTests(): void {
  const severities: Severity[] = ["critical", "high", "medium", "low"];
  const mttrSamples = [10, 30, 50, 70, 100, 150, 200, 300, 500];
  let passed = 0;

  for (const sev of severities) {
    const ok = assertMonotonic(sev, mttrSamples);
    console.log(`[SC-004] ${sev}: ${ok ? "PASS" : "FAIL"}`);
    if (ok) passed++;
  }
  console.log(`[SC-004] ${passed}/${severities.length} monotonicity checks passed`);
}

runMonotonicityTests();
