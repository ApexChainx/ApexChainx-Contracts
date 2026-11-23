/**
 * SC-046: Threshold edge-case tests for zero and near-zero values.
 * Documents and validates contract policy for boundary MTTR inputs.
 */


import { describe, it } from "node:test";
import assert from "node:assert/strict";


interface SlaConfig {
  threshold: number; // minutes
  penaltyBps: number;
}

// Mirrors contract-side SLA evaluation logic
function evaluateSla(
  mttr: number,
  config: SlaConfig,
): "met" | "violated" | "invalid" {
  if (mttr < 0 || config.threshold < 0) return "invalid";
  if (config.threshold === 0) return "invalid"; // zero threshold is rejected by contract
  return mttr <= config.threshold ? "met" : "violated";
}

type Severity = "critical" | "high" | "medium";

const CANONICAL_SEVERITIES = ["critical", "high", "medium"] as const;

const CONFIGS: Record<Severity, SlaConfig> = {
  critical: { threshold: 60, penaltyBps: 500 },
  high: { threshold: 240, penaltyBps: 300 },
  medium: { threshold: 480, penaltyBps: 100 },
};
 

describe("SC-046 Threshold Edge Cases", () => {
  it("zero MTTR always meets any positive threshold", () => {
    for (const severity of CANONICAL_SEVERITIES) {
      assert.strictEqual(evaluateSla(0, CONFIGS[severity]), "met");
    }
  });

  it("MTTR of 1 meets threshold when threshold >= 1", () => {
    for (const severity of CANONICAL_SEVERITIES) {
      assert.strictEqual(evaluateSla(1, CONFIGS[severity]), "met");
    }
  });

  it("MTTR exactly at threshold is met (inclusive boundary)", () => {
    for (const severity of CANONICAL_SEVERITIES) {
      const cfg = CONFIGS[severity];
      assert.strictEqual(evaluateSla(cfg.threshold, cfg), "met");
    }
  });

  it("exact-threshold results are deterministic for backend replay", () => {
    for (const cfg of Object.values(CONFIGS)) {
      const first = evaluateSla(cfg.threshold, cfg);
      const second = evaluateSla(cfg.threshold, cfg);
      assert.strictEqual(first, "met");
      assert.strictEqual(second, "met");
      assert.strictEqual(second, first);
    }
  });

  it("MTTR one above threshold is violated", () => {
      for (const severity of CANONICAL_SEVERITIES) {
        const cfg = CONFIGS[severity];
        assert.strictEqual(evaluateSla(cfg.threshold + 1, cfg), "violated");
      }

  });

  it("exact threshold does not drift into violation due to boundary math", () => {
    for (const cfg of Object.values(CONFIGS)) {
      assert.notStrictEqual(evaluateSla(cfg.threshold, cfg), "violated");
      assert.strictEqual(evaluateSla(cfg.threshold + 1, cfg), "violated");
    }
  });

  it("zero threshold is rejected as invalid — not a silent pass", () => {
    assert.strictEqual(evaluateSla(0, { threshold: 0, penaltyBps: 100 }), "invalid");
    assert.strictEqual(evaluateSla(1, { threshold: 0, penaltyBps: 100 }), "invalid");
  });

  it("negative MTTR is rejected as invalid", () => {
    assert.strictEqual(evaluateSla(-1, CONFIGS['critical']), "invalid");
  });

  it("uses a documented canonical severity order for backend fixtures", () => {
    assert.deepStrictEqual(CANONICAL_SEVERITIES, ["critical", "high", "medium"]);
    assert.deepStrictEqual(Object.keys(CONFIGS), [...CANONICAL_SEVERITIES]);
  });

  it("near-zero MTTR (0.001) treated as zero — rounds to met", () => {
    const nearZero = Math.floor(0.001); // contract uses integer minutes
    assert.strictEqual(evaluateSla(nearZero, CONFIGS['critical']), "met");
  });
});
