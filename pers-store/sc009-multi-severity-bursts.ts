// [SC-009] Generate SLA fixtures for multi-severity bursts

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

function generateBurstFixtures() {
  const bursts = [
    { severity: "critical", mttr: 45 },
    { severity: "high", mttr: 130 },
    { severity: "critical", mttr: 20 },
    { severity: "medium", mttr: 100 },
    { severity: "low", mttr: 400 },
    { severity: "critical", mttr: 70 },
  ] as const;

  const fixtures: any[] = [];
  let totalPayout = 0;

  for (const event of bursts) {
    const result = calcSla(event.severity, event.mttr);
    totalPayout += result.payout;
    fixtures.push({ ...event, result });
  }

  console.log(
    `[SC-009] Generated multi-severity burst fixtures (total payout: ${totalPayout}).`,
  );
  return fixtures;
}

function main() {
  generateBurstFixtures();
}

main();
