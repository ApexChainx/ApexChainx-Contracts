/**
 * SC-W5-064: Simulation snapshots for governance parameter changes.
 * Pins expected evaluation outcomes before and after a governance param update.
 */


import { describe, it } from "node:test";
import assert from "node:assert/strict";
interface GovParams { penaltyBps: number; maxMttrMinutes: number }
type Verdict = "met" | "violated" | "capped";

function evaluate(mttr: number, params: GovParams): Verdict {
  if (mttr > params.maxMttrMinutes) return "capped";
  return mttr <= 60 ? "met" : "violated";
}

const BEFORE_GOVERNANCE: GovParams = { penaltyBps: 300, maxMttrMinutes: 480 };
const AFTER_GOVERNANCE:  GovParams = { penaltyBps: 500, maxMttrMinutes: 240 };

describe("SC-W5-064 Simulation Snapshots — Governance Params", () => {
  it("before update: mttr=300 is violated (under cap)", () => {
    assert.strictEqual(evaluate(300, BEFORE_GOVERNANCE), "violated");
  });

  it("after update: mttr=300 is capped (over new maxMttr)", () => {
    assert.strictEqual(evaluate(300, AFTER_GOVERNANCE), "capped");
  });

  it("before update: mttr=60 is met", () => {
    assert.strictEqual(evaluate(60, BEFORE_GOVERNANCE), "met");
  });

  it("after update: mttr=60 is still met — cap only affects large values", () => {
    assert.strictEqual(evaluate(60, AFTER_GOVERNANCE), "met");
  });

  it("snapshots are stable across repeated calls", () => {
    assert.strictEqual(evaluate(300, BEFORE_GOVERNANCE), evaluate(300, BEFORE_GOVERNANCE));
    assert.strictEqual(evaluate(300, AFTER_GOVERNANCE), evaluate(300, AFTER_GOVERNANCE));
  });
});
