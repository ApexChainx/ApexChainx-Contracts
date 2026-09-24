//! Configuration freeze/unfreeze mechanism for emergency lock-down.
//!
//! This module provides a config freeze mechanism that can be used to
//! temporarily prevent configuration changes during critical operations.
//! When the config is frozen, `set_config` calls are blocked, ensuring
//! that SLA parameters remain stable during audit periods or incident
//! response.
//!
//! # Pause vs Freeze — Operational Semantics (Issue #666)
//!
//! The contract exposes **two independent** operational guards. Operators
//! choosing the right tool during incident response must understand the
//! distinction:
//!
//! | Guard     | Storage flag | Blocks                                        | Allows                          | Who triggers |
//! |-----------|--------------|-----------------------------------------------|---------------------------------|--------------|
//! | **Pause** | `PAUSED_KEY` | All state-changing calls: calculate_sla,      | Read-only queries (`get_*`,     | Admin        |
//! |           |              | config writes, governance transitions         | `is_*`, view functions)         |              |
//! | **Freeze**| `FREEZE_KEY` | Config writes only: `set_config`,             | calculate_sla, read queries,    | Admin        |
//! |           |              | governance proposals/accepts, operator changes| governance non-config calls     |              |
//!
//! ## When to use Pause
//! Use `pause` when you need to **halt all mutations** — typically during a live
//! incident where the contract must be fully read-only while you diagnose. Pause
//! blocks `calculate_sla`, config writes, AND governance transitions.
//!
//! ## When to use Freeze
//! Use `freeze_config` when you want to **hold config writes stable** during a
//! compliance review, audit window, or config migration review, while still
//! allowing normal SLA calculations and operator activity to continue. Freeze is
//! the narrower, scalpel-like tool: it gates only `require_not_frozen` call sites.
//!
//! ## Audit trail events
//! Both transitions emit versioned events so the event stream carries a complete
//! audit trail identifying which guard caused a rejection:
//!
//! - `paused` / `unpause` — emitted by `metadata::pause` / `metadata::unpause`
//! - `cfg_frz` / `cfg_unfrz` — emitted by the `freeze_config` / `unfreeze_config`
//!   contract methods in `lib.rs` (after require_admin, before returning)
//!
//! The event names are distinct, so an indexer receiving a guard-failure can
//! determine unambiguously which guard fired by scanning for the most recent
//! `paused` or `cfg_frz` event.
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
//! Transitions are **state-transition-safe**: calling `freeze_config` on an
//! already-frozen config (or `unfreeze_config` on an already-thawed one) is a
//! silent no-op that returns `false`, so callers can emit an event only on a
//! real transition and an audit system can reconstruct the frozen window
//! exactly from the event stream (#592).
//!
//! # Default State
//!
//! Config starts in the **thawed** state after initialization. Freezing is
//! an explicit admin action, not the default.

use soroban_sdk::{symbol_short, Env, Symbol};

/// On-chain key for the config freeze boolean flag.
const FREEZE_KEY: Symbol = symbol_short!("FREEZE");

/// Freezes the configuration, blocking further config updates.
/// Idempotent: when the config is already frozen this is a silent no-op.
/// Returns `true` when the state actually transitioned thawed → frozen.
/// Event emission (`cfg_frz`) is handled by the contract method in `lib.rs`,
/// and is emitted only on a real transition (#592, #666).
pub fn freeze_config(env: &Env) -> bool {
    if is_config_frozen(env) {
        return false;
    }
    env.storage().instance().set(&FREEZE_KEY, &true);
    true
}

/// Unfreezes the configuration, re-allowing config updates.
/// Idempotent: when the config is already thawed this is a silent no-op.
/// Returns `true` when the state actually transitioned frozen → thawed.
/// Event emission (`cfg_unfrz`) is handled by the contract method in `lib.rs`,
/// and is emitted only on a real transition (#592, #666).
pub fn unfreeze_config(env: &Env) -> bool {
    if !is_config_frozen(env) {
        return false;
    }
    env.storage().instance().set(&FREEZE_KEY, &false);
    true
}

/// Returns `true` if the configuration is currently frozen.
/// Defaults to `false` (thawed) if never explicitly set.
pub fn is_config_frozen(env: &Env) -> bool {
    env.storage()
        .instance()
        .get::<Symbol, bool>(&FREEZE_KEY)
        .unwrap_or(false)
}

/// Requires that the configuration is not currently frozen.
/// Returns `Err(SLAError::ConfigFrozen)` if frozen.
pub fn require_not_frozen(env: &Env) -> Result<(), crate::SLAError> {
    if is_config_frozen(env) {
        return Err(crate::SLAError::ConfigFrozen);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{SLACalculatorContract, SLACalculatorContractClient};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    fn setup() -> (Env, SLACalculatorContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
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

    #[test]
    #[should_panic(expected = "#16")]
    fn test_propose_admin_fails_when_frozen() {
        let (_env, client, admin, _operator) = setup();
        let new_admin = Address::generate(&_env);
        client.freeze_config(&admin);
        client.propose_admin(&admin, &new_admin);
    }

    #[test]
    #[should_panic(expected = "#16")]
    fn test_propose_operator_fails_when_frozen() {
        let (_env, client, admin, _operator) = setup();
        let new_op = Address::generate(&_env);
        client.freeze_config(&admin);
        client.propose_operator(&admin, &new_op);
    }

    #[test]
    #[should_panic(expected = "#16")]
    fn test_renounce_admin_fails_when_frozen() {
        let (_env, client, admin, _operator) = setup();
        client.freeze_config(&admin);
        client.renounce_admin(&admin);
    }

    #[test]
    #[should_panic(expected = "#16")]
    fn test_set_operator_fails_when_frozen() {
        let (_env, client, admin, _operator) = setup();
        let new_op = Address::generate(&_env);
        client.freeze_config(&admin);
        client.set_operator(&admin, &new_op);
    }

    /// Issue #666: guard asymmetry test — freeze must NOT block calculate_sla,
    /// confirming freeze is narrower than pause.
    #[test]
    fn test_freeze_does_not_block_calculate_sla() {
        let (_env, client, admin, operator) = setup();
        client.freeze_config(&admin);
        // calculate_sla is only guarded by require_not_paused, not require_not_frozen.
        let result = client.calculate_sla(&operator, &soroban_sdk::symbol_short!("INC001"), &30u32);
        assert_eq!(result.mttr_minutes, 30, "freeze must not block calculate_sla");
    }
}