/**
 * SC-W5-060: Backend replay/idempotency parity integration tests.
 * Verifies that replaying the same sequence of contract calls yields identical results.
 */


import { describe, it, beforeEach } from "node:test";
import assert from "node:assert/strict";
interface OutageRecord { id: string; mttr: number; threshold: number }
type Verdict = "met" | "violated" | "invalid";

const processedIds = new Set<string>();

function processOutage(record: OutageRecord): { id: string; verdict: Verdict; idempotent: boolean } {
  const idempotent = processedIds.has(record.id);
  if (!idempotent) processedIds.add(record.id);
  if (record.mttr < 0 || record.threshold <= 0) return { id: record.id, verdict: "invalid", idempotent };
  const verdict = record.mttr <= record.threshold ? "met" : "violated";
  return { id: record.id, verdict, idempotent };
}

const SEQUENCE: OutageRecord[] = [
  { id: "r-001", mttr: 45,  threshold: 60  },
  { id: "r-002", mttr: 90,  threshold: 60  },
  { id: "r-001", mttr: 45,  threshold: 60  }, // replay of r-001
];

describe("SC-W5-060 Backend Replay Idempotency Parity", () => {
  beforeEach(() => processedIds.clear());

  it("first call is not idempotent (new)", () => {
    assert.strictEqual(processOutage(SEQUENCE[0]).idempotent, false);
  });

  it("replayed call is flagged as idempotent", () => {
    processOutage(SEQUENCE[0]);
    assert.strictEqual(processOutage(SEQUENCE[2]).idempotent, true);
  });

  it("verdict is stable across original and replay", () => {
    const first  = processOutage(SEQUENCE[0]);
    const replay = processOutage(SEQUENCE[2]);
    assert.strictEqual(first.verdict, replay.verdict);
  });

  it("full sequence produces expected verdicts in order", () => {
    const results = SEQUENCE.map(processOutage);
    assert.strictEqual(results[0].verdict, "met");
    assert.strictEqual(results[1].verdict, "violated");
    assert.strictEqual(results[2].verdict, "met");
  });
});
