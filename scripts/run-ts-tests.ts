/**
 * Cross-platform runner for the repository's TypeScript test suites.
 *
 * Why this exists:
 *   1. `npm test` used to run only `release-replay.test.ts` and the
 *      read-semantics parity suite, while the property suite in `tests/`
 *      (the widest randomized matrix: validity, ordering, parity) ran only
 *      via `npm run test:property` / CI. A green `npm test` therefore said
 *      nothing about the property tests.
 *   2. The previous scripts selected suites with a shell glob
 *      (`tsx --test tests/*.test.ts`). npm's default shell on Windows
 *      (cmd.exe) does not expand globs, so tsx received the literal pattern
 *      `tests\*.test.ts`, found no such file, and exited 0 after running
 *      **zero tests**. A broken suite selection looked like a pass.
 *
 * This runner expands the file list itself (via the filesystem, not the
 * shell) and refuses to run a suite that resolves to zero files, so the
 * silent-pass failure mode is structurally impossible.
 *
 * Usage:
 *   tsx scripts/run-ts-tests.ts [suite]
 *
 * Suites:
 *   all       property + release-replay + parity (default; what `npm test` runs)
 *   property  tests/*.test.ts (property/regression matrix)
 *   replay    scripts/release-replay.test.ts (release-replay smoke tests)
 *   parity    ts/parity/readSemanticsParity.test.ts (read-semantics parity)
 *   list      print the resolved files per suite and exit
 */

import { execSync } from "child_process";
import { existsSync, readdirSync } from "fs";

// ─── Suite definitions ────────────────────────────────────────────────────

/**
 * Property/regression suite discovered from tests/ at run time (not via a
 * shell glob, which cmd.exe does not expand — see the header comment).
 * Sorted so the run order is deterministic across platforms.
 */
function propertySuiteFiles(): string[] {
  if (!existsSync("tests")) {
    throw new Error("tests/ directory not found — run from the repo root");
  }
  return readdirSync("tests")
    .filter((name) => name.endsWith(".test.ts"))
    .sort()
    .map((name) => `tests/${name}`);
}

/** Every suite, in the order the full run executes them. */
function allSuiteFiles(): string[] {
  return [...propertySuiteFiles(), ...SUITES.replay, ...SUITES.parity];
}

// Declared after the helper so the initializer can call it.
const SUITES: Record<string, string[]> = {
  replay: ["scripts/release-replay.test.ts"],
  parity: ["ts/parity/readSemanticsParity.test.ts"],
};

const SUITE_RESOLVERS: Record<string, () => string[]> = {
  all: allSuiteFiles,
  property: propertySuiteFiles,
  replay: () => SUITES.replay,
  parity: () => SUITES.parity,
};

// ─── Suite resolution ─────────────────────────────────────────────────────

function resolveSuite(name: string): string[] {
  const resolver = SUITE_RESOLVERS[name];
  if (!resolver) {
    throw new Error(
      `Unknown suite "${name}". Valid suites: ${Object.keys(SUITE_RESOLVERS).join(", ")}`
    );
  }
  const files = resolver();
  if (files.length === 0) {
    throw new Error(
      `Suite "${name}" resolved to zero test files — refusing to run an empty suite. ` +
        "If test files moved, update scripts/run-ts-tests.ts."
    );
  }
  for (const file of files) {
    if (!existsSync(file)) {
      throw new Error(`Suite "${name}" references missing file: ${file}`);
    }
  }
  return files;
}

function printSuiteFiles(): void {
  for (const name of Object.keys(SUITE_RESOLVERS)) {
    console.log(`${name}:`);
    for (const file of resolveSuite(name)) {
      console.log(`  ${file}`);
    }
  }
}

function printHelp(): void {
  console.log("Usage: tsx scripts/run-ts-tests.ts [suite]");
  console.log("\nSuites:");
  console.log("  all       property + release-replay + parity (default)");
  console.log("  property  tests/*.test.ts (property/regression matrix)");
  console.log("  replay    scripts/release-replay.test.ts");
  console.log("  parity    ts/parity/readSemanticsParity.test.ts");
  console.log("  list      print the resolved files per suite and exit");
}

// ─── Entry point ──────────────────────────────────────────────────────────

const suite = process.argv.slice(2)[0] ?? "all";

if (suite === "--help" || suite === "-h") {
  printHelp();
  process.exit(0);
}

if (suite === "list") {
  printSuiteFiles();
  process.exit(0);
}

const files = resolveSuite(suite);

// Quote each path (no spaces today, but cheap insurance) and rely on forward
// slashes, which Node accepts on every platform. No shell globs anywhere.
const cmd = `npx tsx --test ${files.map((f) => `"${f}"`).join(" ")}`;
console.log(`[run-ts-tests] suite: ${suite} (${files.length} file(s))`);
console.log(`[run-ts-tests] $ ${cmd}\n`);

try {
  execSync(cmd, { stdio: "inherit" });
} catch (err) {
  const status = (err as { status?: number | null }).status ?? 1;
  console.error(`\n[run-ts-tests] ✗ suite "${suite}" failed (exit ${status})`);
  process.exit(status || 1);
}

console.log(`\n[run-ts-tests] ✓ suite "${suite}" passed (${files.length} file(s))`);
