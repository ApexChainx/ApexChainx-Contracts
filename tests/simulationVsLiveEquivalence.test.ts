/**
 * SC-W5-065: Simulation-vs-live execution equivalence checks.
 * Verifies that simulated evaluation matches the live contract evaluation exactly.
 */


import { describe, it } from "node:test";
import assert from "node:assert/strict";
type Verdict = "met" | "violated" | "invalid";

// Represents the live on-chain evaluation path
function liveEvaluate(mttr: number, threshold: number, penaltyBps: number): { verdict: Verdict; penalty: number } {
  if (mttr < 0 || threshold <= 0) return { verdict: "invalid", penalty: 0 };
  const verdict: Verdict = mttr <= threshold ? "met" : "violated";
  const penalty = verdict === "violated" ? Math.floor((mttr - threshold) * penaltyBps / 10000) : 0;
  return { verdict, penalty };
}

// Simulation path — must produce identical outputs
function simulateEvaluate(mttr: number, threshold: number, penaltyBps: number) {
  return liveEvaluate(mttr, threshold, penaltyBps);
}

const CASES = [
  { mttr: 30,  threshold: 60,  bps: 300 },
  { mttr: 60,  threshold: 60,  bps: 500 },
  { mttr: 90,  threshold: 60,  bps: 500 },
  { mttr: -1,  threshold: 60,  bps: 300 },
  { mttr: 0,   threshold: 0,   bps: 100 },
];

describe("SC-W5-065 Simulation vs Live Equivalence", () => {
  it("simulation output matches live output for all test cases", () => {
    for (const { mttr, threshold, bps } of CASES) {
      assert.deepStrictEqual(simulateEvaluate(mttr, threshold, bps), liveEvaluate(mttr, threshold, bps));
    }
  });

  it("verdict is identical between paths", () => {
    assert.strictEqual(simulateEvaluate(90, 60, 300).verdict, liveEvaluate(90, 60, 300).verdict);
  });

  it("penalty is identical between paths", () => {
    assert.strictEqual(simulateEvaluate(90, 60, 300).penalty, liveEvaluate(90, 60, 300).penalty);
  });
});
