//! Storage schema version management and migration detection.
//!
//! This module provides helpers for reading the on-chain storage version
//! and determining whether a storage migration has been completed. Backend
//! consumers use these functions (via `get_migration_state()`) to verify
//! storage compatibility after a contract upgrade.
//!
//! # Version Lifecycle
//!
//! 1. Contract is deployed — `initialize()` stamps `STORAGE_VERSION` (currently 1)
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

use soroban_sdk::{symbol_short, Env, Symbol};

/// On-chain key for the stored storage version number.
const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");

/// On-chain key for the migration completion flag.
/// Set to `true` after a successful migration completes.
const MIGRATION_FLAG_KEY: Symbol = symbol_short!("MIGRATED");

/// Reads the current storage version from on-chain state.
/// Returns `None` if the contract has not been initialized.
pub fn read_storage_version(env: &Env) -> Option<u32> {
    env.storage().instance().get(&STORAGE_VERSION_KEY)
}

/// Checks whether a storage migration has been completed.
/// Returns `false` if the contract was never migrated (default).
pub fn is_migration_complete(env: &Env) -> bool {
    env.storage()
        .instance()
        .get::<Symbol, bool>(&MIGRATION_FLAG_KEY)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_storage_version_unset_returns_none() {
        let env = Env::default();
        assert_eq!(read_storage_version(&env), None);
    }

    #[test]
    fn test_migration_flag_defaults_to_false() {
        let env = Env::default();
        assert!(!is_migration_complete(&env));
    }
}
