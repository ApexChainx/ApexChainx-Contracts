/**
 * SC-W5-070: Long-run fuzz campaign CI integration with shrinking artifacts.
 * Prints a fuzz config suitable for CI and checks for saved shrink artifacts.
 */

import { existsSync, readdirSync } from "fs";
import { execSync } from "child_process";

const FUZZ_CORPUS_DIR = "sla_calculator/fuzz/corpus";
const FUZZ_ARTIFACTS_DIR = "sla_calculator/fuzz/artifacts";
const MAX_FUZZ_SECONDS = 60;

interface FuzzCiResult { passed: boolean; note: string }

function checkFuzzArtifacts(): FuzzCiResult {
  if (!existsSync(FUZZ_ARTIFACTS_DIR)) {
    return { passed: true, note: "No artifacts dir — no crashes recorded." };
  }
  const files = readdirSync(FUZZ_ARTIFACTS_DIR).filter((f) => !f.startsWith("."));
  if (files.length > 0) {
    return { passed: false, note: `${files.length} shrink artifact(s) present: ${files.join(", ")}` };
  }
  return { passed: true, note: "Artifacts dir is clean." };
}

function printFuzzConfig(): void {
  console.log("Fuzz CI config:");
  console.log(`  corpus:    ${FUZZ_CORPUS_DIR}`);
  console.log(`  artifacts: ${FUZZ_ARTIFACTS_DIR}`);
  console.log(`  max_time:  ${MAX_FUZZ_SECONDS}s`);
}

const result = checkFuzzArtifacts();
printFuzzConfig();
console.log(`\nArtifact check: ${result.passed ? "PASS" : "FAIL"} — ${result.note}`);
if (!result.passed) process.exit(1);
