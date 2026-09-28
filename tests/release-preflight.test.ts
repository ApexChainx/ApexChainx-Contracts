import assert from "node:assert/strict";
import { test } from "node:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { assessFuzz, runPreflight, type FuzzRun, type Waiver } from "../scripts/release-preflight";

const now = Date.parse("2026-09-26T12:00:00Z");
const run: FuzzRun = { id: 123, head_sha: "a".repeat(40), head_branch: "main", event: "schedule", status: "completed", conclusion: "success", created_at: "2026-09-26T02:00:00Z", html_url: "https://github.com/org/repo/actions/runs/123" };
const sha = "b".repeat(40);
const waiver: Waiver = { gate: "nightly-fuzz", candidate: sha, runId: 123, approvedBy: "maintainer", reason: "Tracked crash with bounded impact", ticket: "https://github.com/org/repo/issues/674", expiresAt: "2026-09-27T00:00:00Z" };

test("nightly evidence fails closed on red, stale, absent, pending, or wrong source", () => {
  assert.equal(assessFuzz(run, sha, [], now), "PASS");
  for (const change of [ { conclusion: "failure" }, { created_at: "2026-09-20T00:00:00Z" }, { created_at: "bad" }, { status: "in_progress" }, { head_branch: "feature" }, { event: "pull_request" } ]) {
    assert.throws(() => assessFuzz({ ...run, ...change }, sha, [], now));
  }
  assert.throws(() => assessFuzz(undefined, sha, [waiver], now));
  assert.throws(() => assessFuzz(run, sha, [], now, 1), /triage debt/);
  assert.match(assessFuzz(run, sha, [waiver], now, 1), /^WAIVED:/);
});

test("waivers must bind this candidate and run, have review evidence, and remain unexpired", () => {
  const red = { ...run, conclusion: "failure" };
  assert.match(assessFuzz(red, sha, [waiver], now), /^WAIVED:/);
  for (const change of [ { candidate: "other" }, { runId: 999 }, { approvedBy: "" }, { reason: " " }, { ticket: "" }, { expiresAt: "2026-09-25T00:00:00Z" } ]) {
    assert.throws(() => assessFuzz(red, sha, [{ ...waiver, ...change }], now));
  }
});

const targets = ["compute_result", "validate_config", "config_mutation_sequences", "governance_sequences", "history_state_machine"];
interface Scenario {
  run?: FuzzRun | null;
  waivers?: Waiver[];
  artifacts?: { name: string; expired: boolean }[];
  debt?: number;
  fail?: string;
  dirtyFixtures?: string;
  generationFails?: boolean;
}

/** Synthetic external results; real orchestration and evidence-file writes. */
function scenario(input: Scenario = {}) {
  const parent = resolve("validation-results/acceptance-cases");
  mkdirSync(parent, { recursive: true });
  const directory = mkdtempSync(`${parent}/preflight-`);
  const waiverFile = `${directory}/input-waivers.json`;
  writeFileSync(waiverFile, JSON.stringify(input.waivers ?? []));
  const calls: string[] = [];
  const report = runPreflight({
    outputDirectory: `${directory}/output`, waiversFile: waiverFile, now,
    generateFixtures: () => {
      calls.push("cargo fixture generation");
      if (input.generationFails) throw new Error("fixture generator failed");
      return "fixture generation passed";
    },
    command: (program, args) => {
      const invocation = `${program} ${args.join(" ")}`;
      calls.push(invocation);
      if (input.fail && invocation.includes(input.fail)) throw new Error(`simulated failure: ${input.fail}`);
      if (program === "git") {
        if (args[0] === "rev-parse") return sha;
        if (args[0] === "status") return args.includes("ts/fixtures") ? input.dirtyFixtures ?? "" : "";
        if (args[0] === "merge-base") return "";
      }
      if (program === "gh") {
        if (args[0] === "repo") return JSON.stringify({ nameWithOwner: "org/repo" });
        if (invocation.includes("workflows/fuzz.yml/runs")) return JSON.stringify({ workflow_runs: input.run === null ? [] : [input.run ?? run] });
        if (invocation.includes("/artifacts?")) return JSON.stringify({ artifacts: input.artifacts ?? targets.map(target => ({ name: `fuzz-evidence-${target}`, expired: false })) });
        if (invocation.includes("/issues?")) return JSON.stringify([Array.from({ length: input.debt ?? 0 }, (_, i) => ({ title: `Fuzz crash: target ${i}`, html_url: `https://github.com/org/repo/issues/${i + 1}` }))]);
      }
      if (program === process.execPath) return `passed: ${args.join(" ")}`;
      throw new Error(`Unexpected command: ${invocation}`);
    },
  });
  const output = `${directory}/output`;
  assert.deepEqual(JSON.parse(readFileSync(`${output}/report.json`, "utf8")), report);
  const checklist = readFileSync(`${output}/release-ticket.md`, "utf8");
  for (const result of report.results) {
    assert.ok(existsSync(`${output}/${result.log}`), `${result.gate}: evidence missing`);
    assert.ok(checklist.includes(`${result.gate}: ${result.status}`), `${result.gate}: missing checklist entry`);
  }
  return { report, output, calls };
}

