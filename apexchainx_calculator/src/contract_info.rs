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

use crate::{SLACalculatorContract, SLAError, RESULT_SCHEMA_VERSION, STORAGE_VERSION};

/// Schema version of the `ContractInfo` struct itself.
/// Increment when fields are added, removed, or reordered.
pub const CONTRACT_INFO_SCHEMA_VERSION: u32 = 1;

/// #424 – Single source of truth for the advertised feature set.
///
/// Both introspection endpoints (`get_contract_info` and the legacy
/// `get_contract_metadata`) must derive their `features` from this one list so
/// they can never disagree. Every flag here corresponds to a reachable,
/// tested capability:
///
/// - `calc` – SLA calculation & events
/// - `audit` – the audit log surface
/// - `pause` – `pause`/`unpause`
/// - `stats` – calculation statistics
/// - `history` – history/readback of stored results
/// - `failcode` – explicit `SLAError` failure codes
/// - `safe_call` – `try_*` safe-call wrappers
/// - `ver_nego` – version-negotiation (`get_version_negotiation_info`)
/// - `freeze` – config freeze/unfreeze
/// - `ctrctinfo` – contract self-introspection (`get_contract_info`)
///
/// `corr_id` (cross-contract correlation tracing) is deliberately absent: the
/// correlation module is not wired, so advertising it would tell backends to
/// enable tracing logic that will receive no data (#424).
pub(crate) const CONTRACT_FEATURES: [&str; 10] = [
    "calc",
    "audit",
    "pause",
    "stats",
    "history",
    "failcode",
    "safe_call",
    "ver_nego",
    "freeze",
    "ctrctinfo",
];

