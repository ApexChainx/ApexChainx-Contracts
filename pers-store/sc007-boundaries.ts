// [SC-007] Generate SLA fixtures for every boundary condition

type Severity = "critical" | "high" | "medium" | "low";
type Rating = "top" | "excellent" | "good" | "violated";

const THRESHOLDS: Record<Severity, number> = {
  critical: 60,
  high: 120,
  medium: 240,
  low: 480,
};

function calcSla(
  severity: Severity,
  mttr: number,
): { rating: Rating; payout: number } {
  const t = THRESHOLDS[severity];
  if (mttr <= t * 0.5) return { rating: "top", payout: 100 };
  if (mttr <= t * 0.75) return { rating: "excellent", payout: 80 };
  if (mttr <= t) return { rating: "good", payout: 60 };
  return { rating: "violated", payout: 0 };
}

function generateBoundaryFixtures() {
  const severities: Severity[] = ["critical", "high", "medium", "low"];
  const fixtures: any[] = [];
  let passed = 0;

  for (const severity of severities) {
    const t = THRESHOLDS[severity];
    const boundaries = [
      t * 0.5 - 1,
      t * 0.5,
      t * 0.5 + 1,
      t * 0.75 - 1,
      t * 0.75,
      t * 0.75 + 1,
      t - 1,
      t,
      t + 1,
    ];

    for (const mttr of boundaries) {
      if (mttr <= 0) continue;
      const result = calcSla(severity, mttr);
      fixtures.push({ severity, mttr, result });
      passed++;
    }
  }

  console.log(`[SC-007] Generated ${passed} boundary fixtures successfully.`);
  return fixtures;
}

function main() {
  generateBoundaryFixtures();
}

main();
