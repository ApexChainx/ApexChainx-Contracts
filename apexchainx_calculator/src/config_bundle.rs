//! Configuration bundle combining config snapshot and result schema (#1).
//!
//! A `ConfigBundle` groups the full SLA configuration snapshot together with
//! the result schema descriptor in a single read. This is the recommended way
//! for backend consumers to bootstrap their configuration cache in one RPC:
//!
//! 1. Call `SLACalculatorContract::get_config_bundle()` once at startup
//! 2. Use the snapshot for SLA evaluation parameters
//! 3. Use the schema for interpreting SLA result symbols
//! 4. Periodically re-read to detect config or schema changes
//!
//! # Determinism
//!
//! The bundle layout is deterministic and identical to composing
//! `get_config_snapshot()` with `get_result_schema()`. Backends may cache
//! and compare bundles by hash instead of field-by-field comparison.
//!
//! # #1 – type mismatch / runtime deserialization failure
//!
//! `ConfigBundle` is annotated with `#[contracttype]` so the auto-generated
//! Soroban contract client can (de)serialise it across the contract ↔ host
//! boundary. Without that derive, a contract method returning a
//! `ConfigBundle` would fail to compile, and any cross-boundary usage would
//! surface as a type mismatch at the host boundary – the root cause tracked
//! in issue #1.

use soroban_sdk::contracttype;

use crate::{SLAConfigSnapshot, SLAResultSchema};

/// Combined configuration and schema bundle for backend consumption.
///
/// Groups the snapshot (all severity configs in canonical order) with the
/// result schema (symbol mappings) in a single struct.
///
/// `#[contracttype]` is required so this type can be used as the return
/// value of a `#[contractimpl]` method and cross the contract → host →
/// SDK client boundary intact. `Clone`, `Debug`, `Eq`, and `PartialEq` are
/// derived to match the conventions used by every other `#[contracttype]`
/// struct in this contract (e.g. `SLAResult`, `VersionInfo`).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigBundle {
    /// Ordered snapshot of all severity configurations.
    pub snapshot: SLAConfigSnapshot,
    /// Result schema descriptor with symbol mappings.
    pub schema: SLAResultSchema,
}

#[cfg(test)]
mod tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

    use crate::{SLACalculatorContract, SLACalculatorContractClient};

    fn setup() -> (Env, SLACalculatorContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        (env, client, admin)
    }

    #[test]
    fn test_config_bundle_available_after_init() {
        let (_env, client, _admin) = setup();
        let bundle = client.get_config_bundle();
        assert!(
            bundle.is_some(),
            "ConfigBundle must be available after initialize()",
        );
    }

    #[test]
    fn test_config_bundle_snapshot_matches_get_config_snapshot() {
        let (_env, client, _admin) = setup();

        let bundle = client
            .get_config_bundle()
            .expect("bundle must be available after init");
        let snapshot = client.get_config_snapshot();

        assert_eq!(
            bundle.snapshot, snapshot,
            "Bundle snapshot must equal the dedicated snapshot endpoint",
        );
    }

    #[test]
    fn test_config_bundle_schema_matches_get_result_schema() {
        let (_env, client, _admin) = setup();

        let bundle = client
            .get_config_bundle()
            .expect("bundle must be available after init");
        let schema = client.get_result_schema();

        assert_eq!(
            bundle.schema, schema,
            "Bundle schema must equal the dedicated schema endpoint",
        );
        assert!(
            bundle.schema.includes_config_version_hash,
            "Bundle schema must preserve includes_config_version_hash = true",
        );
    }

    #[test]
    fn test_config_bundle_reflects_admin_config_updates() {
        let (_env, client, admin) = setup();

        // Apply a valid config update and verify the bundle picks it up.
        // Values stay inside the per-severity bounds enforced by
        // `validate_config`: critical allows threshold≤60 and penalty≥50.
        client.set_config(&admin, &symbol_short!("critical"), &42, &111, &999);

        let bundle = client
            .get_config_bundle()
            .expect("bundle must be available after init");
        let entry = bundle.snapshot.entries.get(0).unwrap();
        assert_eq!(entry.severity, symbol_short!("critical"));
        assert_eq!(entry.config.threshold_minutes, 42);
        assert_eq!(entry.config.penalty_per_minute, 111);
        assert_eq!(entry.config.reward_base, 999);
    }

    #[test]
    fn test_config_bundle_round_trips_through_client() {
        let (_env, client, _admin) = setup();

        // Pull the bundle via the auto-generated client twice; the Soroban
        // cross-boundary (de)serialisation must produce a structurally
        // identical value, not silently drop or coerce fields along the way.
        let a = client
            .get_config_bundle()
            .expect("bundle must be available after init");
        let b = client
            .get_config_bundle()
            .expect("bundle must be available after init");

        assert_eq!(a.snapshot, b.snapshot);
        assert_eq!(a.schema, b.schema);
        assert_eq!(a.snapshot.entries.len(), 4);
        assert_eq!(a.schema.status_met, symbol_short!("met"));
    }

    // -----------------------------------------------------------------
    // Error-path coverage: prove the Result envelope surfaces typed
    // errors rather than returning a malformed bundle to the caller.
    // -----------------------------------------------------------------

    #[test]
    #[should_panic]
    fn test_config_bundle_panics_before_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        // No initialize() call: client.get_config_bundle() must panic
        // through the SLAError::NotInitialized path. This is the same
        // pattern as `test_check_version_rejects_version_mismatch` in
        // tests.rs — proves the error envelope is wired end-to-end.
        client.get_config_bundle();
    }

    #[test]
    #[should_panic]
    fn test_config_bundle_panics_on_storage_version_mismatch() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        // Stamp a future schema version so check_version rejects the call.
        env.as_contract(&contract_id, || {
            env.storage().instance().set(&crate::STORAGE_VERSION_KEY, &99u32);
        });

        // Must panic with VersionMismatch.
        client.get_config_bundle();
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_call_set_config_via_contract() {
        // Negative auth test: even though this module exposes only the
        // read-side `read_config_bundle()`, the underlying contract enforces
        // admin-only writes. A stranger attempting to mutate the config
        // through the contract client must be rejected.
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        let stranger = Address::generate(&env);
        client.set_config(
            &stranger,
            &soroban_sdk::symbol_short!("critical"),
            &15,
            &100,
            &750,
        );
    }
}
