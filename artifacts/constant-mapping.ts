/**
 * SC-031: Off-chain constant mapping for contract symbols.
 * Generates a stable artifact backend consumers can import to avoid drift.
 * Run: npx ts-node artifacts/constant-mapping.ts > artifacts/constants.json
 */

export const CONTRACT_CONSTANTS = {
  severities: ["critical", "high", "medium", "low"],
  rewardTiers: {
    top: { label: "top", minScore: 95 },
    excellent: { label: "excellent", minScore: 85 },
    good: { label: "good", minScore: 70 },
    standard: { label: "standard", minScore: 0 },
  },
  slaThresholds: {
    critical: 60,
    high: 240,
    medium: 480,
    low: 1440,
  },
  historyMaxEntries: 100,
  schemaVersion: 1,
} as const;

export type ContractConstants = typeof CONTRACT_CONSTANTS;

export function generateMapping(): string {
  return JSON.stringify(
    { generatedAt: new Date().toISOString(), ...CONTRACT_CONSTANTS },
    null,
    2
  );
}

export function assertConstantsUnchanged(snapshot: Partial<ContractConstants>): void {
  for (const [key, value] of Object.entries(snapshot)) {
    const current = (CONTRACT_CONSTANTS as Record<string, unknown>)[key];
    if (JSON.stringify(current) !== JSON.stringify(value)) {
      throw new Error(
        `Constant drift detected for "${key}": expected ${JSON.stringify(value)}, got ${JSON.stringify(current)}`
      );
    }
  }
}

if (require.main === module) {
  const mapping = generateMapping();
  console.log(mapping);

  // Smoke-test: round-trip the generated JSON back through the assertion guard
  const parsed = JSON.parse(mapping);
  assertConstantsUnchanged({ severities: parsed.severities, schemaVersion: parsed.schemaVersion });
  console.error("Constant mapping generated and verified ✓");
}
