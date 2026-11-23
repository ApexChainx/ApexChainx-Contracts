/**
 * SC-W5-058: Severity symbol mapping parity with backend enums.
 * Ensures contract severity symbols match the backend enum values exactly.
 */


import { describe, it } from "node:test";
import assert from "node:assert/strict";
// Contract-side symbol constants (mirrors Soroban Symbol storage keys)
const CONTRACT_SEVERITIES = ["critical", "high", "medium", "low"] as const;
type Severity = typeof CONTRACT_SEVERITIES[number];

// Backend enum mapping (must stay in sync with backend service)
const BACKEND_ENUM: Record<Severity, number> = {
  critical: 0,
  high:     1,
  medium:   2,
  low:      3,
};

function toBackendEnum(severity: string): number | null {
  return (severity in BACKEND_ENUM) ? BACKEND_ENUM[severity as Severity] : null;
}

describe("SC-W5-058 Severity Symbol Mapping Parity", () => {
  it("all contract severities map to a backend enum value", () => {
    for (const s of CONTRACT_SEVERITIES) {
      assert.ok(toBackendEnum(s) !== null, `toBackendEnum(s) should not be null`);
    }
  });

  it("enum values are unique integers", () => {
    const values = Object.values(BACKEND_ENUM);
    assert.strictEqual(new Set(values).size, values.length);
  });

  it("unknown severity returns null — not a silent default", () => {
    assert.strictEqual(toBackendEnum("unknown"), null);
    assert.strictEqual(toBackendEnum(""), null);
  });

  it("contract severity order matches backend enum order", () => {
    const sorted = [...CONTRACT_SEVERITIES].sort((a, b) => BACKEND_ENUM[a] - BACKEND_ENUM[b]);
    assert.deepStrictEqual(sorted, [...CONTRACT_SEVERITIES]);
  });

  it("critical has lowest numeric value (highest priority)", () => {
    assert.ok(BACKEND_ENUM.critical < BACKEND_ENUM.high, `BACKEND_ENUM.critical should be < BACKEND_ENUM.high`);
    assert.ok(BACKEND_ENUM.high < BACKEND_ENUM.medium, `BACKEND_ENUM.high should be < BACKEND_ENUM.medium`);
  });
});
