use soroban_sdk::{symbol_short, Symbol};

/// Centralized registry for reserved storage keys
pub struct ReservedStorageKeys;

impl ReservedStorageKeys {
    pub const CONFIG: Symbol = symbol_short!("Config");
    pub const GOV: Symbol = symbol_short!("Gov");
    pub const HIST: Symbol = symbol_short!("Hist");
    pub const TELEMETRY: Symbol = symbol_short!("Telemetry");
    pub const VERSION: Symbol = symbol_short!("Version");
}

/// Centralized registry for reserved event-topic namespaces
pub struct ReservedEventTopics;

impl ReservedEventTopics {
    pub const CONFIG_UPDATE: Symbol = symbol_short!("cfg_upd");
    pub const GOV_PROPOSE: Symbol = symbol_short!("gov_prop");
    pub const CALC_EXEC: Symbol = symbol_short!("calc_ex");
    pub const SYS_PAUSE: Symbol = symbol_short!("sys_pause");
}

/// Validates that a custom storage key does not violate reserved prefixes
pub fn validate_storage_key(key: &Symbol) -> bool {
    // Prevent overriding core reserved namespaces
    key != &ReservedStorageKeys::CONFIG
        && key != &ReservedStorageKeys::GOV
        && key != &ReservedStorageKeys::HIST
        && key != &ReservedStorageKeys::TELEMETRY
        && key != &ReservedStorageKeys::VERSION
}