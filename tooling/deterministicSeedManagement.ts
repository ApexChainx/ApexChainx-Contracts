/**
 * SC-W5-072: Deterministic test seed management and replay in CI.
 * Stores and retrieves a fixed seed so fuzz/property runs are replayable.
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "fs";
import { join } from "path";

const SEED_FILE = ".ci-test-seed";
const DEFAULT_SEED = 42;
const FUZZ_CORPUS_BASE = "apexchainx_calculator/fuzz/corpus";
const FUZZ_ARTIFACTS_BASE = "apexchainx_calculator/fuzz/artifacts";

export function loadSeed(): number {
  if (existsSync(SEED_FILE)) {
    const raw = readFileSync(SEED_FILE, "utf8").trim();
    const parsed = parseInt(raw, 10);
    return isNaN(parsed) ? DEFAULT_SEED : parsed;
  }
  return DEFAULT_SEED;
}

export function saveSeed(seed: number): void {
  writeFileSync(SEED_FILE, String(seed), "utf8");
}

export function deterministicSequence(seed: number, length: number): number[] {
  let s = seed;
  return Array.from({ length }, () => {
    s = (s * 1664525 + 1013904223) & 0xffffffff;
    return Math.abs(s) % 10000;
  });
}

/**
 * Returns the fuzz seed file path for a given target.
 * CI records the seed used; replay uses the same seed.
 */
export function fuzzSeedFile(target: string): string {
  return join(FUZZ_CORPUS_BASE, target, ".seed");
}

/**
 * Load the persisted fuzz seed for a target, or generate a new one.
 */
export function loadFuzzSeed(target: string): number {
  const path = fuzzSeedFile(target);
  if (existsSync(path)) {
    const raw = readFileSync(path, "utf8").trim();
    const parsed = parseInt(raw, 10);
    return isNaN(parsed) ? Date.now() : parsed;
  }
  return Date.now();
}

/**
 * Persist the fuzz seed for a target so the run is reproducible.
 */
export function saveFuzzSeed(target: string, seed: number): void {
  const dir = join(FUZZ_CORPUS_BASE, target);
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  writeFileSync(fuzzSeedFile(target), String(seed), "utf8");
}

/**
 * List crash/slow artifact files for a given fuzz target.
 */
export function listFuzzArtifacts(target: string): string[] {
  const dir = join(FUZZ_ARTIFACTS_BASE, target);
  if (!existsSync(dir)) {
    return [];
  }
  return readdirSync(dir).filter((f) => !f.startsWith("."));
}

/**
 * List corpus files for a given fuzz target.
 */
export function listFuzzCorpus(target: string): string[] {
  const dir = join(FUZZ_CORPUS_BASE, target);
  if (!existsSync(dir)) {
    return [];
  }
  return readdirSync(dir).filter((f) => !f.startsWith("."));
}

/**
 * Generate a summary report for a fuzz run.
 */
export function fuzzRunSummary(target: string, seed: number): string {
  const corpus = listFuzzCorpus(target);
  const artifacts = listFuzzArtifacts(target);
  return [
    `Fuzz Target: ${target}`,
    `Seed: ${seed}`,
    `Corpus files: ${corpus.length}`,
    `Artifacts (crashes/slows): ${artifacts.length}`,
    artifacts.length > 0
      ? `Artifacts: ${artifacts.join(", ")}`
      : "No artifacts found",
  ].join("\n");
}

if (require.main === module) {
  const seed = loadSeed();
  console.log(`Using seed: ${seed}`);
  const seq = deterministicSequence(seed, 5);
  console.log(`Sample sequence: [${seq.join(", ")}]`);
  saveSeed(seed);
  console.log(`Seed saved to ${SEED_FILE} for CI replay.`);

  // Show fuzz status for each target
  const targets = ["compute_result", "validate_config"];
  for (const target of targets) {
    console.log(`\n--- ${target} ---`);
    console.log(fuzzRunSummary(target, seed));
  }
}
