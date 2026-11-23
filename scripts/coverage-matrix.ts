/**
 * Cross-language coverage matrix (#632).
 *
 * The behaviour contract of this repository is split across two languages:
 * the Rust contract lives in `apexchainx_calculator/` and its mirrored read
 * semantics live in TypeScript (`ts/`, `tests/`, `offchain/`). Each side has
 * its own runner and its own expectations, so it is possible for both tool
 * chains to pass while their models disagree — a method covered in Rust but
 * never touched by TypeScript (or vice versa) is invisible until it bit a
 * consumer.
 *
 * This script is the orchestration layer over the four targets:
 *
 *   rust          unit/integration tests in `apexchainx_calculator/src`
 *                (modules test that the contract's own methods behave as
 *                 documented)
 *   ts-property   the property/regression matrix in `tests/*.test.ts`
 *   ts-parity     the parity suite in `ts/parity/` plus the `ts/` mirrors it
 *                 replays against contract-derived fixtures
 *   offchain      the off-chain integration probes in `offchain/*.ts`
 *
 * It reports which methods of the public API surface are exercised on each
 * target and FAILS (in `--strict` mode, used by CI) when a method is covered
 * on one side of the language boundary but not the other.
 *
 * Usage:
 *   tsx scripts/coverage-matrix.ts             # print the matrix + gap summary
 *   tsx scripts/coverage-matrix.ts --strict    # exit 1 on any gap
 *   tsx scripts/coverage-matrix.ts --out FILE  # also write a markdown report
 */

import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

// ─── Configuration ─────────────────────────────────────────────────────────

const REPO_ROOT = resolve(process.cwd());
const RUST_SRC = join(REPO_ROOT, "apexchainx_calculator", "src");
const TS_PROPERTY = join(REPO_ROOT, "tests");
const TS_SURFACE = join(REPO_ROOT, "ts");
const OFFCHAIN = join(REPO_ROOT, "offchain");

const STRICT = process.argv.includes("--strict");
const OUT_ARG = process.argv.indexOf("--out");
const OUT_FILE = OUT_ARG >= 0 && process.argv[OUT_ARG + 1] ? process.argv[OUT_ARG + 1] : undefined;

/**
 * Feature grouping for the matrix. `METHODS` is derived from the contract's
 * own `get_public_api` descriptor (see `deriveMethods` below), so a method
 * added to the ABI that is not registered here is reported as a MATRIX DRIFT
 * gap instead of silently disappearing — the registry cannot rot unnoticed.
 */
const FEATURES: { feature: string; methods: string[] }[] = [
  {
    feature: "Lifecycle",
    methods: ["initialize", "migrate", "healthcheck"],
  },
  {
    feature: "Configuration",
    methods: [
      "get_config",
      "get_config_count",
      "get_config_snapshot",
      "get_config_bundle",
      "get_config_version_hash",
      "get_custom_config_snapshot",
      "get_custom_severity",
      "set_config",
      "set_custom_severity",
      "remove_custom_severity",
      "list_configs",
      "freeze_config",
      "unfreeze_config",
      "is_config_frozen",
      "get_last_config_update",
    ],
  },
  {
    feature: "Calculation",
    methods: ["calculate_sla", "calculate_sla_view", "replay_calculate_sla", "get_economic_exposure"],
  },
  {
    feature: "History",
    methods: [
      "get_history",
      "get_history_by_outage",
      "get_history_page",
      "get_history_page_with_meta",
      "get_latest_by_outage",
      "prune_history",
      "prune_history_by_age",
    ],
  },
  {
    feature: "Governance & Roles",
    methods: [
      "get_admin",
      "get_operator",
      "get_pending_admin",
      "get_pending_operator",
      "propose_admin",
      "accept_admin",
      "cancel_admin_proposal",
      "propose_operator",
      "accept_operator",
      "cancel_operator_proposal",
      "set_operator",
      "renounce_admin",
    ],
  },
  {
    feature: "Pause & Status",
    methods: ["is_paused", "pause", "unpause", "get_pause_info"],
  },
  {
    feature: "Retention & Budget",
    methods: ["set_retention_limit", "get_retention_limit", "get_retention_metrics", "get_full_audit_state"],
  },
  {
    feature: "Telemetry",
    methods: ["get_stats", "get_severity_telemetry"],
  },
  {
    feature: "Metadata & Negotiation",
    methods: [
      "get_contract_info",
      "get_contract_metadata",
      "get_contract_state_fingerprint",
      "get_public_api",
      "get_result_schema",
      "get_failure_schema",
      "get_storage_version",
      "get_version_info",
      "get_version_negotiation_info",
      "get_rent_estimate",
      "get_storage_footprint_estimate",
      "get_migration_state",
    ],
  },
];

// ─── File collection ───────────────────────────────────────────────────────

function walk(dir: string, out: string[] = []): string[] {
  if (!existsSync(dir)) return out;
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      walk(full, out);
    } else {
      out.push(full);
    }
  }
  return out;
}

