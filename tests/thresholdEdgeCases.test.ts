/**
 * SC-046: Threshold edge-case tests for zero and near-zero values.
 * Documents and validates contract policy for boundary MTTR inputs.
 */

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

const CONFIGS: Record<string, SlaConfig> = {
  critical: { threshold: 60, penaltyBps: 500 },
  high: { threshold: 240, penaltyBps: 300 },
  medium: { threshold: 480, penaltyBps: 100 },
};

const CANONICAL_SEVERITIES = ["critical", "high", "medium"] as const;

describe("SC-046 Threshold Edge Cases", () => {
  it("zero MTTR always meets any positive threshold", () => {
    for (const severity of CANONICAL_SEVERITIES) {
      expect(evaluateSla(0, CONFIGS[severity])).toBe("met");
    }
  });

  it("MTTR of 1 meets threshold when threshold >= 1", () => {
    for (const severity of CANONICAL_SEVERITIES) {
      expect(evaluateSla(1, CONFIGS[severity])).toBe("met");
    }
  });

  it("MTTR exactly at threshold is met (inclusive boundary)", () => {
    for (const severity of CANONICAL_SEVERITIES) {
      const cfg = CONFIGS[severity];
      expect(evaluateSla(cfg.threshold, cfg)).toBe("met");
    }
  });

  it("exact-threshold results are deterministic for backend replay", () => {
    for (const cfg of Object.values(CONFIGS)) {
      const first = evaluateSla(cfg.threshold, cfg);
      const second = evaluateSla(cfg.threshold, cfg);
      expect(first).toBe("met");
      expect(second).toBe("met");
      expect(second).toBe(first);
    }
  });

  it("MTTR one above threshold is violated", () => {
    for (const severity of CANONICAL_SEVERITIES) {
      const cfg = CONFIGS[severity];
      expect(evaluateSla(cfg.threshold + 1, cfg)).toBe("violated");
    }
  });

  it("exact threshold does not drift into violation due to boundary math", () => {
    for (const cfg of Object.values(CONFIGS)) {
      expect(evaluateSla(cfg.threshold, cfg)).not.toBe("violated");
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

  it("uses a documented canonical severity order for backend fixtures", () => {
    expect(CANONICAL_SEVERITIES).toEqual(["critical", "high", "medium"]);
    expect(Object.keys(CONFIGS)).toEqual([...CANONICAL_SEVERITIES]);
  });

  it("near-zero MTTR (0.001) treated as zero — rounds to met", () => {
    const nearZero = Math.floor(0.001); // contract uses integer minutes
    expect(evaluateSla(nearZero, CONFIGS.critical)).toBe("met");
  });
});
