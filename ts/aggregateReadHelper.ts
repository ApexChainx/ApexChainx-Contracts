// SC-020: Aggregate read helper — bundles schema, config, and governance state
// Reduces backend round-trips and prevents mismatched snapshots.

export interface SeverityConfig {
  threshold_seconds: number;
  reward_bps: number;
  penalty_bps: number;
}

export interface ConfigSnapshot {
  critical: SeverityConfig;
  high: SeverityConfig;
  medium: SeverityConfig;
  low: SeverityConfig;
}

export interface GovernanceState {
  admin: string;
  operator: string;
  pending_admin: string | null;
  pending_operator: string | null;
  paused: boolean;
}

export interface ResultSchema {
  fields: string[];
  version: number;
}

export interface AggregateSnapshot {
  schema: ResultSchema;
  config: ConfigSnapshot;
  governance: GovernanceState;
  captured_at_ledger: number;
}

export function buildAggregateSnapshot(
  schema: ResultSchema,
  config: ConfigSnapshot,
  governance: GovernanceState,
  ledger: number
): AggregateSnapshot {
  return { schema, config, governance, captured_at_ledger: ledger };
}

export function snapshotsMatch(a: AggregateSnapshot, b: AggregateSnapshot): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}
