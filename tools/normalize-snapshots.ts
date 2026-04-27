// SC-048: Normalize history snapshot artifacts for low-noise PR review
// Strips non-semantic fields and sorts keys so snapshot diffs reflect real changes only.

import * as fs from "fs";
import * as path from "path";

const SNAPSHOT_DIR = path.resolve(__dirname, "../apexchainx_calculator/test_snapshots/tests");

const VOLATILE_KEYS = new Set(["timestamp", "elapsed_ms", "generated_at"]);

function normalizeValue(val: unknown): unknown {
  if (Array.isArray(val)) return val.map(normalizeValue);
  if (val !== null && typeof val === "object") return normalizeObject(val as Record<string, unknown>);
  return val;
}

function normalizeObject(obj: Record<string, unknown>): Record<string, unknown> {
  return Object.keys(obj)
    .filter((k) => !VOLATILE_KEYS.has(k))
    .sort()
    .reduce<Record<string, unknown>>((acc, k) => {
      acc[k] = normalizeValue(obj[k]);
      return acc;
    }, {});
}

function normalizeSnapshot(filePath: string): void {
  const raw = fs.readFileSync(filePath, "utf8");
  const parsed = JSON.parse(raw);
  const normalized = normalizeValue(parsed);
  const out = JSON.stringify(normalized, null, 2) + "\n";
  if (out !== raw) {
    fs.writeFileSync(filePath, out);
    console.log(`normalized: ${path.basename(filePath)}`);
  } else {
    console.log(`unchanged:  ${path.basename(filePath)}`);
  }
}

function run() {
  if (!fs.existsSync(SNAPSHOT_DIR)) {
    console.log("No snapshot directory found — nothing to normalize.");
    return;
  }
  const files = fs.readdirSync(SNAPSHOT_DIR).filter((f) => f.endsWith(".json"));
  if (files.length === 0) {
    console.log("No snapshot files found.");
    return;
  }
  files.forEach((f) => normalizeSnapshot(path.join(SNAPSHOT_DIR, f)));
  console.log(`\nDone. ${files.length} snapshot(s) processed.`);
}

run();