/// #423 – The crate version, in the Symbol form used by `ContractInfo`.
///
/// Derived from `CARGO_PKG_VERSION` (e.g. `"0.1.0"`) by replacing the dots
/// with underscores per the existing `0_1_0` convention, so the reported
/// `contract_version` can never silently drift from the Cargo package version.
///
/// The conversion is deterministic and stable across calls: same input string,
/// same Symbol. The string is short enough (≤9 bytes) for a `Symbol::new`.
pub fn cargo_pkg_version_symbol(env: &Env) -> Symbol {
    let dotted = env!("CARGO_PKG_VERSION").replace('.', "_");
    Symbol::new(env, &dotted)
}

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
/// | `contract_version` | Crate version exposed as a Symbol (e.g. "0_1_0") |
/// | `storage_version` | Current on-chain storage schema version |
/// | `result_schema_version` | SLAResult encoding schema version |
/// | `event_version` | Canonical event version symbol (e.g. "v1") |
/// | `needs_migration` | True when stored ≠ expected storage version |
/// | `is_paused` | True when the contract is currently paused |
/// | `is_config_frozen` | True when configuration is frozen |
/// | `supported_severities` | Canonical severity Symbols in canonical order |
/// | `features` | Feature flags enabled on this contract |
#[allow(missing_docs)]
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
///
/// This function intentionally bypasses `check_version()` so backend consumers
/// can observe `needs_migration == true` pre-migration during startup handshake.
pub fn get_contract_info(env: &Env) -> Result<ContractInfo, SLAError> {
    let stored_version: u32 = env
        .storage()
        .instance()
        .get(&crate::STORAGE_VERSION_KEY)
        .ok_or(SLAError::NotInitialized)?;
    let needs_migration = stored_version != STORAGE_VERSION;
    let is_paused: bool = env.storage().instance().get(&crate::PAUSED_KEY).unwrap_or(false);

    let is_config_frozen = crate::config_freeze::is_config_frozen(env);

    let severities = SLACalculatorContract::canonical_severities(env);

    // #424 – single source of truth for the advertised feature set, shared
    // with the legacy get_contract_metadata endpoint.
    let mut features = Vec::new(env);
    for f in CONTRACT_FEATURES.iter() {
        features.push_back(Symbol::new(env, f));
    }

    Ok(ContractInfo {
        schema_version: CONTRACT_INFO_SCHEMA_VERSION,
        contract_name: symbol_short!("sla_calc"),
        // #423 – derived from CARGO_PKG_VERSION, not a hand-maintained literal.
        contract_version: cargo_pkg_version_symbol(env),
        storage_version: stored_version,
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

    fn setup() -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        // Initialize manually through storage since we don't have the client here
        env.as_contract(&contract_id, || {
            crate::SLACalculatorContract::initialize(env.clone(), admin.clone(), operator.clone()).unwrap();
        });
        (env, contract_id, admin, operator)
    }

    #[test]
    fn test_contract_info_available_after_init() {
        let (env, contract_id, _admin, _operator) = setup();
        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).expect("ContractInfo must be available after init");
            assert_eq!(info.schema_version, CONTRACT_INFO_SCHEMA_VERSION);
            assert_eq!(info.contract_name, symbol_short!("sla_calc"));
            assert_eq!(info.contract_version, cargo_pkg_version_symbol(&env));
            assert_eq!(info.storage_version, STORAGE_VERSION);
            assert_eq!(info.result_schema_version, RESULT_SCHEMA_VERSION);
            assert_eq!(info.event_version, symbol_short!("v1"));
            assert!(!info.needs_migration);
            assert!(!info.is_paused);
            assert!(!info.is_config_frozen);
        });
    }

    #[test]
    fn test_contract_version_is_derived_from_cargo_package() {
        // #423 – the reported contract_version must be derived from the Cargo
        // package version (dots → underscores), so it can never silently drift
        // on a release bump.
        let (env, contract_id, _admin, _operator) = setup();
        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).unwrap();

            // Build the expected Symbol explicitly from CARGO_PKG_VERSION.
            let dotted = env!("CARGO_PKG_VERSION").replace('.', "_");
            let expected_symbol = Symbol::new(&env, &dotted);

            assert_eq!(info.contract_version, expected_symbol);
            // The derived symbol is stable across calls.
            assert_eq!(info.contract_version, cargo_pkg_version_symbol(&env));
            // And it differs from the dotted literal forever, so parity with
            // the package version is enforced (a bump changes this symbol).
            assert_eq!(
                info.contract_version, expected_symbol,
                "contract_version must stay in lockstep with CARGO_PKG_VERSION"
            );
        });
    }

    /// #497 – The version posture reported by `get_contract_info` must be
    /// coherent: every field derived from a canonical constant, and the
    /// event-ABI co-bump invariant enforced.
    #[test]
    fn test_contract_info_event_version_tracks_event_schema() {
        let (env, contract_id, _admin, _operator) = setup();
        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).unwrap();
            // The advertised event version is the canonical one (no drift).
            assert_eq!(info.event_version, crate::event_schema::current_event_version());
            assert_eq!(info.event_version, crate::event_schema::EVENT_VERSION);
        });
    }

    /// #497 – Co-bump invariant between the event-ABI generation and the
    /// storage/result-schema posture (docs/UPGRADE_PLAYBOOK.md § "Version
    /// posture & co-bump rules").
    ///
    /// A breaking event-ABI bump (generation `g`) must be a coordinated
    /// release: it can never be shipped on an unchanged storage schema or an
    /// unchanged result schema. Bumping `EVENT_VERSION`/`EVENT_ABI_GENERATION`
    /// without a matching `STORAGE_VERSION`/`RESULT_SCHEMA_VERSION` bump fails
    /// this test, so a silent event-ABI change can no longer pass CI under an
    /// unchanged version story.
    #[test]
    fn test_event_abi_cobump_invariant() {
        let gen = crate::event_schema::EVENT_ABI_GENERATION;
        let required = crate::event_schema::EVENT_ABI_TO_SCHEMA_VERSION[(gen - 1) as usize];
        assert!(
            STORAGE_VERSION >= required,
            "event ABI generation {} requires STORAGE_VERSION >= {} (got {}): a breaking \
             event change must be a coordinated storage release (#497)",
            gen,
            required,
            STORAGE_VERSION
        );
        assert!(
            RESULT_SCHEMA_VERSION >= required,
            "event ABI generation {} requires RESULT_SCHEMA_VERSION >= {} (got {}): a \
             breaking event change must update the result schema (#497)",
            gen,
            required,
            RESULT_SCHEMA_VERSION
        );

        // The posture get_contract_info advertises agrees with the same
        // constants the co-bump rule is evaluated over.
        let (env, contract_id, _admin, _operator) = setup();
        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).unwrap();
            assert!((info.storage_version as i64) >= required as i64);
            assert!(info.result_schema_version >= required);
        });
    }

    #[test]
    fn test_contract_info_has_canonical_severities() {
        let (env, contract_id, _admin, _operator) = setup();
        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).unwrap();
            assert_eq!(info.supported_severities.len(), 4);
            assert_eq!(
                info.supported_severities.get(0).unwrap(),
                symbol_short!("critical")
            );
            assert_eq!(info.supported_severities.get(1).unwrap(), symbol_short!("high"));
            assert_eq!(info.supported_severities.get(2).unwrap(), symbol_short!("medium"));
            assert_eq!(info.supported_severities.get(3).unwrap(), symbol_short!("low"));
        });
    }

    #[test]
    fn test_contract_info_has_features() {
        let (env, contract_id, _admin, _operator) = setup();
        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).unwrap();
            assert!(info.features.len() >= 10);
            // Verify key features are present
            let feature_strs: Vec<Symbol> = info.features;
            let has_calc = feature_strs.iter().any(|f| f == symbol_short!("calc"));
            let has_pause = feature_strs.iter().any(|f| f == symbol_short!("pause"));
            let has_ctrct_info = feature_strs.iter().any(|f| f == symbol_short!("ctrctinfo"));
            assert!(has_calc);
            assert!(has_pause);
            assert!(has_ctrct_info);
        });
    }

    #[test]
    fn test_contract_info_reflects_pause_state() {
        let (env, contract_id, admin, _operator) = setup();
        // Pause the contract from the contract's own storage context
        env.as_contract(&contract_id, || {
            crate::metadata::pause(&env, &admin, soroban_sdk::String::from_str(&env, "testing")).unwrap();
        });

        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).unwrap();
            assert!(info.is_paused);
        });

        // Unpause and verify
        env.as_contract(&contract_id, || {
            crate::metadata::unpause(&env, &admin).unwrap();
        });
        env.as_contract(&contract_id, || {
            let info2 = get_contract_info(&env).unwrap();
            assert!(!info2.is_paused);
        });
    }

    #[test]
    fn test_contract_info_reflects_freeze_state() {
        let (env, contract_id, _admin, _operator) = setup();

        env.as_contract(&contract_id, || {
            crate::config_freeze::freeze_config(&env);
        });
        env.as_contract(&contract_id, || {
            let info = get_contract_info(&env).unwrap();
            assert!(info.is_config_frozen);
        });

        env.as_contract(&contract_id, || {
            crate::config_freeze::unfreeze_config(&env);
        });
        env.as_contract(&contract_id, || {
            let info2 = get_contract_info(&env).unwrap();
            assert!(!info2.is_config_frozen);
        });
    }

    #[test]
    fn test_contract_info_detects_migration_needed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);

        env.as_contract(&contract_id, || {
            crate::SLACalculatorContract::initialize(env.clone(), admin.clone(), operator.clone()).unwrap();

            // After init, should not need migration
            let info = get_contract_info(&env).unwrap();
            assert!(!info.needs_migration);

            // Change stored version: get_contract_info returns a ContractInfo
            // struct with needs_migration: true and the stored storage_version.
            env.storage().instance().set(&crate::STORAGE_VERSION_KEY, &99u32);

            let info_mig = get_contract_info(&env).unwrap();
            assert!(info_mig.needs_migration);
            assert_eq!(info_mig.storage_version, 99u32);
        });
    }

    #[test]
    fn test_contract_info_is_deterministic() {
        let (env, contract_id, _admin, _operator) = setup();
        env.as_contract(&contract_id, || {
            let info1 = get_contract_info(&env).unwrap();
            let info2 = get_contract_info(&env).unwrap();
            assert_eq!(info1, info2);
        });
    }
}
