/**
 * SC-016 / SC-W5-029: Read-cost regression tests for history, config-bundle,
 * and version negotiation helpers.
 * Simulates budget-aware reads against contract view helpers and asserts
 * that response sizes stay within documented thresholds.
 *
 * Payload shapes match the on-chain ABI: the SLAConfigSnapshot entries,
 * SLAResult tuples, and VersionInfo struct as returned by the contract.
 * Sizes are estimated using Soroban SCVal encoding widths, not JSON.
 *
 * # Issue #614: regression gating
 * Two layers are enforced:
 *  1. Absolute budgets (READ_BUDGET_LIMITS) — a payload shape may never
 *     grow past the documented per-helper ceiling.
 *  2. Regression baseline (readCostBaseline.json) — a payload shape may not
 *     grow more than READ_REGRESSION_TOLERANCE_FRACTION above its committed
 *     baseline size. The baseline is the "before" measurement; every run
 *     writes the "after" measurements to readCostMeasurements.json so CI can
 *     publish before/after artifacts alongside the build.
 *
 * Refresh the baseline deliberately (a documented, reviewed enlargement —
 * e.g. an additive SLAResult field) with `--update-baseline`, which rewrites
 * readCostBaseline.json from the current measurements. A PR that grows a
 * payload without updating the baseline fails this script.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// ─── Soroban SCVal byte-size estimation ───────────────────────────────────
// On-chain, view responses are SCVal-encoded. We estimate sizes using
// the same byte-width rules the Soroban host uses.

function estimateScValSize(payload: unknown): number {
  if (Array.isArray(payload)) {
    let size = 4; // Vec header
    for (const val of payload) {
      size += estimateScValSize(val);
    }
    return size;
  }
  if (typeof payload === "string") {
    // Symbol or Address: 4 bytes overhead + content (up to 9 for short Symbol)
    return 4 + Math.min(payload.length, 9);
  }
  if (typeof payload === "boolean") return 4;
  if (typeof payload === "number") {
    return Number.isInteger(payload) && payload >= 0 && payload <= 0xFFFFFFFF ? 4 : 16;
  }
  if (typeof payload === "bigint") return 16;
  if (typeof payload === "object" && payload !== null) {
    // Map/Object: estimate as a flat SCVal vec of key-value pairs
    let size = 4; // Map header
    for (const val of Object.values(payload as Record<string, unknown>)) {
      size += 4; // key Symbol overhead
      size += estimateScValSize(val);
    }
    return size;
  }
  return 0;
}

const READ_BUDGET_LIMITS = {
  /** SLAConfigSnapshot: Vec<SLAConfigEntry> with 4 canonical entries. */
  configSnapshot: 320,
  /** History entry: SLAResult tuple (9 fields). */
  historyEntry: 96,
  /** VersionInfo struct (6 fields). */
  versionInfo: 80,
  /** Metadata read (contract_info fields). */
  metadataRead: 128,
};

/**
 * Largest permitted relative growth over the committed baseline before a
 * change is treated as a read-cost regression. 0.10 = +10%.
 */
const READ_REGRESSION_TOLERANCE_FRACTION = 0.1;

// Offchain scripts are executed from the repository root in CI.
const BASELINE_PATH = join(process.cwd(), "offchain", "readCostBaseline.json");
const MEASUREMENTS_PATH = join(process.cwd(), "offchain", "readCostMeasurements.json");

interface ReadSample {
  helper: keyof typeof READ_BUDGET_LIMITS;
  payload: unknown;
}

function measure(samples: ReadSample[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const sample of samples) {
    out[sample.helper] = estimateScValSize(sample.payload);
  }
  return out;
}

function assertReadBudget(sample: ReadSample): void {
  const limit = READ_BUDGET_LIMITS[sample.helper];
  const size = estimateScValSize(sample.payload);
  if (size > limit) {
    throw new Error(
      `[SC-016] ${sample.helper} payload ${size}B exceeds budget ${limit}B`
    );
  }
  console.log(`  ✓ ${sample.helper}: ${size}B / ${limit}B`);
}