test("#674 clean preflight executes every safety gate and records reviewable proof", () => {
  const { report, output, calls } = scenario();
  assert.equal(report.results.length, 11);
  assert.ok(report.results.every(r => r.status === "PASS"));
  assert.ok(calls.some(c => c.includes("merge-base --is-ancestor")));
  assert.ok(calls.some(c => c.includes("--untracked-files=all -- ts/fixtures ts/generated")));
  for (const file of ["nightly-run.json", "nightly-artifacts.json", "fuzz-triage-debt.json", "waivers.json"]) {
    assert.ok(existsSync(`${output}/${file}`));
  }
});

test("#674 gate failures preserve all logs and do not skip independent checks", () => {
  for (const [fail, gate] of [
    ["tsc", "ts-typecheck"],
    ["run-ts-tests.ts", "ts-parity"],
    ["check-parity-coverage.ts", "parity-coverage"],
    ["eventSizeRegression", "offchain-eventSizeRegression"],
    ["readCostRegression", "offchain-readCostRegression"],
    ["pers-store/generate-fixtures", "persistence-freshness"],
  ]) {
    const { report } = scenario({ fail });
    assert.equal(report.results.length, 11);
    assert.equal(report.results.find(r => r.gate === gate)?.status, "FAIL");
    assert.equal(report.results.filter(r => r.status === "FAIL").length, 1);
  }
});

test("#674 regeneration errors, tracked drift and newly generated files fail freshness", () => {
  for (const input of [{ generationFails: true }, { dirtyFixtures: " M ts/generated/contractConstants.ts" }, { dirtyFixtures: "?? ts/fixtures/new.json" }]) {
    const { report } = scenario(input);
    assert.equal(report.results.find(r => r.gate === "fixture-freshness")?.status, "FAIL");
  }
});

test("#674 latest red/missing evidence, API errors, unrelated history and triage debt fail closed", () => {
  for (const input of [
    { run: null }, { run: { ...run, conclusion: "failure" } }, { debt: 1 },
    { fail: "workflows/fuzz.yml/runs" }, { fail: "merge-base" },
    { artifacts: [] },
    { artifacts: targets.map(target => ({ name: `fuzz-evidence-${target}`, expired: true })) },
    { artifacts: targets.slice(1).map(target => ({ name: `fuzz-evidence-${target}`, expired: false })) },
  ]) {
    const { report } = scenario(input);
    assert.equal(report.results.find(r => r.gate === "nightly-fuzz")?.status, "FAIL");
  }
});

test("#674 an exact reviewed waiver is visibly tracked and cannot waive parity failures", () => {
  const { report, output } = scenario({ run: { ...run, conclusion: "failure" }, waivers: [waiver], fail: "run-ts-tests.ts" });
  assert.equal(report.results.find(r => r.gate === "nightly-fuzz")?.status, "WAIVED");
  assert.equal(report.results.find(r => r.gate === "ts-parity")?.status, "FAIL");
  assert.deepEqual(JSON.parse(readFileSync(`${output}/waivers.json`, "utf8")), [waiver]);
  assert.match(readFileSync(`${output}/nightly-fuzz.log`, "utf8"), /maintainer.*expires.*Tracked crash/);
});

test("#674 workflow retains failed preflight evidence and hash recipes depend on the gate", () => {
  const workflow = readFileSync(".github/workflows/release-validation.yml", "utf8");
  assert.match(workflow, /run: npm run release:preflight/);
  assert.match(workflow, /name: Upload validation report\s+if: always\(\)[\s\S]*validation-results\//);
  assert.match(workflow, /name: Enforce release preflight[\s\S]*steps\.preflight\.outcome != 'success'[\s\S]*exit 1/);
  assert.match(workflow, /name: Record validation outcomes\s+if: always\(\)/);
  assert.match(workflow, /steps\.get\(key, \{\}\)\.get\("outcome", "skipped"\)/);
  const recipes = readFileSync("justfile", "utf8");
  for (const recipe of ["hash", "hash-save", "hash-verify"]) {
    assert.match(recipes, new RegExp(`^${recipe}: release-preflight wasm-release$`, "m"));
  }
});


test("#674 a waiver cannot substitute feature/manual evidence for the nightly", () => {
  for (const change of [{ event: "workflow_dispatch" }, { head_branch: "feature" }]) {
    assert.throws(() => assessFuzz({ ...run, ...change }, sha, [waiver], now), /cannot be waived/);
  }
});
