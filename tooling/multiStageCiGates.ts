/**
 * SC-W5-071: Multi-stage CI gates for unit, property, and integration tests.
 * Defines and validates the required test stages before a merge is allowed.
 */

import { execSync } from "child_process";

interface CiGate { name: string; cmd: string; required: boolean }
interface GateResult { gate: string; passed: boolean; note?: string }

const CI_GATES: CiGate[] = [
  { name: "unit-tests",        cmd: "cargo test --lib",          required: true },
  { name: "property-tests",    cmd: "cargo test property",       required: true },
  { name: "integration-tests", cmd: "cargo test --test '*'",     required: true },
];

function runGate(gate: CiGate): GateResult {
  try {
    execSync(gate.cmd, { stdio: "pipe" });
    return { gate: gate.name, passed: true };
  } catch (e: any) {
    return { gate: gate.name, passed: false, note: e.message?.slice(0, 120) };
  }
}

export function runAllGates(gates = CI_GATES): GateResult[] {
  return gates.map(runGate);
}

export function assertAllPassed(results: GateResult[]): void {
  const failed = results.filter((r) => r.passed === false);
  results.forEach((r) => console.log(`${r.passed ? "✓" : "✗"} ${r.gate}${r.note ? ` — ${r.note}` : ""}`));
  if (failed.length) { console.error(`\n${failed.length} gate(s) failed.`); process.exit(1); }
  console.log("\nAll CI gates passed.");
}