function fileContents(files: string[]): string {
  return files.map((f) => readFileSync(f, "utf8")).join("\n");
}

/**
 * The contract's own public API surface, derived from the `method("...")`
 * records that back `get_public_api()`. This is the single source of truth for
 * "what the behaviour contract contains", so the matrix never guesses.
 */
function deriveMethods(): string[] {
  const lib = readFileSync(join(RUST_SRC, "lib.rs"), "utf8");
  const matches = [...lib.matchAll(/method\("([a-z0-9_]+)"/g)].map((m) => m[1]);
  return [...new Set(matches)].sort();
}

// ─── Target coverage ───────────────────────────────────────────────────────

type Row = {
  method: string;
  rust: boolean;
  tsProperty: boolean;
  tsParity: boolean;
  offchain: boolean;
  registered: boolean;
};

function wordBoundary(method: string): RegExp {
  // The contract names methods in snake_case (Rust) while the TS mirrors use
  // the idiomatic camelCase (get_latest_by_outage -> getLatestByOutage), so a
  // row matches either spelling.
  const camel = method.replace(/_([a-z])/g, (_, c: string) => c.toUpperCase());
  return new RegExp(`\\b${method}\\b|\\b${camel}\\b`);
}

function collectTargets(): { rustFiles: string[]; propertyFiles: string[]; tsFiles: string[]; offchainFiles: string[] } {
  const rustFiles = walk(RUST_SRC).filter((f) => /_tests?\.rs$|tests\.rs$/.test(f));
  const propertyFiles = walk(TS_PROPERTY).filter((f) => f.endsWith(".test.ts"));
  const tsFiles = walk(TS_SURFACE).filter((f) => !f.includes("/fixtures/") && !f.includes("/generated/"));
  const offchainFiles = walk(OFFCHAIN).filter((f) => f.endsWith(".ts"));
  return { rustFiles, propertyFiles, tsFiles, offchainFiles };
}

function buildRows(): { rows: Row[]; drift: string[] } {
  const { rustFiles, propertyFiles, tsFiles, offchainFiles } = collectTargets();
  const rustContent = fileContents(rustFiles);
  const propertyContent = fileContents(propertyFiles);
  const tsContent = fileContents(tsFiles);
  const offchainContent = fileContents(offchainFiles);

  const registered = new Set(FEATURES.flatMap((f) => f.methods));
  const methods = deriveMethods();
  const drift = methods.filter((m) => !registered.has(m));

  const rows: Row[] = methods.map((method) => ({
    method,
    rust: wordBoundary(method).test(rustContent),
    tsProperty: wordBoundary(method).test(propertyContent),
    tsParity: wordBoundary(method).test(tsContent),
    offchain: wordBoundary(method).test(offchainContent),
    registered: registered.has(method),
  }));

  return { rows, drift };
}

function verdict(row: Row): { label: string; gap: boolean } {
  const anyTs = row.tsProperty || row.tsParity || row.offchain;
  if (!row.rust && !anyTs) return { label: "UNCOVERED", gap: true };
  if (row.rust && !anyTs) return { label: "RUST-ONLY GAP", gap: true };
  if (!row.rust && anyTs) return { label: "TS-ONLY GAP", gap: true };
  return { label: "OK", gap: false };
}

// ─── Reporting ─────────────────────────────────────────────────────────────

function render(rows: Row[], drift: string[]): string {
  const header =
    "| method | rust | ts-property | ts-parity | offchain | verdict |\n" +
    "| ------ | ---- | ----------- | --------- | -------- | ------- |";
  const body = rows
    .map((r) => {
      const v = verdict(r);
      return `| ${r.method} | ${r.rust ? "x" : ""} | ${r.tsProperty ? "x" : ""} | ${r.tsParity ? "x" : ""} | ${r.offchain ? "x" : ""} | ${v.label} |`;
    })
    .join("\n");

  const gaps = rows.filter((r) => verdict(r).gap);
  const summary = [
    `\n## Summary`,
    `Public API methods: **${rows.length}** · uncovered/gapped: **${gaps.length}** · drift: **${drift.length}**`,
    drift.map((m) => `- \`${m}\` missing from the matrix feature registry — add it to \`FEATURES\` in \`scripts/coverage-matrix.ts\``),
  ]
    .filter((s) => s)
    .flat();

  return `# Cross-Language Coverage Matrix (#632)\n\n` + `${header}\n${body}\n` + summary.join("\n") + "\n";
}

const { rows, drift } = buildRows();

const table = render(rows, drift);
const gapCount = rows.filter((r) => verdict(r).gap).length + (STRICT ? drift.length : 0);

if (OUT_FILE) {
  writeFileSync(resolve(process.cwd(), OUT_FILE), table, "utf8");
  process.stdout.write(`Coverage matrix written to ${OUT_FILE}\n`);
} else {
  process.stdout.write(table + "\n");
}

if (STRICT && gapCount > 0) {
  process.stderr.write(
    `coverage-matrix: ${gapCount} gap(s) — cross-language coverage is not complete (see matrix above).\n`,
  );
  process.exitCode = 1;
}