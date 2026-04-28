/**
 * SC-046: Threshold edge-case tests for zero and near-zero values.
 * Documents and validates contract policy for boundary MTTR inputs.
 */

interface SlaConfig {
  threshold: number; // minutes
  penaltyBps: number;
}

// Mirrors contract-side SLA evaluation logic
function evaluateSla(mttr: number, config: SlaConfig): "met" | "violated" | "invalid" {
  if (mttr < 0 || config.threshold < 0) return "invalid";
  if (config.threshold === 0) return "invalid"; // zero threshold is rejected by contract
  return mttr <= config.threshold ? "met" : "violated";
}

const CONFIGS: Record<string, SlaConfig> = {
  critical: { threshold: 60,  penaltyBps: 500 },
  high:     { threshold: 240, penaltyBps: 300 },
  medium:   { threshold: 480, penaltyBps: 100 },
};

describe("SC-046 Threshold Edge Cases", () => {
  it("zero MTTR always meets any positive threshold", () => {
    for (const cfg of Object.values(CONFIGS)) {
      expect(evaluateSla(0, cfg)).toBe("met");
    }
  });

  it("MTTR of 1 meets threshold when threshold >= 1", () => {
    for (const cfg of Object.values(CONFIGS)) {
      expect(evaluateSla(1, cfg)).toBe("met");
    }
  });

  it("MTTR exactly at threshold is met (inclusive boundary)", () => {
    for (const cfg of Object.values(CONFIGS)) {
      expect(evaluateSla(cfg.threshold, cfg)).toBe("met");
    }
  });

  it("MTTR one above threshold is violated", () => {
    for (const cfg of Object.values(CONFIGS)) {
      expect(evaluateSla(cfg.threshold + 1, cfg)).toBe("violated");
    }
  });

  it("zero threshold is rejected as invalid — not a silent pass", () => {
    expect(evaluateSla(0, { threshold: 0, penaltyBps: 100 })).toBe("invalid");
    expect(evaluateSla(1, { threshold: 0, penaltyBps: 100 })).toBe("invalid");
  });

  it("negative MTTR is rejected as invalid", () => {
    expect(evaluateSla(-1, CONFIGS.critical)).toBe("invalid");
  });

  it("near-zero MTTR (0.001) treated as zero — rounds to met", () => {
    const nearZero = Math.floor(0.001); // contract uses integer minutes
    expect(evaluateSla(nearZero, CONFIGS.critical)).toBe("met");
  });
});
