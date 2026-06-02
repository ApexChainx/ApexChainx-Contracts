/**
 * SC-W5-072: Deterministic test seed management and replay in CI.
 * Stores and retrieves a fixed seed so fuzz/property runs are replayable.
 */

import { existsSync, readFileSync, writeFileSync } from "fs";

const SEED_FILE = ".ci-test-seed";
const DEFAULT_SEED = 42;

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

if (require.main === module) {
  const seed = loadSeed();
  console.log(`Using seed: ${seed}`);
  const seq = deterministicSequence(seed, 5);
  console.log(`Sample sequence: [${seq.join(", ")}]`);
  saveSeed(seed);
  console.log(`Seed saved to ${SEED_FILE} for CI replay.`);
}
