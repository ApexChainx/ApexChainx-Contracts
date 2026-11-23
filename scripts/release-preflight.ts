/** Release safety gate. Every check writes evidence, including failed checks. */
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export interface FuzzRun {
  id: number;
  head_sha: string;
  head_branch: string;
  event: string;
  status: string;
  conclusion: string | null;
  created_at: string;
  html_url: string;
}
export interface Waiver {
  gate: "nightly-fuzz";
  candidate: string;
  runId: number;
  reason: string;
  approvedBy: string;
  ticket: string;
  expiresAt: string;
}

/** Only a recent, completed successful nightly is clean. Never search for an older green run. */
export function assessFuzz(run: FuzzRun | undefined, candidate: string, waivers: Waiver[], now = Date.now(), openCrashes = 0): string {
  if (!run) throw new Error("No nightly fuzz run; missing evidence cannot be waived");
  if (run.event !== "schedule" || run.head_branch !== "main")
    throw new Error("Only scheduled main evidence is eligible; wrong-source runs cannot be waived");
  const age = now - Date.parse(run.created_at);
  const clean = run.event === "schedule" && run.head_branch === "main" &&
    run.status === "completed" && run.conclusion === "success" &&
    Number.isFinite(age) && age >= 0 && age <= 48 * 60 * 60 * 1000 && openCrashes === 0;
  if (clean) return "PASS";
  const waiver = waivers.find(w => w.gate === "nightly-fuzz" &&
    w.candidate === candidate && w.runId === run.id && w.reason?.trim() &&
    w.approvedBy?.trim() && /^https:\/\/github\.com\/[^/]+\/[^/]+\/(issues|pull)\/\d+$/.test(w.ticket) &&
    Date.parse(w.expiresAt) > now);
  if (waiver) return `WAIVED: ${waiver.ticket} (${waiver.approvedBy}), expires ${waiver.expiresAt}: ${waiver.reason}`;
  throw new Error(`Latest nightly ${run.id} is missing freshness/success or has triage debt: ${run.status}/${run.conclusion}, ${openCrashes} open crashes; an exact-candidate, exact-run reviewed waiver is required`);
}

export interface PreflightOptions {
  outputDirectory?: string;
  waiversFile?: string;
  now?: number;
  command?: (program: string, args: string[]) => string;
  generateFixtures?: () => string;
}

