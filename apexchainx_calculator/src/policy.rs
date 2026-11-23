//! Centralized policy registry for reserved storage keys and event topics.
//!
//! This module documents the logical namespaces reserved by the contract —
//! storage keys and event topics — so any future path that accepts
//! caller-supplied keys can validate against them before writing.
//!
//! # Relationship to the live contract
//!
//! The contract's own storage writes use the explicit symbol constants
//! declared in `lib.rs` (e.g. [`crate::CONFIG_KEY`],
//! [`crate::HISTORY_KEY`], [`crate::STORAGE_VERSION_KEY`]). Those constants
//! are the authoritative layout and are guaranteed distinct by
//! `tests::test_storage_key_namespace_symbols_are_distinct`. This module is
//! the *policy layer* over that layout: it names the reserved namespaces so
//! future caller-supplied-key entry points have a single place to check.
//!
//! There is no path today that accepts an arbitrary top-level storage key, so
//! `validate_storage_key` is not called from live code paths; it is provided
//! as the guard for such entry points and is pinned by the tests in this
//! module so its contract is exercised every build.

use soroban_sdk::{symbol_short, Symbol};

/// Centralized registry for reserved storage key namespaces.
///
/// These namespaces are reserved for core contract functionality and must not
/// be reused by custom or user-defined keys. See the module docs for how this
/// registry relates to the live key constants in `lib.rs`.
pub struct ReservedStorageKeys;

impl ReservedStorageKeys {
    /// Reserved namespace for configuration storage.
    pub const CONFIG: Symbol = symbol_short!("Config");
    /// Reserved namespace for governance storage.
    pub const GOV: Symbol = symbol_short!("Gov");
    /// Reserved namespace for history storage.
    pub const HIST: Symbol = symbol_short!("Hist");
    /// Reserved namespace for telemetry storage.
    pub const TELEMETRY: Symbol = symbol_short!("Telemetry");
    /// Reserved namespace for version storage.
    pub const VERSION: Symbol = symbol_short!("Version");
}

/// Centralized registry for reserved event-topic namespaces.
///
/// These topic names are reserved and must not be reused by new events
/// without a corresponding event-version bump.
pub struct ReservedEventTopics;

impl ReservedEventTopics {
    /// Reserved topic for configuration update events.
    pub const CONFIG_UPDATE: Symbol = symbol_short!("cfg_upd");
    /// Reserved topic for governance proposal events.
    pub const GOV_PROPOSE: Symbol = symbol_short!("gov_prop");
    /// Reserved topic for calculation execution events.
    pub const CALC_EXEC: Symbol = symbol_short!("calc_ex");
    /// Reserved topic for system pause events.
    pub const SYS_PAUSE: Symbol = symbol_short!("sys_pause");
}

/// Validates that a custom storage key does not collide with reserved namespaces.
///
/// Returns `true` if the key is safe to use as a top-level storage key, `false`
/// if it overlaps with a reserved namespace.
pub fn validate_storage_key(key: &Symbol) -> bool {
    // Prevent overriding core reserved namespaces
    key != &ReservedStorageKeys::CONFIG
        && key != &ReservedStorageKeys::GOV
        && key != &ReservedStorageKeys::HIST
        && key != &ReservedStorageKeys::TELEMETRY
        && key != &ReservedStorageKeys::VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserved_storage_keys_are_distinct() {
        let reserved = [
            ReservedStorageKeys::CONFIG,
            ReservedStorageKeys::GOV,
            ReservedStorageKeys::HIST,
            ReservedStorageKeys::TELEMETRY,
            ReservedStorageKeys::VERSION,
        ];
        for i in 0..reserved.len() {
            for j in (i + 1)..reserved.len() {
                assert_ne!(
                    reserved[i], reserved[j],
                    "reserved storage keys must not alias each other"
                );
            }
        }
    }

    #[test]
    fn test_reserved_event_topics_are_distinct() {
        let reserved = [
            ReservedEventTopics::CONFIG_UPDATE,
            ReservedEventTopics::GOV_PROPOSE,
            ReservedEventTopics::CALC_EXEC,
            ReservedEventTopics::SYS_PAUSE,
        ];
        for i in 0..reserved.len() {
            for j in (i + 1)..reserved.len() {
                assert_ne!(
                    reserved[i], reserved[j],
                    "reserved event topics must not alias each other"
                );
            }
        }
    }

    #[test]
    fn test_validate_storage_key_rejects_reserved_namespaces() {
        assert!(!validate_storage_key(&ReservedStorageKeys::CONFIG));
        assert!(!validate_storage_key(&ReservedStorageKeys::GOV));
        assert!(!validate_storage_key(&ReservedStorageKeys::HIST));
        assert!(!validate_storage_key(&ReservedStorageKeys::TELEMETRY));
        assert!(!validate_storage_key(&ReservedStorageKeys::VERSION));
    }

    #[test]
    fn test_validate_storage_key_accepts_custom_keys() {
        assert!(validate_storage_key(&symbol_short!("my_custom")));
        assert!(validate_storage_key(&symbol_short!("tenant_1")));
    }
}
