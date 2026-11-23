//! Configuration update metadata tracking.
//!
//! This module records the ledger sequence at which the last configuration
//! update occurred, together with the administrative actor that performed it
//! (#671). Backend consumers can query this to determine whether their cached
//! configuration is stale and needs to be refreshed, and to answer
//! "who tuned severity X right before the outage" without correlating config
//! events by hand.
//!
//! # Usage
//!
//! - `record_config_update()` is called by `set_config()`/`set_custom_severity()`
//!   after a successful update, passing the authorized admin (`caller`)
//! - `get_last_config_update()` is exposed through a read-only endpoint for backends
//! - When the returned sequence differs from the backend's cached value, the
//!   backend should re-fetch the full config via `get_config_snapshot()`
//!
//! # Actor semantics
//!
//! The actor is snapshotted in the same storage generation as the ledger
//! sequence, so the pair is always self-consistent. On deployments whose last
//! update predates the v4 storage schema (issue #671), the actor key is
//! absent and `get_last_config_update()` reports `actor: None` — meaning
//! "recorded before attribution existed", never "no actor exists".

use soroban_sdk::{symbol_short, Address, Env, Symbol};

/// On-chain key storing the ledger sequence of the last config update.
/// "LCFGUPD" = Last ConFiG UPDate.
pub const LAST_CFG_UPDATE_KEY: Symbol = symbol_short!("LCFGUPD");

/// On-chain key storing the admin address that performed the last config
/// update (#671). "LCFGUPDA" = Last ConFiG UPDate Actor. Written alongside
/// `LAST_CFG_UPDATE_KEY` by every `set_config`/`set_custom_severity` call;
/// absent until the first config update on a v4+ schema.
pub const LCFG_UPD_ACTOR_KEY: Symbol = symbol_short!("LCFGUPDA");

/// Records the current ledger sequence and the acting admin as the metadata
/// of the latest config update. Called internally by `set_config` and
/// `set_custom_severity` after a successful update.
pub fn record_config_update(env: &Env, actor: &Address) {
    let ledger = env.ledger().sequence();
    env.storage().instance().set(&LAST_CFG_UPDATE_KEY, &ledger);
    env.storage().instance().set(&LCFG_UPD_ACTOR_KEY, actor);
}

/// Returns the ledger sequence and acting admin of the last configuration
/// update. Returns `None` if no config update has been recorded since
/// initialization. The actor is `None` when the recorded update predates the
/// v4 storage schema (see module docs).
pub fn get_last_config_update(env: &Env) -> Option<(u32, Option<Address>)> {
    let ledger: u32 = env.storage().instance().get(&LAST_CFG_UPDATE_KEY)?;
    let actor: Option<Address> = env.storage().instance().get(&LCFG_UPD_ACTOR_KEY);
    Some((ledger, actor))
}

/// Test-only helper that writes only the ledger sequence, reproducing the
/// pre-v4 metadata shape (sequence recorded, no actor key) so migration
/// tests can exercise a legacy v3 deployment.
#[cfg(test)]
pub(crate) fn record_config_update_legacy_sequence(env: &Env, ledger: u32) {
    env.storage().instance().set(&LAST_CFG_UPDATE_KEY, &ledger);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SLACalculatorContract;
    use soroban_sdk::testutils::Address as _;
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
        let admin = Address::generate(&env);
        env.as_contract(&contract_id, || {
            record_config_update(&env, &admin);
            let (ledger, actor) = get_last_config_update(&env).expect("update must be recorded");
            assert_eq!(ledger, env.ledger().sequence());
            assert_eq!(actor, Some(admin), "actor must be persisted with the update");
        });
    }

    /// #671 – the actor snapshot is overwritten by later updates, so the
    /// metadata always reflects the most recent configuration change.
    #[test]
    fn test_actor_is_overwritten_by_later_update() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let first = Address::generate(&env);
        let second = Address::generate(&env);
        env.as_contract(&contract_id, || {
            record_config_update(&env, &first);
            record_config_update(&env, &second);
            let (_, actor) = get_last_config_update(&env).expect("update must be recorded");
            assert_eq!(actor, Some(second), "latest actor must win");
        });
    }

    /// #671 – a sequence recorded without an actor (pre-v4 shape, simulated
    /// here by removing the actor key) must decode as `actor: None` rather
    /// than failing or reporting a zero address.
    #[test]
    fn test_actor_absent_decodes_as_none() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let admin = Address::generate(&env);
        env.as_contract(&contract_id, || {
            record_config_update(&env, &admin);
            env.storage().instance().remove(&LCFG_UPD_ACTOR_KEY);
            let (ledger, actor) = get_last_config_update(&env).expect("sequence must survive");
            assert_eq!(ledger, env.ledger().sequence());
            assert_eq!(actor, None, "missing actor key must decode as None");
        });
    }
}
