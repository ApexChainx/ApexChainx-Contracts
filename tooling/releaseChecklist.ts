/**
 * releaseChecklist.ts
 * Issue #160 — enforces the repeatable release checklist for apexchainx_calculator.
 * Run before tagging a release to catch common mistakes early.
 */

import { execSync } from "child_process";
import { existsSync, statSync } from "fs";

interface CheckResult { label: string; passed: boolean; note?: string }

function run(label: string, fn: () => void): CheckResult {
  try { fn(); return { label, passed: true }; }
  catch (e: any) { return { label, passed: false, note: e.message }; }
}

const WASM = "apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm";
const WASM_SIZE_LIMIT_KB = 100;

const checks: CheckResult[] = [
  run("cargo test passes", () =>
    execSync("cargo test", { cwd: "apexchainx_calculator", stdio: "pipe" })),

  run("WASM artifact exists", () => {
    if (!existsSync(WASM)) throw new Error(`${WASM} not found — run cargo build --release`);
  }),

  run(`WASM size < ${WASM_SIZE_LIMIT_KB} KB`, () => {
    const kb = statSync(WASM).size / 1024;
    if (kb > WASM_SIZE_LIMIT_KB) throw new Error(`${kb.toFixed(1)} KB exceeds limit`);
  }),

  run("CHANGELOG.md has Unreleased section", () => {
    const log = require("fs").readFileSync("CHANGELOG.md", "utf8") as string;
    if (!log.includes("## [Unreleased]")) throw new Error("No [Unreleased] section found");
  }),

  run("No uncommitted changes", () =>
    execSync("git diff --exit-code", { stdio: "pipe" })),
];

const failed = checks.filter((c) => !c.passed);

checks.forEach((c) =>
  console.log(`${c.passed ? "✓" : "✗"} ${c.label}${c.note ? ` — ${c.note}` : ""}`)
);

if (failed.length) {
  console.error(`\n${failed.length} check(s) failed. Fix before tagging.`);
  process.exit(1);
}
console.log("\nAll checks passed. Safe to tag.");
