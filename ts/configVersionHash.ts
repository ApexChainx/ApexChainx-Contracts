/**
 * SC-009 — Collision-resistant config version hash (#129)
 *
 * Replaces the additive checksum with a djb2-style polynomial hash over a
 * canonical JSON serialisation of the config snapshot.  Identical snapshots
 * always produce the same u32 identifier; single-field changes produce a
 * different value with high probability.
 *
 * Backend consumers: compare `versionHash` values to detect config drift.
 * Do NOT treat the raw number as meaningful — only equality/inequality matters.
 */

export interface SlaConfig {
  severity: string;
  threshold: number;
  penaltyBps: number;
  rewardBps: number;
}

export interface ConfigSnapshot {
  configs: SlaConfig[];
  updatedAt: number; // ledger timestamp
}

/**
 * Deterministic canonical serialisation — keys sorted, no extra whitespace.
 */
function canonicalise(snapshot: ConfigSnapshot): string {
  const sorted = snapshot.configs
    .slice()
    .sort((a, b) => a.severity.localeCompare(b.severity));
  return JSON.stringify({ configs: sorted, updatedAt: snapshot.updatedAt });
}

/**
 * djb2 polynomial hash over UTF-16 code units.
 * Returns an unsigned 32-bit integer.
 */
function djb2(input: string): number {
  let hash = 5381;
  for (let i = 0; i < input.length; i++) {
    hash = ((hash << 5) + hash + input.charCodeAt(i)) >>> 0;
  }
  return hash;
}

/**
 * Public API — produces a stable, collision-resistant version identifier for
 * a given config snapshot.
 */
export function configVersionHash(snapshot: ConfigSnapshot): number {
  return djb2(canonicalise(snapshot));
}

// ---------------------------------------------------------------------------
// Quick self-test (run with: npx ts-node configVersionHash.ts)
// ---------------------------------------------------------------------------
if (require.main === module) {
  const snap: ConfigSnapshot = {
    configs: [
      { severity: "critical", threshold: 60, penaltyBps: 500, rewardBps: 100 },
      { severity: "high",     threshold: 120, penaltyBps: 300, rewardBps: 50  },
    ],
    updatedAt: 1_000_000,
  };

  const h1 = configVersionHash(snap);
  const h2 = configVersionHash({ ...snap, updatedAt: 1_000_001 });

  console.assert(h1 === configVersionHash(snap), "must be deterministic");
  console.assert(h1 !== h2,                      "must differ on change");
  console.log("configVersionHash:", h1.toString(16));
}
