//! Storage schema version management and migration detection.
//!
//! This module provides helpers for reading the on-chain storage version
//! and determining whether a storage migration has been completed. Backend
//! consumers use these functions (via `get_migration_state()`) to verify
//! storage compatibility after a contract upgrade.
//!
//! # Version Lifecycle
//!
//! 1. Contract is deployed — `initialize()` stamps `STORAGE_VERSION` (currently 3)
//! 2. Contract is upgraded — new binary may expect a higher version
//! 3. `get_migration_state()` reports `needs_migration: true`
//! 4. Admin calls `migrate()` — storage is transformed and version is bumped
//! 5. `get_migration_state()` reports `needs_migration: false`
//!
//! # Shape-Change Checklist
//!
//! Any PR that increments `STORAGE_VERSION`, adds or renames storage keys, or
//! introduces a new `migrate()` path **must** be reviewed against:
//!
//! `docs/CONTRACT_SHAPE_CHANGE_CHECKLIST.md` — Section 1 (Storage Schema Changes)
//!
//! Key gates from that checklist:
//! - New keys documented in `docs/RESERVED_KEYS_POLICY.md`
//! - `initialize()` stamps the new version constant
//! - Migration is idempotent and covered by a before/after snapshot test
//! - `CHANGELOG.md` records the version number that changed

use soroban_sdk::Env;

use crate::{STORAGE_VERSION, STORAGE_VERSION_KEY};

/// Reads the current storage version from on-chain state.
/// Returns `None` if the contract has not been initialized.
pub fn read_storage_version(env: &Env) -> Option<u32> {
    env.storage().instance().get(&STORAGE_VERSION_KEY)
}

/// Checks whether the on-chain storage has been migrated to the version
/// this contract binary expects. Returns `false` if the contract has not
/// been initialized, or if `migrate()` has not yet been run to bring an
/// older deployment up to `STORAGE_VERSION`.
///
/// This reads the same `STORAGE_VERSION_KEY` that `initialize()` stamps and
/// `migrate()` advances, so it always agrees with `get_migration_state()`.
pub fn is_migration_complete(env: &Env) -> bool {
    read_storage_version(env) == Some(STORAGE_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_storage_version_unset_returns_none() {
        let env = Env::default();
        let cid = env.register_contract(None, crate::SLACalculatorContract);
        env.as_contract(&cid, || {
            assert_eq!(read_storage_version(&env), None);
        });
    }

    #[test]
    fn test_migration_incomplete_when_unset() {
        let env = Env::default();
        let cid = env.register_contract(None, crate::SLACalculatorContract);
        env.as_contract(&cid, || {
            assert!(!is_migration_complete(&env));
        });
    }

    #[test]
    fn test_migration_complete_when_version_current() {
        let env = Env::default();
        let cid = env.register_contract(None, crate::SLACalculatorContract);
        env.as_contract(&cid, || {
            env.storage()
                .instance()
                .set(&STORAGE_VERSION_KEY, &STORAGE_VERSION);
            assert!(is_migration_complete(&env));
        });
    }

    #[test]
    fn test_migration_incomplete_when_version_stale() {
        let env = Env::default();
        let cid = env.register_contract(None, crate::SLACalculatorContract);
        env.as_contract(&cid, || {
            env.storage().instance().set(&STORAGE_VERSION_KEY, &0u32);
            assert!(!is_migration_complete(&env));
        });
    }
}
