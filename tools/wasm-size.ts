#!/usr/bin/env -S npx tsx
/**
 * Issue #82 — WASM Size History Tracking
 *
 * Two responsibilities in one tool:
 *
 *   1. Enforce the 100 KB WASM size budget (SC-042 hard limit).
 *   2. Persist WASM sizes across releases in a baseline + rolling
 *      history, exposing a markdown summary for PR dashboards/comments.
 *
 * Files written:
 *   - apexchainx_calculator/.wasm-size.baseline.txt  (single-line current size)
 *   - apexchainx_calculator/.wasm-size.history.txt  (TSV: sha, size, source, ISO-date)
 *
 * Usage:
 *   tsx tools/wasm-size.ts                   # check budget + persist history
 *   tsx tools/wasm-size.ts summary           # emit markdown summary only
 *   tsx tools/wasm-size.ts --wasm=/path/x    # override wasm path
 *
 * Environment variables (consumed when present):
 *   GITHUB_SHA      — short SHA of the running build (history entry)
 *   GITHUB_REF_NAME — branch or ref name (history entry)
 *   GITHUB_EVENT_NAME — 'pull_request' / 'push' / etc.
 *
 * Exit codes:
 *   0   budget OK (or summary mode)
 *   1   WASM missing or over budget
 */

import * as fs from "fs";
import * as path from "path";

const DEFAULT_WASM = path.resolve(
  "apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm",
);
const BASELINE_FILE = path.resolve(
  "apexchainx_calculator/.wasm-size.baseline.txt",
);
const HISTORY_FILE = path.resolve(
  "apexchainx_calculator/.wasm-size.history.txt",
);
const HISTORY_HEADER =
  "# sha\tts\tsize_bytes\tsource\tevent\tiso_date\n";
const BUDGET_BYTES = 100 * 1024; // 100 KB — SC-042

type SizeEntry = {
  sha: string;
  ts: string;
  size: number;
  source: string;
  event: string;
  iso: string;
};

function formatKB(bytes: number): string {
  return `${(bytes / 1024).toFixed(2)} KB`;
}

function shortSha(sha: string): string {
  return sha.length > 7 ? sha.slice(0, 7) : sha;
}

function envOr(name: string, fallback: string): string {
  const v = process.env[name];
  return v && v.length > 0 ? v : fallback;
}

function ensureFile(p: string, initial: string): void {
  const dir = path.dirname(p);
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  if (!fs.existsSync(p)) fs.writeFileSync(p, initial, "utf8");
}

function readHistory(): SizeEntry[] {
  ensureFile(HISTORY_FILE, HISTORY_HEADER);
  // Strip CR to tolerate CRLF line endings from Windows checkouts
  // or editors that normalise LF → CRLF.
  const lines = fs
    .readFileSync(HISTORY_FILE, "utf8")
    .replace(/\r/g, "")
    .split("\n")
    .filter((l) => l && !l.startsWith("#"));
  return lines.map((line) => {
    const parts = line.split("\t");
    return {
      sha: parts[0] ?? "",
      ts: parts[1] ?? "",
      size: parseInt(parts[2] ?? "0", 10),
      source: parts[3] ?? "",
      event: parts[4] ?? "",
      iso: parts[5] ?? "",
    };
  });
}

function appendHistory(entry: SizeEntry): void {
  ensureFile(HISTORY_FILE, HISTORY_HEADER);
  fs.appendFileSync(
    HISTORY_FILE,
    `${entry.sha}\t${entry.ts}\t${entry.size}\t${entry.source}\t${entry.event}\t${entry.iso}\n`,
    "utf8",
  );
}

function writeBaseline(sizeBytes: number): void {
  ensureFile(BASELINE_FILE, "0\n");
  fs.writeFileSync(BASELINE_FILE, String(sizeBytes), "utf8");
}

