/**
 * governanceSummary.ts
 * Issue #154 — single-read governance state summary for dashboards and health checks.
 */

export interface PauseInfo {
  reason: string;
  pausedAt: number; // ledger timestamp
}

export interface GovernanceSummary {
  isPaused: boolean;
  pauseInfo: PauseInfo | null;
  pendingAdmin: string | null;
  pendingOperator: string | null;
}

/** Soroban contract view responses (minimal shape). */
interface ContractClient {
  isPaused(): Promise<boolean>;
  getPauseInfo(): Promise<PauseInfo | null>;
  getPendingAdmin(): Promise<string | null>;
  getPendingOperator(): Promise<string | null>;
}

/**
 * Fetches all governance state in parallel and returns a deterministic summary.
 * Replaces four separate reads with one aggregated call.
 */
export async function getGovernanceSummary(
  client: ContractClient
): Promise<GovernanceSummary> {
  const [isPaused, pauseInfo, pendingAdmin, pendingOperator] =
    await Promise.all([
      client.isPaused(),
      client.getPauseInfo(),
      client.getPendingAdmin(),
      client.getPendingOperator(),
    ]);

  return { isPaused, pauseInfo, pendingAdmin, pendingOperator };
}

/** Returns true when any governance action is pending or the contract is paused. */
export function isGovernanceActionPending(s: GovernanceSummary): boolean {
  return s.isPaused || s.pendingAdmin !== null || s.pendingOperator !== null;
}
