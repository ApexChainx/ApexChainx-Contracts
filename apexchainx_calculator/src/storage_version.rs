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
//!    Note: Issues #97 (operator parity) and #92 (cross-parameter/cross-severity
//!    validation) ship *additively* on top of v1 and do **not** bump
//!    `STORAGE_VERSION` — existing contracts migrate transparently because
//!    the new `PENDING_*_STATE_KEY` keys are written alongside (not in
//!    place of) the legacy `PENDING_*_KEY` entries, and the new SLAError
//!    codes (19, 20) only ever fail-write on invalid config updates.
//! 2. Contract is upgraded — new binary may expect a higher version
//! 3. `get_migration_state()` reports `needs_migration: true`
//! 4. Admin calls `migrate()` — storage is transformed and version is bumped
//! 5. `get_migration_state()` reports `needs_migration: false`

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
