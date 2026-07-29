//! #191 – Typed, versioned contract-info object for all read surfaces.
//!
//! This module provides `ContractInfo`, a comprehensive, versioned snapshot of
//! the contract's identity, version posture, feature set, and operational
//! status. It replaces the older, narrower `ContractMetadata` and `VersionInfo`
//! structs with a single, unified payload that backend consumers can fetch in
//! one RPC at startup.
//!
//! # Relationship to existing types
//!
//! - `ContractInfo` supersedes `ContractMetadata` (#60) and `VersionInfo`
//!   (SC-W5-029). Both legacy endpoints remain available for backward
//!   compatibility.
//! - The `schema_version` field is bumped whenever `ContractInfo` adds or
//!   removes fields. Backends should check this first before decoding.
//!
//! # Determinism
//!
//! Two calls to `get_contract_info()` within the same ledger produce
//! identical results when no state-changing operation has intervened.

use soroban_sdk::{contracttype, symbol_short, Env, Symbol, Vec};

use crate::{
    SLACalculatorContract, SLAError, RESULT_SCHEMA_VERSION, STORAGE_VERSION,
};

/// Schema version of the `ContractInfo` struct itself.
/// Increment when fields are added, removed, or reordered.
pub const CONTRACT_INFO_SCHEMA_VERSION: u32 = 1;

/// #191 – Comprehensive, versioned contract information for all read surfaces.
///
/// Backend consumers should call `get_contract_info()` once at startup (and
/// after every contract upgrade) to verify compatibility before resuming
/// operations.
///
/// # Field descriptions
///
/// | Field | Description |
/// |-------|-------------|
/// | `schema_version` | Version of this `ContractInfo` struct (bumped on schema changes) |
/// | `contract_name` | Human-readable contract name for log correlation |
/// | `contract_version` | Crate version exposed as a Symbol (e.g. "0.1.0") |
/// | `storage_version` | Current on-chain storage schema version |
/// | `result_schema_version` | SLAResult encoding schema version |
/// | `event_version` | Canonical event version symbol (e.g. "v1") |
/// | `needs_migration` | True when stored ≠ expected storage version |
/// | `is_paused` | True when the contract is currently paused |
/// | `is_config_frozen` | True when configuration is frozen |
/// | `supported_severities` | Canonical severity Symbols in canonical order |
/// | `features` | Feature flags enabled on this contract |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInfo {
    /// Version of this struct; bumped on field changes.
    pub schema_version: u32,
    /// Human-readable contract name.
    pub contract_name: Symbol,
    /// Crate version exposed as a Symbol.
    pub contract_version: Symbol,
    /// On-chain storage schema version.
    pub storage_version: u32,
    /// SLAResult encoding schema version.
    pub result_schema_version: u32,
    /// Canonical event version symbol.
    pub event_version: Symbol,
    /// True when storage version ≠ binary expectation.
    pub needs_migration: bool,
    /// True when the contract is paused.
    pub is_paused: bool,
    /// True when configuration is frozen.
    pub is_config_frozen: bool,
    /// Canonical severities in canonical order.
    pub supported_severities: Vec<Symbol>,
    /// Feature flags.
    pub features: Vec<Symbol>,
}

