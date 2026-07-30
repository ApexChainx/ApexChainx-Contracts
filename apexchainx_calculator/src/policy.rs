//! Centralized policy registry for reserved storage keys and event topics.
//!
//! This module defines the authoritative registries of reserved namespaces
//! for storage keys and event topics, plus validation helpers to prevent
//! collisions with custom keys.

use soroban_sdk::{symbol_short, Symbol};

/// Centralized registry for reserved storage key namespaces.
///
/// These keys are reserved for core contract functionality and must not be
/// overwritten by custom or user-defined keys.
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
/// Returns `true` if the key is safe to use, `false` if it overlaps with a
/// reserved prefix.
pub fn validate_storage_key(key: &Symbol) -> bool {
    // Prevent overriding core reserved namespaces
    key != &ReservedStorageKeys::CONFIG
        && key != &ReservedStorageKeys::GOV
        && key != &ReservedStorageKeys::HIST
        && key != &ReservedStorageKeys::TELEMETRY
        && key != &ReservedStorageKeys::VERSION
}
