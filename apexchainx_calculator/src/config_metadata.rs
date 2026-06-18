//! Configuration update metadata tracking.
//!
//! This module records the ledger sequence at which the last configuration
//! update occurred. Backend consumers can query this to determine whether
//! their cached configuration is stale and needs to be refreshed.
//!
//! # Usage
//!
//! - `record_config_update()` is called by `set_config()` after a successful update
//! - `get_last_config_update()` is exposed through a read-only endpoint for backends
//! - When the returned sequence differs from the backend's cached value, the
//!   backend should re-fetch the full config via `get_config_snapshot()`

use soroban_sdk::{symbol_short, Env, Symbol};

/// On-chain key storing the ledger sequence of the last config update.
/// "LCFGUPD" = Last ConFiG UPDate.
pub const LAST_CFG_UPDATE_KEY: Symbol = symbol_short!("LCFGUPD");

/// Records the current ledger sequence as the time of the latest config update.
/// Called internally by `set_config` after a successful update.
pub fn record_config_update(env: &Env) {
    let ledger = env.ledger().sequence();
    env.storage().instance().set(&LAST_CFG_UPDATE_KEY, &ledger);
}

/// Returns the ledger sequence of the last configuration update.
/// Returns `None` if no config update has been recorded since initialization.
pub fn get_last_config_update(env: &Env) -> Option<u32> {
    env.storage().instance().get(&LAST_CFG_UPDATE_KEY)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SLACalculatorContract;
    use soroban_sdk::Env;

    // These tests exercise the helper functions in isolation through the
    // contract's instance-storage context. Without `env.as_contract(...)`,
    // Soroban rejects instance-storage access from a bare `Env::default()`.
    #[test]
    fn test_last_config_update_unset() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        env.as_contract(&contract_id, || {
            assert_eq!(get_last_config_update(&env), None);
        });
    }

    #[test]
    fn test_record_and_read_config_update() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        env.as_contract(&contract_id, || {
            record_config_update(&env);
            let ledger = get_last_config_update(&env);
            assert!(ledger.is_some());
        });
    }
}
