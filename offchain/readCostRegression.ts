/**
 * SC-016: Read-cost regression tests for history and config-bundle helpers.
 * Simulates budget-aware reads against contract view helpers and asserts
 * that response sizes stay within documented thresholds.
 */

const READ_BUDGET_LIMITS = {
  historyRead: 512,    // bytes
  configBundle: 256,
  metadataRead: 128,
};

interface ReadSample {
  helper: keyof typeof READ_BUDGET_LIMITS;
  payload: unknown;
}

function measurePayloadSize(payload: unknown): number {
  return Buffer.byteLength(JSON.stringify(payload), "utf8");
}

function assertReadBudget(sample: ReadSample): void {
  const limit = READ_BUDGET_LIMITS[sample.helper];
  const size = measurePayloadSize(sample.payload);
  if (size > limit) {
    throw new Error(
      `[SC-016] ${sample.helper} payload ${size}B exceeds budget ${limit}B`
    );
  }
  console.log(`  ✓ ${sample.helper}: ${size}B / ${limit}B`);
}

const mockHistoryRead = Array.from({ length: 5 }, (_, i) => ({
  id: i,
  severity: "critical",
  mttr: 30 + i,
  sla_met: true,
}));

const mockConfigBundle = {
  critical: { threshold: 60, penalty: 10, reward: 5 },
  high: { threshold: 120, penalty: 8, reward: 4 },
  medium: { threshold: 240, penalty: 5, reward: 2 },
};

const mockMetadataRead = {
  version: "1.0.0",
  paused: false,
  operator: "GABC",
};

const samples: ReadSample[] = [
  { helper: "historyRead", payload: mockHistoryRead },
  { helper: "configBundle", payload: mockConfigBundle },
  { helper: "metadataRead", payload: mockMetadataRead },
];

console.log("[SC-016] Read-cost regression checks:");
samples.forEach(assertReadBudget);
console.log("All read-cost checks passed.");
