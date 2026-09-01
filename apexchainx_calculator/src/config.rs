//! Configuration management for canonical and custom severity levels.
//!
//! This module handles reading, writing, and snapshotting of severity-level
//! SLA configurations. It enforces validation, cross-severity ordering,
//! and freeze-state gating for all config mutations.

use soroban_sdk::{Env, Map, Symbol, Vec};

use crate::{
    config_freeze, config_metadata, SLAConfig, SLAConfigEntry, SLAConfigSnapshot, SLAError, CONFIG_KEY,
    CONFIG_SNAPSHOT_SCHEMA_VERSION, CUSTOM_CONFIG_KEY, EVENT_CONFIG_REM, EVENT_CONFIG_UPD, EVENT_SEV_ADD,
    EVENT_SEV_UPD, EVENT_VERSION,
};

/// Sets the SLA configuration for a given severity level.
///
/// Validates parameters, enforces cross-severity penalty ordering, records
/// the config update timestamp, and emits a `cfg_upd` event.
pub fn set_config(
    env: &Env,
    severity: Symbol,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    require_not_frozen(env)?;

    crate::SLACalculatorContract::validate_config(
        &severity,
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    )?;

    // Cross-severity threshold ordering: enforce critical <= high <= medium <= low
    crate::SLACalculatorContract::validate_cross_severity_threshold_ordering(
        env,
        &severity,
        threshold_minutes,
    )?;

    let mut configs: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CONFIG_KEY)
        .ok_or(SLAError::NotInitialized)?;

    configs.set(
        severity.clone(),
        SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        },
    );
    env.storage().instance().set(&CONFIG_KEY, &configs);

    config_metadata::record_config_update(env);

    env.events().publish(
        (EVENT_CONFIG_UPD, EVENT_VERSION, severity),
        (threshold_minutes, penalty_per_minute, reward_base),
    );
    Ok(())
}

/// Returns the SLA configuration for the given severity level.
pub fn get_config(env: &Env, severity: Symbol) -> Result<SLAConfig, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::load_config(env, &severity)
}

/// Returns a deterministic backend-friendly snapshot of all canonical config values.
///
/// # Canonical Config Endpoint
///
/// This is the **canonical endpoint** for reading configuration data. It returns
/// entries in a guaranteed canonical severity order (critical → high → medium → low)
/// with typed `SLAConfigEntry` structs, making it suitable for:
/// - Backend consumers that need stable ordering
/// - Serialization and diffing logic
/// - Config bundle generation
///
/// # When to Use list_configs Instead
///
/// Use `list_configs` only if you need:
/// - Raw map access for low-level inspection
/// - Direct iteration over the underlying storage map
///
/// Note that `list_configs` does not guarantee any ordering and returns raw
/// `SLAConfig` values without the typed entry wrapper.
pub fn get_config_snapshot(env: &Env) -> Result<SLAConfigSnapshot, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;

    let mut entries = Vec::new(env);

    for severity in crate::SLACalculatorContract::canonical_severities(env) {
        let config = crate::SLACalculatorContract::load_config(env, &severity)?;
        entries.push_back(SLAConfigEntry { severity, config });
    }

    Ok(SLAConfigSnapshot {
        version: CONFIG_SNAPSHOT_SCHEMA_VERSION,
        entries,
    })
}

/// Returns the full map of severity-to-config entries.
///
/// # Raw/Low-Level Config Endpoint
///
/// This is a **raw endpoint** that returns the underlying storage map directly.
/// It is provided for low-level inspection and debugging purposes.
///
/// **Important caveats:**
/// - Does **not** guarantee any ordering (map-internal ordering is SDK-dependent)
/// - Returns raw `SLAConfig` values without the typed entry wrapper
/// - Not suitable for consumers that need stable ordering across SDK versions
///
/// # Canonical Endpoint
///
/// For most use cases, use `get_config_snapshot` instead, which:
/// - Guarantees canonical severity order (critical → high → medium → low)
/// - Returns typed `SLAConfigEntry` structs with severity labels
/// - Is stable across SDK versions
pub fn list_configs(env: &Env) -> Result<Map<Symbol, SLAConfig>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    env.storage()
        .instance()
        .get(&CONFIG_KEY)
        .ok_or(SLAError::NotInitialized)
}

