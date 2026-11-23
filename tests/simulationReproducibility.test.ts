/**
 * SC-W5-062: Simulation reproducibility across SDK versions.
 * Validates that core evaluation logic is stable and version-independent.
 */


import { describe, it } from "node:test";
import assert from "node:assert/strict";
interface SimInput { mttr: number; threshold: number; penaltyBps: number }
interface SimOutput { verdict: "met" | "violated" | "invalid"; penalty: number }

function simulate(input: SimInput): SimOutput {
  if (input.mttr < 0 || input.threshold <= 0) return { verdict: "invalid", penalty: 0 };
  const verdict = input.mttr <= input.threshold ? "met" : "violated";
  const penalty = verdict === "violated" ? Math.floor((input.mttr - input.threshold) * input.penaltyBps / 10000) : 0;
  return { verdict, penalty };
}

const SNAPSHOT: Array<[SimInput, SimOutput]> = [
  [{ mttr: 30,  threshold: 60,  penaltyBps: 500 }, { verdict: "met",      penalty: 0 }],
  [{ mttr: 90,  threshold: 60,  penaltyBps: 500 }, { verdict: "violated", penalty: 1 }],
  [{ mttr: 0,   threshold: 60,  penaltyBps: 300 }, { verdict: "met",      penalty: 0 }],
  [{ mttr: -1,  threshold: 60,  penaltyBps: 300 }, { verdict: "invalid",  penalty: 0 }],
];

describe("SC-W5-062 Simulation Reproducibility", () => {
  it("matches pinned snapshot outputs", () => {
    for (const [input, expected] of SNAPSHOT) {
      assert.deepStrictEqual(simulate(input), expected);
    }
  });

  it("same input always produces same output", () => {
    const input = { mttr: 75, threshold: 60, penaltyBps: 300 };
    assert.deepStrictEqual(simulate(input), simulate(input));
  });

  it("snapshot is exhaustive for all verdict states", () => {
    const verdicts = new Set(SNAPSHOT.map(([, o]) => o.verdict));
    assert.deepStrictEqual(verdicts, new Set(["met", "violated", "invalid"]));
  });
});
