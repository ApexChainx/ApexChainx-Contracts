// [SC-006] Generate SLA fixtures for every reward tier

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

function generateRewardTierFixtures() {
  const severities: Severity[] = ["critical", "high", "medium", "low"];
  const fixtures: any[] = [];

  for (const severity of severities) {
    const t = THRESHOLDS[severity];
    const testCases = [
      { tier: "top", mttr: t * 0.25 },
      { tier: "excellent", mttr: t * 0.6 },
      { tier: "good", mttr: t * 0.9 },
      { tier: "violated", mttr: t * 1.5 },
    ];

    for (const { tier, mttr } of testCases) {
      const result = calcSla(severity, mttr);
      fixtures.push({ severity, mttr, expectedTier: tier, result });
      if (result.rating !== tier) {
        console.error(
          `[SC-006] FAIL: Expected ${tier} but got ${result.rating} for ${severity} at MTTR ${mttr}`,
        );
        process.exit(1);
      }
    }
  }

  console.log(
    `[SC-006] Generated ${fixtures.length} reward tier fixtures successfully.`,
  );
  return fixtures;
}

function main() {
  generateRewardTierFixtures();
}

main();
