use soroban_sdk::Env;

/// TTL (Time-To-Live) threshold for contract storage.
/// If TTL falls below this, we bump it.
const TTL_THRESHOLD_LEDGERS: u32 = 100_000;

/// Extend TTL to this many ledgers from now.
const TTL_EXTEND_TO_LEDGERS: u32 = 1_000_000;

/// Bump the instance storage TTL to prevent contract archival.
/// Call this after every state-mutating operation.
pub fn bump_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_LEDGERS, TTL_EXTEND_TO_LEDGERS);
}
