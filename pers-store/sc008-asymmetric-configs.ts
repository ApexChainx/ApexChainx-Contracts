// [SC-008] Generate SLA fixtures for asymmetric configs

type Severity = "critical" | "high" | "medium" | "low";
type Rating = "top" | "excellent" | "good" | "violated";

const ASYMMETRIC_THRESHOLDS: Record<Severity, number> = {
  critical: 15,
  high: 45,
  medium: 300,
  low: 1440,
};

function calcSlaAsymmetric(
  severity: Severity,
  mttr: number,
): { rating: Rating; payout: number } {
  const t = ASYMMETRIC_THRESHOLDS[severity];
  if (mttr <= t * 0.3) return { rating: "top", payout: 120 };
  if (mttr <= t * 0.6) return { rating: "excellent", payout: 90 };
  if (mttr <= t * 0.9) return { rating: "good", payout: 50 };
  return { rating: "violated", payout: 0 };
}

function generateAsymmetricFixtures() {
  const severities: Severity[] = ["critical", "high", "medium", "low"];
  const fixtures: any[] = [];

  for (const severity of severities) {
    const t = ASYMMETRIC_THRESHOLDS[severity];
    const mttrSamples = [
      Math.floor(t * 0.2),
      Math.floor(t * 0.5),
      Math.floor(t * 0.8),
      Math.floor(t * 1.2),
    ];
    for (const mttr of mttrSamples) {
      if (mttr <= 0) continue;
      const result = calcSlaAsymmetric(severity, mttr);
      fixtures.push({ severity, mttr, result });
    }
  }

  console.log(
    `[SC-008] Generated ${fixtures.length} asymmetric config fixtures successfully.`,
  );
  return fixtures;
}

function main() {
  generateAsymmetricFixtures();
}

main();
