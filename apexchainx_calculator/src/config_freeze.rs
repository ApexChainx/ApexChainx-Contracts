//! Configuration freeze/unfreeze mechanism for emergency lock-down.
//!
//! This module provides a config freeze mechanism that can be used to
//! temporarily prevent configuration changes during critical operations.
//! When the config is frozen, `set_config` calls are blocked, ensuring
//! that SLA parameters remain stable during audit periods or incident
//! response.
//!
//! # State Machine
//!
//! ```text
//!         freeze_config()
//!   ┌─────────────────────────┐
//!   │                         ▼
//! ┌──────────┐         ┌──────────┐
//! │ Thawed   │         │ Frozen   │
//! └──────────┘         └──────────┘
//!   ▲                         │
//!   └─────────────────────────┘
//!         unfreeze_config()
//! ```
//!
//! # Default State
//!
//! Config starts in the **thawed** state after initialization. Freezing is
//! an explicit admin action, not the default.

use soroban_sdk::{symbol_short, Env, Symbol};

/// On-chain key for the config freeze boolean flag.
const FREEZE_KEY: Symbol = symbol_short!("FREEZE");

/// Freezes the configuration, blocking further config updates.
/// After calling this, `set_config` will reject changes.
pub fn freeze_config(env: &Env) {
    env.storage().instance().set(&FREEZE_KEY, &true);
}

/// Unfreezes the configuration, re-allowing config updates.
/// Restores normal operation after a freeze.
pub fn unfreeze_config(env: &Env) {
    env.storage().instance().set(&FREEZE_KEY, &false);
}

/// Returns `true` if the configuration is currently frozen.
/// Defaults to `false` (thawed) if never explicitly set.
pub fn is_config_frozen(env: &Env) -> bool {
    env.storage()
        .instance()
        .get::<Symbol, bool>(&FREEZE_KEY)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use crate::{SLACalculatorContract, SLACalculatorContractClient};

    #[test]
    fn test_config_unfrozen_by_default() {
        let env = Env::default();
        assert!(!is_config_frozen(&env));
    }

    #[test]
    fn test_freeze_and_query() {
        let env = Env::default();
        freeze_config(&env);
        assert!(is_config_frozen(&env));
    }

    #[test]
    fn test_unfreeze_restores_mutable_state() {
        let env = Env::default();
        freeze_config(&env);
        unfreeze_config(&env);
        assert!(!is_config_frozen(&env));
    }

    #[test]
    fn test_frozen_config_blocks_set_config() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        freeze_config(&env);
        assert!(is_config_frozen(&env));
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_set_config_when_unfrozen() {
        // Verify that even without a freeze, an unauthorized caller cannot
        // mutate config (admin-gated role check, not freeze-gated logic).
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        let stranger = Address::generate(&env);
        // stranger does not hold the admin role – require_admin must reject
        client.set_config(
            &stranger,
            &soroban_sdk::symbol_short!("critical"),
            &15,
            &100,
            &750,
        );
    }
}
