use soroban_sdk::{symbol_short, Env, Map, Symbol, Vec};

use crate::{
    SLAConfig, SLAConfigEntry, SLAConfigSnapshot, SLAError,
    CONFIG_KEY, CUSTOM_CONFIG_KEY,
    EVENT_CONFIG_UPD, EVENT_VERSION,
    config_freeze, config_metadata,
};

pub fn set_config(
    env: &Env,
    severity: Symbol,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    require_not_frozen(env)?;

    crate::SLACalculatorContract::validate_config(&severity, threshold_minutes, penalty_per_minute, reward_base)?;

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

pub fn get_config(env: &Env, severity: Symbol) -> Result<SLAConfig, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::load_config(env, &severity)
}

pub fn get_config_snapshot(env: &Env) -> Result<SLAConfigSnapshot, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;

    let mut entries = Vec::new(env);

    for severity in crate::SLACalculatorContract::canonical_severities(env) {
        let config = crate::SLACalculatorContract::load_config(env, &severity)?;
        entries.push_back(SLAConfigEntry { severity, config });
    }

    Ok(SLAConfigSnapshot {
        version: symbol_short!("v1"),
        entries,
    })
}

pub fn list_configs(env: &Env) -> Result<Map<Symbol, SLAConfig>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    env.storage()
        .instance()
        .get(&CONFIG_KEY)
        .ok_or(SLAError::NotInitialized)
}

pub fn get_config_version_hash(env: &Env) -> Result<u64, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::compute_config_version_hash(env)
}

pub fn get_last_config_update(env: &Env) -> Result<Option<crate::ConfigUpdateInfo>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(config_metadata::get_last_config_update(env).map(|seq| crate::ConfigUpdateInfo { sequence: seq }))
}

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

    crate::SLACalculatorContract::validate_general_bounds(threshold_minutes, penalty_per_minute, reward_base)?;

    let mut custom: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CUSTOM_CONFIG_KEY)
        .unwrap_or_else(|| Map::new(env));

    custom.set(
        severity.clone(),
        SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        },
    );
    env.storage().instance().set(&CUSTOM_CONFIG_KEY, &custom);

    env.events().publish(
        (EVENT_CONFIG_UPD, EVENT_VERSION, severity),
        (threshold_minutes, penalty_per_minute, reward_base),
    );
    Ok(())
}

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
        .publish((EVENT_CONFIG_UPD, EVENT_VERSION, severity), (0u32, 0i128, 0i128));
    Ok(())
}

pub fn get_custom_severity(env: &Env, severity: Symbol) -> Result<SLAConfig, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let custom: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CUSTOM_CONFIG_KEY)
        .unwrap_or_else(|| Map::new(env));
    custom.get(severity).ok_or(SLAError::SeverityNotInSet)
}

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
        version: symbol_short!("v1"),
        entries,
    })
}

fn require_not_frozen(env: &Env) -> Result<(), SLAError> {
    if config_freeze::is_config_frozen(env) {
        return Err(SLAError::ConfigFrozen);
    }
    Ok(())
}