/** The same orchestration is used by the CLI and by controlled acceptance tests. */
export function runPreflight(options: PreflightOptions = {}) {
  const output = resolve(options.outputDirectory ?? "validation-results/preflight");
  mkdirSync(output, { recursive: true });
  for (const file of ["nightly-run.json", "nightly-artifacts.json", "fuzz-triage-debt.json", "waivers.json"]) {
    rmSync(resolve(output, file), { force: true });
  }
  const command = options.command ?? ((program: string, args: string[]) => execFileSync(program, args, {
    encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], maxBuffer: 32 * 1024 * 1024,
  }));
  const candidate = command("git", ["rev-parse", "HEAD"]).trim();
  const results: { gate: string; status: string; log: string }[] = [];
  function check(gate: string, fn: () => string) {
    let status = "PASS", log: string;
    try { log = fn(); if (log.startsWith("WAIVED:")) status = "WAIVED"; }
    catch (error) {
      const e = error as Error & { stdout?: string; stderr?: string };
      status = "FAIL"; log = `${e.message}\n${e.stdout ?? ""}\n${e.stderr ?? ""}`;
    }
    writeFileSync(`${output}/${gate}.log`, log);
    results.push({ gate, status, log: `${gate}.log` });
    console.log(`${gate}: ${status}`);
  }
  check("candidate-clean", () => {
    const dirty = command("git", ["status", "--porcelain", "--untracked-files=all"]);
    if (dirty.trim()) throw new Error(`Candidate SHA does not describe the working tree; commit changes before release:\n${dirty}`);
    return candidate;
  });
  check("nightly-fuzz", () => {
    const repository = process.env.GITHUB_REPOSITORY ?? JSON.parse(command("gh", ["repo", "view", "--json", "nameWithOwner"])).nameWithOwner;
    const runs = JSON.parse(command("gh", ["api", `repos/${repository}/actions/workflows/fuzz.yml/runs?branch=main&event=schedule&per_page=1`]));
    const run: FuzzRun | undefined = runs.workflow_runs[0];
    writeFileSync(`${output}/nightly-run.json`, JSON.stringify(runs, null, 2));
    if (!run) throw new Error("No nightly fuzz evidence available");
    // A run on unrelated history is not evidence for this candidate, even with a waiver.
    command("git", ["merge-base", "--is-ancestor", run.head_sha, candidate]);
    const artifacts = JSON.parse(command("gh", ["api", `repos/${repository}/actions/runs/${run.id}/artifacts?per_page=100`]));
    writeFileSync(`${output}/nightly-artifacts.json`, JSON.stringify(artifacts, null, 2));
    const issuePages = JSON.parse(command("gh", ["api", "--paginate", "--slurp", `repos/${repository}/issues?state=open&per_page=100`]));
    const crashes = issuePages.flat().filter((i: { title: string; pull_request?: unknown }) => !i.pull_request && i.title.startsWith("Fuzz crash:"));
    writeFileSync(`${output}/fuzz-triage-debt.json`, JSON.stringify(crashes, null, 2));
    const waivers: Waiver[] = JSON.parse(readFileSync(options.waiversFile ?? process.env.RELEASE_WAIVERS_FILE ?? "release/waivers.json", "utf8"));
    writeFileSync(`${output}/waivers.json`, JSON.stringify(waivers.filter(w => w.candidate === candidate && w.runId === run.id), null, 2));
    const status = assessFuzz(run, candidate, waivers, options.now ?? Date.now(), crashes.length);
    const targets = ["compute_result", "validate_config", "config_mutation_sequences", "governance_sequences", "history_state_machine"];
    if (status === "PASS" && targets.some(target => !artifacts.artifacts.some((a: { name: string; expired: boolean }) => a.name === `fuzz-evidence-${target}` && !a.expired))) {
      throw new Error("Successful nightly is missing retained evidence for one or more fuzz targets");
    }
    return `${status}\nRun: ${run.html_url}\nCommit: ${run.head_sha}\nOpen crash issues: ${crashes.length}\n`;
  });
  check("fixture-freshness", () => {
    const generate = options.generateFixtures ?? (() => {
      const result = spawnSync("cargo", ["test", "--locked", "--lib", "ts_parity_fixtures"], { encoding: "utf8", maxBuffer: 32 * 1024 * 1024 });
      const log = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
      if (result.error || result.status !== 0) throw new Error(`${result.error ?? "Rust fixture generation failed"}\n${log}`);
      return log;
    });
    const log = generate();
    const dirty = command("git", ["status", "--porcelain", "--untracked-files=all", "--", "ts/fixtures", "ts/generated"]);
    if (dirty.trim()) throw new Error(`Generated artifacts are stale or uncommitted:\n${dirty}\n${log}`);
    return log;
  });
  const require = createRequire(import.meta.url);
  const tsx = (args: string[]) => command(process.execPath, [require.resolve("tsx/cli"), ...args]);
  check("parity-coverage", () => tsx(["scripts/check-parity-coverage.ts"]));
  check("ts-typecheck", () => command(process.execPath, [resolve(require.resolve("typescript/package.json"), "../bin/tsc"), "--noEmit"]));
  check("ts-parity", () => tsx(["scripts/run-ts-tests.ts", "parity"]));
  for (const name of ["eventSizeRegression", "readCostRegression", "governanceConsistency", "contractMetadata"]) {
    check(`offchain-${name}`, () => tsx([`offchain/${name}.ts`]));
  }
  check("persistence-freshness", () => tsx(["pers-store/generate-fixtures.ts"]));
  const report = { candidate, generatedAt: new Date(options.now ?? Date.now()).toISOString(), results };
  writeFileSync(`${output}/report.json`, JSON.stringify(report, null, 2));
  const runUrl = process.env.GITHUB_RUN_ID ? `https://github.com/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}` : "Local validation-results/preflight directory";
  writeFileSync(`${output}/release-ticket.md`, [
    "# Release preflight evidence", "", `Candidate: ${candidate}`, `Evidence: ${runUrl}`, "",
    ...results.map(r => `- [${r.status === "FAIL" ? " " : "x"}] ${r.gate}: ${r.status} — ${r.log}`),
    "", "Attach the release-validation-report artifact and this checklist to the release ticket.",
    "Review nightly-run.json, nightly-artifacts.json and waivers.json; a hash alone does not approve release.",
  ].join("\n") + "\n");
  return report;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const report = runPreflight();
  if (report.results.some(r => r.status === "FAIL")) process.exitCode = 1;
}
