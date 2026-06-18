//! Configuration bundle combining config snapshot and result schema.
//!
//! A `ConfigBundle` groups the full SLA configuration snapshot together with
//! the result schema descriptor in a single read. This is the recommended way
//! for backend consumers to bootstrap their configuration cache:
//!
//! 1. Call `read_config_bundle()` once at startup
//! 2. Use the snapshot for SLA evaluation parameters
//! 3. Use the schema for interpreting SLA result symbols
//! 4. Periodically re-read to detect config changes
//!
//! # Determinism
//!
//! The bundle layout is deterministic: the same configuration always produces
//! the same snapshot and schema. This allows backends to cache and compare
//! bundles by hash rather than field-by-field comparison.

use soroban_sdk::Env;
use crate::{SLAConfigSnapshot, SLAResultSchema, CONFIG_KEY, RESULT_SCHEMA_VERSION};

/// Combined configuration and schema bundle for backend consumption.
///
/// Groups the snapshot (all severity configs in canonical order) with
/// the result schema (symbol mappings) in a single struct.
pub struct ConfigBundle {
    /// Ordered snapshot of all severity configurations.
    pub snapshot: SLAConfigSnapshot,
    /// Result schema descriptor with symbol mappings.
    pub schema: SLAResultSchema,
}

/// Reads the full configuration bundle from on-chain storage.
///
/// Returns `None` if the contract has not been initialized (no config stored).
/// The snapshot is built from the raw on-chain config map, and the schema
/// is built from the canonical `SLAResultSchema` definition.
pub fn read_config_bundle(env: &Env) -> Option<ConfigBundle> {
    let snapshot: SLAConfigSnapshot = env.storage().instance().get(&CONFIG_KEY)?;
    let schema = build_result_schema(env);
    Some(ConfigBundle { snapshot, schema })
}

fn build_result_schema(env: &Env) -> SLAResultSchema {
    use soroban_sdk::symbol_short;
    SLAResultSchema {
        version: symbol_short!("v1"),
        schema_version: RESULT_SCHEMA_VERSION,
        status_met: symbol_short!("met"),
        status_violated: symbol_short!("viol"),
        payment_reward: symbol_short!("rew"),
        payment_penalty: symbol_short!("pen"),
        rating_exceptional: symbol_short!("top"),
        rating_excellent: symbol_short!("excel"),
        rating_good: symbol_short!("good"),
        rating_poor: symbol_short!("poor"),
    }
}

#[cfg(test)]
mod tests {
    use soroban_sdk::{testutils::Address as _, Address, Env};
    use crate::{SLACalculatorContract, SLACalculatorContractClient};

    #[test]
    fn test_config_bundle_available_after_init() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        let bundle = client.get_config_bundle();
        assert!(bundle.is_some());
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
