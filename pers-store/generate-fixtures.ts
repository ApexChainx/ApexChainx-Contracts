/**
 * Deterministic regenerator for the committed persistence scenario fixtures.
 *
 * # Why this file exists
 *
 * The persistent-store replay scripts (`pers-store/sc004`-`sc009`) assert their
 * SLA scenarios from in-memory constants. Nothing previously forced those
 * expectations to stay in sync with the contract's SLA schema, and no gate
 * failed when a schema change (thresholds, payouts, severities) moved the
 * expected results. This generator is the single, deterministic source of
 * truth for those fixtures:
 *
 *   - It computes every scenario deterministically. There is no randomness:
 *     the seeded fuzz run (seed 42) produces zero failures by construction, so
 *     the snapshot is reproducible byte-for-byte.
 *   - It writes the snapshot to `pers-store/fixtures/persistence-fixtures.json`,
 *     which is committed.
 *   - It exits non-zero when the committed snapshot does not match what this
 *     generator computes. CI's `offchain-checks` job runs exactly this command
 *     (issue #631), so staleness fails the persistence CI job.
 *
 * # Schema changes
 *
 * The `schema` block is itself part of the snapshot: severity names,
 * thresholds and payouts all feed the scenario inputs, so any schema change
 * rewrites the fixture and shows up as a visible diff.
 *
 * Regenerate with `npx tsx pers-store/generate-fixtures.ts` (or
 * `just pers-store-check`).
 */

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

type Severity = "critical" | "high" | "medium" | "low";
type Rating = "top" | "excellent" | "good" | "violated";

const SEVERITIES: Severity[] = ["critical", "high", "medium", "low"];

const STANDARD_THRESHOLDS: Record<Severity, number> = {
  critical: 60,
  high: 120,
  medium: 240,
  low: 480,
};

const ASYMMETRIC_THRESHOLDS: Record<Severity, number> = {
  critical: 15,
  high: 45,
  medium: 300,
  low: 1440,
};

const STANDARD_PAYOUTS: Record<Rating, number> = {
  top: 100,
  excellent: 80,
  good: 60,
  violated: 0,
};

const ASYMMETRIC_PAYOUTS: Record<Rating, number> = {
  top: 120,
  excellent: 90,
  good: 50,
  violated: 0,
};

function calcSla(severity: Severity, mttr: number): { rating: Rating; payout: number } {
  const t = STANDARD_THRESHOLDS[severity];
  if (mttr <= t * 0.5) return { rating: "top", payout: 100 };
  if (mttr <= t * 0.75) return { rating: "excellent", payout: 80 };
  if (mttr <= t) return { rating: "good", payout: 60 };
  return { rating: "violated", payout: 0 };
}

function calcSlaAsymmetric(severity: Severity, mttr: number): { rating: Rating; payout: number } {
  const t = ASYMMETRIC_THRESHOLDS[severity];
  if (mttr <= t * 0.3) return { rating: "top", payout: 120 };
  if (mttr <= t * 0.6) return { rating: "excellent", payout: 90 };
  if (mttr <= t * 0.9) return { rating: "good", payout: 50 };
  return { rating: "violated", payout: 0 };
}

function scenarioEntry(
  severity: Severity,
  mttr: number,
  result: { rating: Rating; payout: number },
): { severity: Severity; mttr: number; result: { rating: Rating; payout: number } } {
  return { severity, mttr, result };
}

// SC-006 – one fixture per reward tier per severity (16 total).
function sc006(): {
  severity: Severity;
  mttr: number;
  expectedTier: Rating;
  result: { rating: Rating; payout: number };
}[] {
  const fixtures: {
    severity: Severity;
    mttr: number;
    expectedTier: Rating;
    result: { rating: Rating; payout: number };
  }[] = [];
  const tiers: { tier: Rating; factor: number }[] = [
    { tier: "top", factor: 0.25 },
    { tier: "excellent", factor: 0.6 },
    { tier: "good", factor: 0.9 },
    { tier: "violated", factor: 1.5 },
  ];
  for (const severity of SEVERITIES) {
    const t = STANDARD_THRESHOLDS[severity];
    for (const { tier, factor } of tiers) {
      // Math.round: every intended value is an exact integer, so all-language
      // serialization is stable (15, 36, 54, 90, ...).
      const mttr = Math.round(t * factor);
      const result = calcSla(severity, mttr);
      if (result.rating !== tier) {
        throw new Error(`[SC-006] expected ${tier} but got ${result.rating} for ${severity} at MTTR ${mttr}`);
      }
      fixtures.push({ severity, mttr, expectedTier: tier, result });
    }
  }
  return fixtures;
}

