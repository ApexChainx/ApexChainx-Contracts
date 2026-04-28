/**
 * SC-043: Focused test runner for faster local contract test cycles.
 * Wraps `cargo test` with filter presets so contributors skip unrelated tests.
 */

import { execSync, ExecSyncOptions } from "child_process";

const PRESETS: Record<string, string> = {
  governance: "governance",
  calc:       "calculate_sla",
  stats:      "stats",
  pause:      "pause",
  history:    "history",
  parity:     "backend_parity",
  all:        "",
};

const opts: ExecSyncOptions = { stdio: "inherit", cwd: "apexchainx_calculator" };

function run(filter: string, release: boolean): void {
  const filterArg = filter ? ` ${filter}` : "";
  const profile = release ? " --release" : "";
  const cmd = `cargo test${profile}${filterArg} -- --nocapture`;
  console.log(`\n$ ${cmd}\n`);
  execSync(cmd, opts);
}

function printHelp(): void {
  console.log("Usage: ts-node run-tests.ts <preset|filter> [--release]");
  console.log("\nPresets:");
  for (const [name, filter] of Object.entries(PRESETS)) {
    console.log(`  ${name.padEnd(12)} → cargo test ${filter || "(all)"}`);
  }
  console.log("\nOr pass any string to use as a direct cargo test filter.");
  console.log("\nExamples:");
  console.log("  ts-node scripts/run-tests.ts governance");
  console.log("  ts-node scripts/run-tests.ts parity --release");
  console.log("  ts-node scripts/run-tests.ts test_pause_blocks_calculate");
}

const args = process.argv.slice(2);
if (!args.length || args[0] === "--help") {
  printHelp();
  process.exit(0);
}

const preset = args[0];
const release = args.includes("--release");
const filter = PRESETS[preset] !== undefined ? PRESETS[preset] : preset;

run(filter, release);
