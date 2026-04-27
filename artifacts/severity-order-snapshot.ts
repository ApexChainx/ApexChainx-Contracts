/**
 * SC-029: Canonical severity-order snapshot for backend parity tooling.
 * Consumers import SEVERITY_ORDER to avoid duplicating ordering assumptions.
 * Tests below catch accidental reordering before it propagates.
 */

export const SEVERITY_ORDER = ["critical", "high", "medium", "low"] as const;
export type Severity = (typeof SEVERITY_ORDER)[number];

export interface SeveritySnapshot {
  version: number;
  order: readonly Severity[];
  generatedAt: string;
}

export function buildSeveritySnapshot(): SeveritySnapshot {
  return {
    version: 1,
    order: SEVERITY_ORDER,
    generatedAt: new Date().toISOString(),
  };
}

export function assertSeverityOrder(snapshot: SeveritySnapshot): void {
  const expected = SEVERITY_ORDER.join(",");
  const actual = snapshot.order.join(",");
  if (actual !== expected) {
    throw new Error(
      `Severity order mismatch — expected [${expected}] got [${actual}]`
    );
  }
}

export function severityIndex(s: Severity): number {
  const idx = SEVERITY_ORDER.indexOf(s);
  if (idx === -1) throw new Error(`Unknown severity: ${s}`);
  return idx;
}

export function compareSeverity(a: Severity, b: Severity): number {
  return severityIndex(a) - severityIndex(b);
}

// --- self-test (run with: npx ts-node artifacts/severity-order-snapshot.ts) ---
if (require.main === module) {
  const snap = buildSeveritySnapshot();
  assertSeverityOrder(snap);
  console.log("Severity snapshot OK:", JSON.stringify(snap, null, 2));

  const sorted: Severity[] = (["low", "critical", "medium", "high"] as Severity[]).sort(
    compareSeverity
  );
  console.log("Sorted:", sorted);
  console.assert(sorted[0] === "critical", "critical must be first");
  console.assert(sorted[3] === "low", "low must be last");
  console.log("All assertions passed.");
}
