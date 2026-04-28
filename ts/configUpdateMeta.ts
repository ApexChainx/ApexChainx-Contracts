// SC-033: Deterministic metadata for the last config update actor and ledger context.
// Stored and updated on every successful config change; queryable by backend.

export interface ConfigUpdateMeta {
  actor: string;
  ledger: number;
  timestamp: number;
  update_count: number;
}

let _meta: ConfigUpdateMeta | null = null;

export function recordConfigUpdate(actor: string, ledger: number, timestamp: number): void {
  _meta = {
    actor,
    ledger,
    timestamp,
    update_count: (_meta?.update_count ?? 0) + 1,
  };
}

export function getConfigUpdateMeta(): ConfigUpdateMeta | null {
  return _meta ? { ..._meta } : null;
}

export function resetConfigUpdateMeta(): void {
  _meta = null;
}

// --- determinism tests ---

function assert(cond: boolean, msg: string): void {
  if (!cond) throw new Error(`FAIL: ${msg}`);
  console.log(`PASS  ${msg}`);
}

resetConfigUpdateMeta();
assert(getConfigUpdateMeta() === null, "no meta before first update");

recordConfigUpdate("admin-abc", 1000, 1714000000);
const m1 = getConfigUpdateMeta()!;
assert(m1.actor === "admin-abc", "actor recorded correctly");
assert(m1.ledger === 1000, "ledger recorded correctly");
assert(m1.update_count === 1, "update_count starts at 1");

recordConfigUpdate("admin-xyz", 1050, 1714000300);
const m2 = getConfigUpdateMeta()!;
assert(m2.actor === "admin-xyz", "actor updates on second change");
assert(m2.update_count === 2, "update_count increments");

// idempotency: same call twice should increment
recordConfigUpdate("admin-xyz", 1050, 1714000300);
assert(getConfigUpdateMeta()!.update_count === 3, "each call increments count");