function assertNoRegression(
  baseline: Record<string, number>,
  current: Record<string, number>
): void {
  for (const [helper, after] of Object.entries(current)) {
    const before = baseline[helper];
    if (before === undefined) {
      throw new Error(
        `[SC-016/#614] ${helper} has no committed baseline; run 'tsx offchain/readCostRegression.ts --update-baseline' after a reviewed payload enlargement.`
      );
    }
    const ceiling = Math.ceil(before * (1 + READ_REGRESSION_TOLERANCE_FRACTION));
    if (after > ceiling) {
      throw new Error(
        `[SC-016/#614] ${helper} read cost BEFORE ${before}B -> AFTER ${after}B exceeds +${Math.round(READ_REGRESSION_TOLERANCE_FRACTION * 100)}% regression tolerance (ceiling ${ceiling}B). If the enlargement is intentional and reviewed, update the baseline with --update-baseline.`
      );
    }
    if (after >= before) {
      console.log(`  ~ ${helper}: ${before}B -> ${after}B (within tolerance)`);
    } else {
      console.log(`  ~ ${helper}: ${before}B -> ${after}B (improvement)`);
    }
  }
}

// ─── Mock payloads matching real on-chain shapes ──────────────────────────

/**
 * SLAConfigSnapshot: Vec<SLAConfigEntry> in canonical order.
 * Each SLAConfigEntry = (severity: Symbol, config: SLAConfig)
 * SLAConfig = (threshold_minutes: u32, penalty_per_minute: i128, reward_base: i128)
 */
const mockConfigSnapshot = [
  { severity: "critical", threshold_minutes: 15, penalty_per_minute: 100, reward_base: 750 },
  { severity: "high", threshold_minutes: 30, penalty_per_minute: 50, reward_base: 750 },
  { severity: "medium", threshold_minutes: 60, penalty_per_minute: 25, reward_base: 750 },
  { severity: "low", threshold_minutes: 120, penalty_per_minute: 10, reward_base: 600 },
];

/**
 * History entry: SLAResult tuple (9 fields).
 * (outage_id: Symbol, severity: Symbol, status: Symbol, payment_type: Symbol,
 *  rating: Symbol, mttr_minutes: u32, threshold_minutes: u32,
 *  config_version_hash: u64, amount: i128)
 */
const mockHistoryEntry = [
  "out_001", "critical", "met", "rew", "top", 10, 15, 0, 1500,
];

/**
 * VersionInfo: (storage_version: u32, result_schema_version: u32,
 *   needs_migration: bool, is_paused: bool, contract_name: Symbol,
 *   event_version: Symbol)
 */
const mockVersionInfo = [1, 1, false, false, "sla_calc", "v1"];

/**
 * Metadata: contract_info fields returned by get_metadata.
 * (version: Symbol, paused: bool, operator: Address, admin: Address)
 */
const mockMetadataRead = ["1.0.0", false, "GOPER123", "GADMIN456"];

const samples: ReadSample[] = [
  { helper: "configSnapshot", payload: mockConfigSnapshot },
  { helper: "historyEntry", payload: mockHistoryEntry },
  { helper: "versionInfo", payload: mockVersionInfo },
  { helper: "metadataRead", payload: mockMetadataRead },
];

function run(): void {
  console.log("[SC-016/SC-W5-029] Read-cost regression checks (Soroban SCVal estimation):");
  samples.forEach(assertReadBudget);

  const current = measure(samples);
  const wantsBaselineUpdate = process.argv.includes("--update-baseline");
  const baseline: Record<string, number> =
    readJSONOrEmpty(BASELINE_PATH);

  if (wantsBaselineUpdate) {
    writeFileSync(BASELINE_PATH, JSON.stringify(current, null, 2) + "\n");
    console.log(`Updated read-cost baseline (${Object.keys(current).length} helpers).`);
  } else {
    assertNoRegression(baseline, current);
  }

  // Failure artifacts: always record the before/after measurements (and the
  // budgets) so CI uploads a diffable report on every run.
  writeFileSync(
    MEASUREMENTS_PATH,
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        toleranceFraction: READ_REGRESSION_TOLERANCE_FRACTION,
        budgets: READ_BUDGET_LIMITS,
        baseline: baseline,
        current: current,
      },
      null,
      2
    ) + "\n"
  );
  console.log("All read-cost checks passed.");
}

function readJSONOrEmpty(path: string): Record<string, number> {
  try {
    return JSON.parse(readFileSync(path, "utf8")) as Record<string, number>;
  } catch {
    return {};
  }
}

run();