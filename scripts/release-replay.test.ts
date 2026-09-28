/**
 * Smoke tests for the release-replay script.
 *
 * Verifies the script can be imported, its CLI interface works end-to-end,
 * and helper functions return correct data shapes.
 *
 * Run from repo root:
 *   npx tsx --test scripts/release-replay.test.ts
 */

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import test from "node:test";

import { runStep, StepResult, ReplayResult } from "./release-replay.ts";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SCRIPT_PATH = path.resolve(__dirname, "release-replay.ts");

function runReleaseReplay(args: string): { stdout: string; stderr: string; exitCode: number } {
  try {
    const stdout = execFileSync(process.execPath, [createRequire(import.meta.url).resolve("tsx/cli"), SCRIPT_PATH, ...args.split(" ")], {
      cwd: path.resolve(__dirname, ".."),
      timeout: 30_000,
      stdio: "pipe",
    });
    return { stdout: stdout.toString(), stderr: "", exitCode: 0 };
  } catch (err: any) {
    return {
      stdout: err.stdout?.toString() || "",
      stderr: err.stderr?.toString() || "",
      exitCode: err.status || 1,
    };
  }
}

// ---------------------------------------------------------------------------
// CLI interface smoke tests
// ---------------------------------------------------------------------------

test("--help prints usage and exits 0", () => {
  const result = runReleaseReplay("--help");

  assert.equal(result.exitCode, 0, "--help should exit with code 0");
  assert.ok(
    result.stdout.includes("ApexChainx Release Replay"),
    "should include script name"
  );
  assert.ok(
    result.stdout.includes("Usage:"),
    "should include usage section"
  );
  assert.ok(
    result.stdout.includes("--full"),
    "should document --full flag"
  );
  assert.ok(
    result.stdout.includes("--json"),
    "should document --json flag"
  );
  assert.ok(
    result.stdout.includes("RELEASE_PROVENANCE_POLICY"),
    "should reference provenance policy"
  );
});

test("-h (short flag) prints usage and exits 0", () => {
  const result = runReleaseReplay("-h");

  assert.equal(result.exitCode, 0, "-h should exit with code 0");
  assert.ok(
    result.stdout.includes("Usage:"),
    "short flag should print usage"
  );
});

test("unknown options fail before starting validation", () => {
  const result = runReleaseReplay("--unknown-flag");
  assert.equal(result.exitCode, 2);
  assert.match(result.stderr, /Unknown option/);
  assert.doesNotMatch(result.stdout, /validation sequence/);
});

// ---------------------------------------------------------------------------
// JSON output smoke test
// ---------------------------------------------------------------------------

test("--json flag with --help prints help and exits 0 (help takes precedence)", () => {
  const result = runReleaseReplay("--json --help");

  // Help takes precedence over --json in the current implementation,
  // so exit 0 with help text (not JSON).
  assert.equal(result.exitCode, 0);
  assert.ok(
    result.stdout.includes("Usage:"),
    "help text should appear"
  );
});

// ---------------------------------------------------------------------------
// runStep unit tests
// ---------------------------------------------------------------------------

test("runStep returns success for a passing command", () => {
  const result = runStep("Echo test", "echo hello");

  assert.equal(result.success, true);
  assert.equal(result.step, "Echo test");
  assert.equal(result.command, "echo hello");
  assert.ok(result.durationMs >= 0, "duration should be non-negative");
  assert.equal(result.error, undefined, "no error on success");
});

test("runStep returns failure for a failing command", () => {
  const result = runStep("Failing test", "exit 1");

  assert.equal(result.success, false);
  assert.equal(result.step, "Failing test");
  assert.ok(result.durationMs >= 0, "duration should be non-negative");
  assert.ok(result.error, "error should be present on failure");
  assert.ok(
    result.error!.length > 0,
    "error message should not be empty"
  );
});

test("runStep returns failure for a nonexistent command", () => {
  const result = runStep(
    "Nonexistent",
    "this_command_definitely_does_not_exist_12345"
  );

  assert.equal(result.success, false);
  assert.ok(result.error, "error should be present");
});

// ---------------------------------------------------------------------------
// Type shape validation
// ---------------------------------------------------------------------------

test("StepResult has correct shape on success", () => {
  const result = runStep("Type check", "echo ok");

  // Verify the object meets the StepResult interface
  assert.equal(typeof result.step, "string");
  assert.equal(typeof result.command, "string");
  assert.equal(typeof result.success, "boolean");
  assert.equal(typeof result.durationMs, "number");
  assert.equal(result.success, true);
  assert.equal(result.error, undefined);
});

test("StepResult has correct shape on failure", () => {
  const result = runStep("Type check fail", "exit 2");

  assert.equal(typeof result.step, "string");
  assert.equal(typeof result.command, "string");
  assert.equal(typeof result.success, "boolean");
  assert.equal(typeof result.durationMs, "number");
  assert.equal(result.success, false);
  assert.equal(typeof result.error, "string");
});

test("ReplayResult interface is structurally sound", () => {
  // Verify a manually constructed result matches the interface
  const result: ReplayResult = {
    passed: true,
    totalSteps: 1,
    passedSteps: 1,
    failedSteps: 0,
    totalDurationMs: 42,
    steps: [
      {
        step: "test",
        command: "echo test",
        success: true,
        durationMs: 42,
      },
    ],
    generatedAt: new Date().toISOString(),
  };

  assert.equal(result.passed, true);
  assert.equal(result.totalSteps, 1);
  assert.equal(result.failedSteps, 0);
  assert.ok(Date.parse(result.generatedAt) > 0, "generatedAt should be valid ISO");
  assert.equal(result.steps.length, 1);
  assert.equal(result.steps[0].step, "test");
});

// ---------------------------------------------------------------------------
// Real (fast) end-to-end: just release-replay via justfile
// ---------------------------------------------------------------------------

test("just replay recipes invoke the tested CLI with the correct mode", () => {
  const recipes = readFileSync(path.resolve(__dirname, "../justfile"), "utf8");
  assert.match(recipes, /release-replay:\s*\n\s+npx --yes tsx scripts\/release-replay\.ts\s*\n/);
  assert.match(recipes, /release-replay-full:\s*\n\s+npx --yes tsx scripts\/release-replay\.ts --full/);
  assert.equal(runReleaseReplay("--help").exitCode, 0);
});