/// Returns a deterministic config version hash for backend cache invalidation.
pub fn get_config_version_hash(env: &Env) -> Result<u64, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::compute_config_version_hash(env)
}

/// Returns metadata about the most recent configuration update, if any.
pub fn get_last_config_update(env: &Env) -> Result<Option<crate::ConfigUpdateInfo>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(config_metadata::get_last_config_update(env).map(|seq| crate::ConfigUpdateInfo { sequence: seq }))
}

/// Registers or updates a custom (non-canonical) severity level.
///
/// # Overwrite & Lifecycle Behavior
/// - If a custom severity with the given symbol is not registered, a `sev_add`
///   (`EVENT_SEV_ADD`) event is emitted — indexers can reconstruct the
///   registered set from these creation events alone.
/// - If a custom severity with the given symbol already exists, a `sev_upd`
///   (`EVENT_SEV_UPD`) event is emitted — indexers can tell reconfiguration
///   from first registration by the distinct event name.
/// - The payload shape is identical in both cases `(threshold_minutes,
///   penalty_per_minute, reward_base)` so consumers that only care about
///   values can parse either event.
pub fn set_custom_severity(
    env: &Env,
    severity: Symbol,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    require_not_frozen(env)?;

    if crate::SLACalculatorContract::is_canonical_severity(&severity) {
        return Err(SLAError::InvalidSeverity);
    }

    crate::SLACalculatorContract::validate_general_bounds(
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    )?;

    let mut custom: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CUSTOM_CONFIG_KEY)
        .unwrap_or_else(|| Map::new(env));

    // #456 – Determine lifecycle transition before writing so the emitted
    // event distinguishes creation from reconfiguration.
    let is_update = custom.contains_key(severity.clone());

    custom.set(
        severity.clone(),
        SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        },
    );
    env.storage().instance().set(&CUSTOM_CONFIG_KEY, &custom);

    let event_name = if is_update { EVENT_SEV_UPD } else { EVENT_SEV_ADD };
    env.events().publish(
        (event_name, EVENT_VERSION, severity),
        (threshold_minutes, penalty_per_minute, reward_base),
    );
    Ok(())
}

/// Removes a previously registered custom severity level.
pub fn remove_custom_severity(env: &Env, severity: Symbol) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    require_not_frozen(env)?;

    let mut custom: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CUSTOM_CONFIG_KEY)
        .unwrap_or_else(|| Map::new(env));

    if !custom.contains_key(severity.clone()) {
        return Err(SLAError::SeverityNotInSet);
    }

    custom.remove(severity.clone());
    env.storage().instance().set(&CUSTOM_CONFIG_KEY, &custom);

    env.events()
        .publish((EVENT_CONFIG_REM, EVENT_VERSION, severity), ());
    Ok(())
}

/// Returns the SLAConfig for a registered custom severity.
pub fn get_custom_severity(env: &Env, severity: Symbol) -> Result<SLAConfig, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let custom: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CUSTOM_CONFIG_KEY)
        .unwrap_or_else(|| Map::new(env));
    custom.get(severity).ok_or(SLAError::SeverityNotInSet)
}

/// Returns a deterministic snapshot of all registered custom severity configurations.
pub fn get_custom_config_snapshot(env: &Env) -> Result<SLAConfigSnapshot, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;

    let custom: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CUSTOM_CONFIG_KEY)
        .unwrap_or_else(|| Map::new(env));

    let mut entries = Vec::new(env);
    for (severity, config) in custom.iter() {
        entries.push_back(SLAConfigEntry { severity, config });
    }

    Ok(SLAConfigSnapshot {
        version: CONFIG_SNAPSHOT_SCHEMA_VERSION,
        entries,
    })
}

fn require_not_frozen(env: &Env) -> Result<(), SLAError> {
    if config_freeze::is_config_frozen(env) {
        return Err(SLAError::ConfigFrozen);
    }
    Ok(())
}
