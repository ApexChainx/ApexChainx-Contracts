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
    use crate::{SLACalculatorContract, SLACalculatorContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup() -> (Env, SLACalculatorContractClient<'static>, Address, Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        (env, client, admin, operator)
    }

    #[test]
    fn test_config_unfrozen_by_default() {
        let (_env, client, _admin, _operator) = setup();
        assert!(!client.is_config_frozen());
    }

    #[test]
    fn test_freeze_and_query() {
        let (_env, client, admin, _operator) = setup();
        client.freeze_config(&admin);
        assert!(client.is_config_frozen());
    }

    #[test]
    fn test_unfreeze_restores_mutable_state() {
        let (_env, client, admin, _operator) = setup();
        client.freeze_config(&admin);
        client.unfreeze_config(&admin);
        assert!(!client.is_config_frozen());
    }

    #[test]
    fn test_frozen_config_flag() {
        let (_env, client, admin, _operator) = setup();
        client.freeze_config(&admin);
        assert!(client.is_config_frozen());
    }

    #[test]
    #[should_panic(expected = "#16")]
    fn test_set_config_fails_when_frozen() {
        let (_env, client, admin, _operator) = setup();
        client.freeze_config(&admin);
        client.set_config(&admin, &soroban_sdk::symbol_short!("critical"), &15, &100, &750);
    }

    #[test]
    fn test_unfreeze_allows_set_config() {
        let (_env, client, admin, _operator) = setup();
        client.freeze_config(&admin);
        assert!(client.is_config_frozen());
        client.unfreeze_config(&admin);
        assert!(!client.is_config_frozen());
        client.set_config(&admin, &soroban_sdk::symbol_short!("critical"), &15, &100, &750);
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_set_config_when_unfrozen() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        let stranger = Address::generate(&env);
        client.set_config(
            &stranger,
            &soroban_sdk::symbol_short!("critical"),
            &15,
            &100,
            &750,
        );
    }
}