function readBaseline(): number | null {
  try {
    const v = fs.readFileSync(BASELINE_FILE, "utf8").trim();
    if (!v) return null;
    return parseInt(v, 10);
  } catch {
    return null;
  }
}

type Args = { wasm: string; mode: "check" | "summary" };

function parseArgs(argv: string[]): Args {
  let wasm = DEFAULT_WASM;
  let mode: Args["mode"] = argv.includes("summary") ? "summary" : "check";
  for (const arg of argv) {
    if (arg.startsWith("--wasm=")) {
      wasm = path.resolve(arg.slice("--wasm=".length));
    }
  }
  return { wasm, mode };
}

function check(wasmPath: string): void {
  if (!fs.existsSync(wasmPath)) {
    console.error(`❌ WASM not found at ${wasmPath}`);
    console.error(
      "Hint: cd apexchainx_calculator && cargo build --target wasm32-unknown-unknown --release",
    );
    process.exit(1);
  }

  const size = fs.statSync(wasmPath).size;
  const baseline = readBaseline();

  console.log(`WASM size : ${formatKB(size)}`);
  console.log(`Budget    : ${formatKB(BUDGET_BYTES)}`);

  if (baseline !== null && baseline !== size) {
    const delta = size - baseline;
    const sign = delta >= 0 ? "+" : "";
    console.log(`Delta vs baseline: ${sign}${formatKB(delta)}`);
    if (delta > 0) {
      console.warn(
        "⚠️  Size increased since last baseline — update baseline if intentional.",
      );
    }
  } else if (baseline === null) {
    console.log("No baseline found — writing current size as baseline.");
  }

  if (size > BUDGET_BYTES) {
    console.error(
      `❌ WASM size ${formatKB(size)} exceeds budget ${formatKB(BUDGET_BYTES)}`,
    );
    console.error(
      "To raise the budget, update BUDGET_BYTES in tools/wasm-size.ts with a PR justification.",
    );
    process.exit(1);
  }

  console.log("✅ WASM size within budget.");

  // Persist new baseline + history entry.
  writeBaseline(size);
  const fullSha = envOr("GITHUB_SHA", "local");
  const sha = shortSha(fullSha);
  const source = envOr("GITHUB_REF_NAME", "local");
  const event = envOr("GITHUB_EVENT_NAME", "local");
  const now = new Date().toISOString();
  const ts = Date.now().toString();
  appendHistory({ sha, ts, size, source, event, iso: now });

  console.log(
    `📝 Recorded history: sha=${sha} source=${source} event=${event} size=${size}B`,
  );
}

function summary(): void {
  const entries = readHistory();
  if (entries.length === 0) {
    process.stdout.write("<!-- wasm-size-history: no entries yet -->\n");
    return;
  }
  const last = entries[entries.length - 1];
  const baseline = readBaseline();
  // Pretty-print last 5 entries, oldest first so the table reads chronologically.
  const start = Math.max(0, entries.length - 5);
  const window = entries.slice(start);

  process.stdout.write(
    [
      "### Build Size Trend",
      "",
      "| Commit | Time (UTC) | Source | Event | Size | Delta vs baseline |",
      "| --- | --- | --- | --- | ---: | ---: |",
      ...window.map((e) => {
        const delta = baseline !== null ? e.size - baseline : 0;
        const sign = delta >= 0 ? "+" : "";
        return `| \`${e.sha}\` | ${e.iso} | ${e.source} | ${e.event} | ${formatKB(e.size)} | ${sign}${formatKB(delta)} |`;
      }),
      "",
      `_Budget: ${formatKB(BUDGET_BYTES)} • Baseline: ${
        baseline !== null ? formatKB(baseline) : "n/a"
      } • Latest: ${formatKB(last.size)}_`,
      "",
    ].join("\n"),
  );
}

const { wasm, mode } = parseArgs(process.argv.slice(2));
if (mode === "summary") {
  summary();
} else {
  check(wasm);
}
