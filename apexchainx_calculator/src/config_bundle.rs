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
//! # Bundle Cache (Issue #668)
//!
//! `get_config_bundle` previously rebuilt the snapshot on every call even
//! when the config hash was unchanged, wasting CPU and storage reads on
//! every backend poll. The bundle is now cached under `BUNDLE_CACHE_KEY`
//! and served from the cache whenever the stored `config_version_hash`
//! matches the current hash. Any config write invalidates the cache so
//! the next read recomposes and re-caches a fresh bundle.
//!
//! # #1 – type mismatch / runtime deserialization failure
//!
//! `ConfigBundle` is annotated with `#[contracttype]` so the auto-generated
//! Soroban contract client can (de)serialise it across the contract ↔ host
//! boundary. Without that derive, a contract method returning a
//! `ConfigBundle` would fail to compile, and any cross-boundary usage would
//! surface as a type mismatch at the host boundary – the root cause tracked
//! in issue #1.
//!
//! # Low-severity exemption for consumers (#594)
//!
//! When reading a bundle, do **not** flag or reject a configuration where
//! `low.penalty > medium.penalty`. Low is intentionally exempt from the
//! cross-severity penalty ladder (its cap of 100 exceeds medium's minimum of
//! 10), so an inverted low segment is a state the contract deliberately
//! admits. See `docs/config-validation.md` ("Penalty ladder & the
//! low-severity exemption").

use soroban_sdk::{contracttype, symbol_short, Env, Symbol};

use crate::{SLAConfigSnapshot, SLAResultSchema};

/// On-chain key for the cached `ConfigBundle`.
///
/// Written on every `get_config_bundle` call that recomposes the bundle
/// (i.e. when the current config hash differs from the cached hash).
/// Removed by `invalidate_config_bundle_cache` on any config write.
pub(crate) const BUNDLE_CACHE_KEY: Symbol = symbol_short!(\"BNDLCCH\");

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
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigBundle {
    /// Ordered snapshot of all severity configurations.
    pub snapshot: SLAConfigSnapshot,
    /// Result schema descriptor with symbol mappings.
    pub schema: SLAResultSchema,
    /// Config version hash corresponding to the snapshot for duplicate detection.
    pub config_version_hash: u64,
}

/// Invalidates the cached `ConfigBundle`.
///
/// Must be called by every config-write path (e.g. `set_config`,
/// `freeze_config`, `unfreeze_config`) so that the next `get_config_bundle`
/// call recomposes a fresh bundle rather than serving a stale cache entry.
pub fn invalidate_config_bundle_cache(env: &Env) {
    env.storage().instance().remove(&BUNDLE_CACHE_KEY);
}

/// Read the `ConfigBundle` from cache if the version hash is unchanged,
/// otherwise recompose, cache, and return the fresh bundle.
///
/// This is the internal helper used by `SLACalculatorContract::get_config_bundle`.
pub fn read_config_bundle(env: &Env, current_hash: u64) -> Option<ConfigBundle> {
    // Serve from cache when the hash hasn't changed.
    if let Some(cached) = env
        .storage()
        .instance()
        .get::<Symbol, ConfigBundle>(&BUNDLE_CACHE_KEY)
    {
        if cached.config_version_hash == current_hash {
            return Some(cached);
        }
    }

    // Recompose the bundle with the current snapshot and schema.
    let snapshot = crate::SLACalculatorContract::build_config_snapshot(env)?;
    let schema = crate::SLACalculatorContract::build_result_schema(env);
    let bundle = ConfigBundle {
        snapshot,
        schema,
        config_version_hash: current_hash,
    };

    // Cache for subsequent unchanged-hash reads.
    env.storage().instance().set(&BUNDLE_CACHE_KEY, &bundle);
    Some(bundle)
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
        let b = bundle.unwrap();
        assert_eq!(
            b.config_version_hash,
            client.get_config_version_hash(),
            "ConfigBundle config_version_hash must match get_config_version_hash()",
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
        client.set_config(&admin, &symbol_short!("high"), &42, &50, &750);
        client.set_config(&admin, &symbol_short!("critical"), &42, &111, &999);

        let bundle = client
            .get_config_bundle()
            .expect("bundle must be available after init");
        let entry = bundle.snapshot.entries.get(0).unwrap();
        assert_eq!(entry.severity, symbol_short!("critical"));
        assert_eq!(entry.config.threshold_minutes, 42);
        assert_eq!(entry.config.penalty_per_minute, 111);
        assert_eq!(entry.config.reward_base, 999);
        assert_eq!(
            bundle.config_version_hash,
            client.get_config_version_hash(),
            "Bundle config_version_hash must update when config changes",
        );
    }

    #[test]
    fn test_config_bundle_round_trips_through_client() {
        let (_env, client, _admin) = setup();

        let a = client
            .get_config_bundle()
            .expect("bundle must be available after init");
        let b = client
            .get_config_bundle()
            .expect("bundle must be available after init");

        assert_eq!(a.snapshot, b.snapshot);
        assert_eq!(a.schema, b.schema);
        assert_eq!(a.config_version_hash, b.config_version_hash);
        assert_eq!(a.snapshot.entries.len(), 4);
        assert_eq!(a.schema.status_met, symbol_short!("met"));
    }

    /// Issue #668: unchanged-hash reads must be served from cache (no recompose).
    /// After a write the bundle must reflect the new config, proving cache
    /// invalidation on write works correctly.
    #[test]
    fn test_config_bundle_cache_serves_unchanged_hash_reads() {
        let (_env, client, admin) = setup();

        let first = client
            .get_config_bundle()
            .expect("bundle available after init");

        // Second read with unchanged config — same hash, same bundle.
        let second = client
            .get_config_bundle()
            .expect("bundle available on second read");

        assert_eq!(
            first, second,
            "Repeated reads with unchanged config must return identical bundles (cache hit)",
        );

        // Mutate config — cache must be invalidated.
        client.set_config(&admin, &symbol_short!("high"), &50, &60, &800);

        let after_write = client
            .get_config_bundle()
            .expect("bundle available after write");

        assert_ne!(
            first.config_version_hash, after_write.config_version_hash,
            "Config write must invalidate the bundle cache and return a fresh hash",
        );
    }

    // -----------------------------------------------------------------
    // Error-path coverage
    // -----------------------------------------------------------------

    #[test]
    #[should_panic]
    fn test_config_bundle_panics_before_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
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

        env.as_contract(&contract_id, || {
            env.storage().instance().set(&crate::STORAGE_VERSION_KEY, &99u32);
        });

        client.get_config_bundle();
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_call_set_config_via_contract() {
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