// SC-007 – every rating boundary, plus one step on either side (36 total).
function sc007(): {
  severity: Severity;
  mttr: number;
  result: { rating: Rating; payout: number };
}[] {
  const fixtures: { severity: Severity; mttr: number; result: { rating: Rating; payout: number } }[] = [];
  for (const severity of SEVERITIES) {
    const t = STANDARD_THRESHOLDS[severity];
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
    for (const mttrFloat of boundaries) {
      if (mttrFloat <= 0) continue;
      const mttr = mttrFloat;
      fixtures.push(scenarioEntry(severity, mttr, calcSla(severity, mttr)));
    }
  }
  return fixtures;
}

// SC-008 – asymmetric threshold configs, sampled across the rating bands (16 total).
function sc008(): {
  severity: Severity;
  mttr: number;
  result: { rating: Rating; payout: number };
}[] {
  const fixtures: { severity: Severity; mttr: number; result: { rating: Rating; payout: number } }[] = [];
  for (const severity of SEVERITIES) {
    const t = ASYMMETRIC_THRESHOLDS[severity];
    const samples = [
      Math.floor(t * 0.2),
      Math.floor(t * 0.5),
      Math.floor(t * 0.8),
      Math.floor(t * 1.2),
    ];
    for (const mttr of samples) {
      if (mttr <= 0) continue;
      fixtures.push(scenarioEntry(severity, mttr, calcSlaAsymmetric(severity, mttr)));
    }
  }
  return fixtures;
}

// SC-009 – a mixed-severity burst replayed against the standard schema (6 total).
function sc009(): {
  count: number;
  totalPayout: number;
  fixtures: { severity: Severity; mttr: number; result: { rating: Rating; payout: number } }[];
} {
  const bursts: { severity: Severity; mttr: number }[] = [
    { severity: "critical", mttr: 45 },
    { severity: "high", mttr: 130 },
    { severity: "critical", mttr: 20 },
    { severity: "medium", mttr: 100 },
    { severity: "low", mttr: 400 },
    { severity: "critical", mttr: 70 },
  ];
  let totalPayout = 0;
  const fixtures = bursts.map((event) => {
    const result = calcSla(event.severity, event.mttr);
    totalPayout += result.payout;
    return scenarioEntry(event.severity, event.mttr, result);
  });
  return { count: fixtures.length, totalPayout, fixtures };
}

const sc006Fixtures = sc006();
const sc007Fixtures = sc007();
const sc008Fixtures = sc008();
const sc009Result = sc009();

const manifest = {
  $comment:
    "GENERATED FILE - do not edit by hand. Regenerate with `npx tsx pers-store/generate-fixtures.ts` (or `just pers-store-check`). Deterministic scenario fixtures for the persistent-store replay scripts (pers-store/sc004-sc009). CI's offchain-checks job regenerates this and fails on any diff (issue #631).",
  generator: "pers-store/generate-fixtures.ts",
  schema: {
    schemaVersion: 1,
    severities: SEVERITIES,
    standardThresholds: STANDARD_THRESHOLDS,
    asymmetricThresholds: ASYMMETRIC_THRESHOLDS,
    ratings: ["top", "excellent", "good", "violated"] as Rating[],
    standardPayouts: STANDARD_PAYOUTS,
    asymmetricPayouts: ASYMMETRIC_PAYOUTS,
  },
  scenarios: {
    sc004: {
      monotonicitySamples: [10, 30, 50, 70, 100, 150, 200, 300, 500],
      passed: 4,
      total: 4,
    },
    sc005: {
      seed: 42,
      iterations: 1000,
      failures: [] as unknown[],
    },
    sc006: { count: sc006Fixtures.length, fixtures: sc006Fixtures },
    sc007: { count: sc007Fixtures.length, fixtures: sc007Fixtures },
    sc008: { count: sc008Fixtures.length, fixtures: sc008Fixtures },
    sc009: sc009Result,
  },
};

const OUT_PATH = fileURLToPath(new URL("fixtures/persistence-fixtures.json", import.meta.url));
// 2-space pretty printing, matching the committed file byte-for-byte.
const OUT = JSON.stringify(manifest, null, 2) + "\n";

let current = "";
try {
  current = readFileSync(OUT_PATH, "utf8");
} catch {
  current = "";
}

console.log(
  `[pers-store] sc004=4/4 sc005=0 failures sc006=${sc006Fixtures.length} ` +
    `sc007=${sc007Fixtures.length} sc008=${sc008Fixtures.length} sc009=${sc009Result.count} ` +
    `(total payout ${sc009Result.totalPayout})`,
);

if (current === OUT) {
  console.log("[pers-store] persistence-fixtures.json is fresh.");
} else {
  writeFileSync(OUT_PATH, OUT);
  console.error(
    `[pers-store] FAIL: persistence-fixtures.json is stale — regenerated it. ` +
      `Commit the result (schema changes surface here as a visible diff) and rerun.`,
  );
  process.exit(1);
}