/// #191 – Returns the full typed, versioned contract-info object.
///
/// Backend consumers should call this at startup to verify contract
/// identity, version posture, and feature availability before resuming
/// operations. The `schema_version` field lets consumers detect when
/// new fields have been added to `ContractInfo`.
pub fn get_contract_info(env: &Env) -> Result<ContractInfo, SLAError> {
    SLACalculatorContract::check_version(env)?;

    let stored_version: u32 = env
        .storage()
        .instance()
        .get(&crate::STORAGE_VERSION_KEY)
        .unwrap_or(0);
    let needs_migration = stored_version != STORAGE_VERSION;
    let is_paused: bool = env
        .storage()
        .instance()
        .get(&crate::PAUSED_KEY)
        .unwrap_or(false);

    let is_config_frozen = crate::config_freeze::is_config_frozen(env);

    let severities = SLACalculatorContract::canonical_severities(env);

    let mut features = Vec::new(env);
    features.push_back(symbol_short!("calc"));
    features.push_back(symbol_short!("audit"));
    features.push_back(symbol_short!("pause"));
    features.push_back(symbol_short!("stats"));
    features.push_back(symbol_short!("history"));
    features.push_back(symbol_short!("failcode"));
    features.push_back(symbol_short!("safe_call"));
    features.push_back(symbol_short!("ver_nego"));
    features.push_back(symbol_short!("corr_id"));
    features.push_back(symbol_short!("freeze"));
    features.push_back(symbol_short!("ctrct_info"));

    Ok(ContractInfo {
        schema_version: CONTRACT_INFO_SCHEMA_VERSION,
        contract_name: symbol_short!("sla_calc"),
        contract_version: symbol_short!("0.1.0"),
        storage_version: STORAGE_VERSION,
        result_schema_version: RESULT_SCHEMA_VERSION,
        event_version: crate::event_schema::current_event_version(),
        needs_migration,
        is_paused,
        is_config_frozen,
        supported_severities: severities,
        features,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SLACalculatorContract;
    use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        // Initialize manually through storage since we don't have the client here
        env.as_contract(&contract_id, || {
            crate::SLACalculatorContract::initialize(
                env.clone(),
                admin.clone(),
                operator.clone(),
            )
            .unwrap();
        });
        (env, admin, operator)
    }

    #[test]
    fn test_contract_info_available_after_init() {
        let (env, _admin, _operator) = setup();
        let info = get_contract_info(&env).expect("ContractInfo must be available after init");
        assert_eq!(info.schema_version, CONTRACT_INFO_SCHEMA_VERSION);
        assert_eq!(info.contract_name, symbol_short!("sla_calc"));
        assert_eq!(info.contract_version, symbol_short!("0.1.0"));
        assert_eq!(info.storage_version, STORAGE_VERSION);
        assert_eq!(info.result_schema_version, RESULT_SCHEMA_VERSION);
        assert_eq!(info.event_version, symbol_short!("v1"));
        assert!(!info.needs_migration);
        assert!(!info.is_paused);
        assert!(!info.is_config_frozen);
    }

    #[test]
    fn test_contract_info_has_canonical_severities() {
        let (env, _admin, _operator) = setup();
        let info = get_contract_info(&env).unwrap();
        assert_eq!(info.supported_severities.len(), 4);
        assert_eq!(info.supported_severities.get(0).unwrap(), symbol_short!("critical"));
        assert_eq!(info.supported_severities.get(1).unwrap(), symbol_short!("high"));
        assert_eq!(info.supported_severities.get(2).unwrap(), symbol_short!("medium"));
        assert_eq!(info.supported_severities.get(3).unwrap(), symbol_short!("low"));
    }

    #[test]
    fn test_contract_info_has_features() {
        let (env, _admin, _operator) = setup();
        let info = get_contract_info(&env).unwrap();
        assert!(info.features.len() >= 10);
        // Verify key features are present
        let feature_strs: Vec<Symbol> = info.features;
        let has_calc = feature_strs.iter().any(|f| f == symbol_short!("calc"));
        let has_pause = feature_strs.iter().any(|f| f == symbol_short!("pause"));
        let has_ctrct_info = feature_strs.iter().any(|f| f == symbol_short!("ctrct_info"));
        assert!(has_calc);
        assert!(has_pause);
        assert!(has_ctrct_info);
    }

    #[test]
    fn test_contract_info_reflects_pause_state() {
        let (env, admin, _operator) = setup();
        // Pause the contract
        crate::metadata::pause(
            &env,
            &admin,
            soroban_sdk::String::from_str(&env, "testing"),
        )
        .unwrap();

        let info = get_contract_info(&env).unwrap();
        assert!(info.is_paused);

        // Unpause and verify
        crate::metadata::unpause(&env, &admin).unwrap();
        let info2 = get_contract_info(&env).unwrap();
        assert!(!info2.is_paused);
    }

    #[test]
    fn test_contract_info_reflects_freeze_state() {
        let (env, admin, _operator) = setup();

        crate::config_freeze::freeze_config(&env);
        let info = get_contract_info(&env).unwrap();
        assert!(info.is_config_frozen);

        crate::config_freeze::unfreeze_config(&env);
        let info2 = get_contract_info(&env).unwrap();
        assert!(!info2.is_config_frozen);
    }

    #[test]
    fn test_contract_info_detects_migration_needed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);

        env.as_contract(&contract_id, || {
            crate::SLACalculatorContract::initialize(
                env.clone(),
                admin.clone(),
                operator.clone(),
            )
            .unwrap();

            // After init, should not need migration
            let info = get_contract_info(&env).unwrap();
            assert!(!info.needs_migration);

            // Corrupt the stored version
            env.storage()
                .instance()
                .set(&crate::STORAGE_VERSION_KEY, &99u32);

            let info2 = get_contract_info(&env).unwrap();
            assert!(info2.needs_migration);
        });
    }

    #[test]
    fn test_contract_info_is_deterministic() {
        let (env, _admin, _operator) = setup();
        let info1 = get_contract_info(&env).unwrap();
        let info2 = get_contract_info(&env).unwrap();
        assert_eq!(info1, info2);
    }
}
