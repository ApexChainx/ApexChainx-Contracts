/**
 * SC-042: Enforce contract WASM size budget.
 * Fails CI if the built WASM exceeds the threshold; prints delta vs baseline.
 */

import * as fs from "fs";
import * as path from "path";

const WASM_PATH = path.resolve(
  "apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm"
);

// Hard budget in bytes. Raise via PR with documented justification.
const SIZE_BUDGET_BYTES = 100 * 1024; // 100 KB

// Baseline snapshot file — updated by maintainers on intentional size changes.
const BASELINE_FILE = path.resolve("apexchainx_calculator/.wasm-size-baseline");

function readBaseline(): number | null {
  try {
    return parseInt(fs.readFileSync(BASELINE_FILE, "utf8").trim(), 10);
  } catch {
    return null;
  }
}

function formatKB(bytes: number): string {
  return `${(bytes / 1024).toFixed(2)} KB`;
}

function checkSize(): void {
  if (!fs.existsSync(WASM_PATH)) {
    console.error(`❌ WASM not found at ${WASM_PATH}. Run: cargo build --target wasm32-unknown-unknown --release`);
    process.exit(1);
  }

  const { size } = fs.statSync(WASM_PATH);
  const baseline = readBaseline();

  console.log(`WASM size : ${formatKB(size)}`);
  console.log(`Budget    : ${formatKB(SIZE_BUDGET_BYTES)}`);

  if (baseline !== null) {
    const delta = size - baseline;
    const sign = delta >= 0 ? "+" : "";
    console.log(`Delta vs baseline: ${sign}${formatKB(delta)}`);
    if (delta > 0) console.warn("⚠️  Size increased since last baseline — update baseline if intentional.");
  } else {
    console.log("No baseline found — writing current size as baseline.");
    fs.writeFileSync(BASELINE_FILE, String(size));
  }

  if (size > SIZE_BUDGET_BYTES) {
    console.error(`❌ WASM size ${formatKB(size)} exceeds budget ${formatKB(SIZE_BUDGET_BYTES)}`);
    console.error("To raise the budget, update SIZE_BUDGET_BYTES in scripts/check-wasm-size.ts with a PR justification.");
    process.exit(1);
  }

  console.log("✅ WASM size within budget.");
}

checkSize();
