/**
 * release-replay.ts
 *
 * Minimal release candidate validation command for maintainers.
 * Runs a fast, focused sequence that validates the most critical
 * release-readiness checks without the full CI pipeline duration.
 *
 * Usage (run from repo root):
 *   npx tsx scripts/release-replay.ts
 *
 * Or via just:
 *   just release-replay
 *
 * What it checks:
 *   1. Format compliance (cargo fmt --check)
 *   2. Clippy warnings denied
 *   3. no-std compliance for wasm32 target
 *   4. Core library tests
 *   5. Topic stability tests
 *   6. WASM build
 *
 * Optional flags:
 *   --full     Also run fuzz tests and full test suite (equivalent to just ci)
 *   --json     Output results as JSON for CI consumption
 */

import { execSync, ExecSyncOptions } from "child_process";
import * as path from "path";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface StepResult {
  step: string;
  command: string;
  success: boolean;
  durationMs: number;
  error?: string;
}

export interface ReplayResult {
  passed: boolean;
  totalSteps: number;
  passedSteps: number;
  failedSteps: number;
  totalDurationMs: number;
  steps: StepResult[];
  generatedAt: string;
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const CRATE_DIR = "apexchainx_calculator";
const WASM_TARGET = "wasm32-unknown-unknown";

const DEFAULT_TIMEOUT_MS = 600_000; // 10 minutes per step
const MAX_BUFFER = 10 * 1024 * 1024; // 10 MB for verbose cargo output

const opts: ExecSyncOptions = {
  stdio: "pipe",
  cwd: process.cwd(),
  timeout: DEFAULT_TIMEOUT_MS,
  maxBuffer: MAX_BUFFER,
};

// ---------------------------------------------------------------------------
// Step runner
// ---------------------------------------------------------------------------

export function runStep(step: string, command: string, cwd?: string, timeoutMs?: number): StepResult {
  const start = Date.now();
  const stepOpts: ExecSyncOptions = { ...opts, timeout: timeoutMs ?? DEFAULT_TIMEOUT_MS };
  if (cwd) stepOpts.cwd = path.resolve(process.cwd(), cwd);

  process.stdout.write(`  ⏳ ${step}... `);

  try {
    execSync(command, stepOpts);
    const durationMs = Date.now() - start;
    console.log(`✅ (${(durationMs / 1000).toFixed(1)}s)`);
    return { step, command, success: true, durationMs };
  } catch (err: unknown) {
    const durationMs = Date.now() - start;
    let error: string;
    if (err instanceof Error) {
      error = (err as any).stderr?.toString() || err.message;
    } else {
      error = String(err);
    }
    console.log(`❌ (${(durationMs / 1000).toFixed(1)}s)`);
    return { step, command, success: false, durationMs, error };
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function main(): void {
  const args = process.argv.slice(2);
  const full = args.includes("--full");
  const json = args.includes("--json");

  if (args.includes("--help") || args.includes("-h")) {
    console.log(
      "ApexChainx Release Replay — minimal release candidate validation\n" +
      "\nUsage:\n" +
      "  npx tsx scripts/release-replay.ts [--full] [--json]\n" +
      "\nOptions:\n" +
      "  --full   Run fuzz tests + full test suite (slower but comprehensive)\n" +
      "  --json   Output results as JSON\n" +
      "\nSee docs/RELEASE_PROVENANCE_POLICY.md for full release procedures.\n"
    );
    process.exit(0);
  }

  console.log("\n🔁 ApexChainx Release Replay");
  console.log(`   ${full ? "Full" : "Minimal"} validation sequence`);
  console.log(`   Started: ${new Date().toISOString()}\n`);

  const steps: StepResult[] = [];

  // Always run the core checks
  steps.push(
    runStep("Format check", "cargo fmt --check", CRATE_DIR)
  );
  steps.push(
    runStep("Clippy lint", "cargo clippy --all-targets -- -D warnings", CRATE_DIR)
  );
  steps.push(
    runStep("no-std check", `cargo check --target ${WASM_TARGET} --lib`, CRATE_DIR)
  );
  steps.push(
    runStep("Core tests", "cargo test --lib", CRATE_DIR)
  );
  steps.push(
    runStep("Topic stability tests", "cargo test topic_stability_tests", CRATE_DIR)
  );
  steps.push(
    runStep("WASM build", `cargo build --target ${WASM_TARGET}`, CRATE_DIR)
  );

  // Optional full checks
  if (full) {
    steps.push(
      runStep("Fuzz tests", "cargo test --lib fuzz_tests::", CRATE_DIR)
    );
    steps.push(
      runStep("Full test suite", "cargo test", CRATE_DIR)
    );
    steps.push(
      runStep("WASM release build", `cargo build --target ${WASM_TARGET} --release`, CRATE_DIR)
    );
  }

  // Summarize
  const passedSteps = steps.filter((s) => s.success).length;
  const failedSteps = steps.filter((s) => !s.success).length;
  const totalDurationMs = steps.reduce((sum, s) => sum + s.durationMs, 0);

  const result: ReplayResult = {
    passed: failedSteps === 0,
    totalSteps: steps.length,
    passedSteps,
    failedSteps,
    totalDurationMs,
    steps,
    generatedAt: new Date().toISOString(),
  };

  if (json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log("─".repeat(62));
    console.log(`  ${result.passed ? "✅ ALL PASSED" : "❌ FAILURES DETECTED"}`);
    console.log(`  ${passedSteps}/${result.totalSteps} steps passed`);
    console.log(`  Duration: ${(totalDurationMs / 1000).toFixed(1)}s`);
    console.log("─".repeat(62));

    // Print failures
    for (const step of steps) {
      if (!step.success) {
        console.log(`\n❌ ${step.step}:`);
        console.log(`   Command: ${step.command}`);
        if (step.error) {
          const truncated = step.error.slice(0, 500);
          console.log(`   Error: ${truncated}`);
        }
      }
    }
    console.log();
  }

  process.exit(result.passed ? 0 : 1);
}

// Run main() only when executed directly, not when imported for testing.
const scriptPath = process.argv[1] ?? "";
if (scriptPath.endsWith("release-replay.ts") || scriptPath.endsWith("release-replay.js")) {
  main();
}
