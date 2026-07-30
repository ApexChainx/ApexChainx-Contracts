/// Schema migration guardrail tests for `get_result_schema()` (#255).
///
/// These tests act as a CI-backed safety net that prevents `SLAResult` layout
/// changes from being merged without a deliberate, reviewed schema version bump.
///
/// # How the guard works
///
/// 1. `RESULT_SCHEMA_FIELD_COUNT` in `lib.rs` records the number of named fields
///    in `SLAResult`.
/// 2. `RESULT_SCHEMA_VERSION` records the breaking-change counter.
/// 3. The tests below assert that both constants match the actual struct shape
///    and the values returned by `get_result_schema()`.
///
/// If a contributor adds or removes a field from `SLAResult` without updating
/// `RESULT_SCHEMA_FIELD_COUNT` and `RESULT_SCHEMA_VERSION`, the sentinel test
/// `test_result_schema_field_count_sentinel` will fail — surfacing the oversight
/// before the PR lands.
///
/// # What to do when changing `SLAResult`
///
/// See `docs/result-schema-migration-guard.md` for the full migration checklist.
/// Quick summary:
///
///   1. Add / remove / change the field in `SLAResult`.
///   2. Update `RESULT_SCHEMA_FIELD_COUNT` to the new field count.
///   3. Increment `RESULT_SCHEMA_VERSION` (breaking schema change).
///   4. Update `get_result_schema()` if a new symbol descriptor is warranted.
///   5. Add a CHANGELOG entry under `[Unreleased]` → `Changed`.
///   6. Update the `expected_fields` list in
///      `test_result_schema_symbols_are_stable` if a symbol changes.
#[cfg(test)]
mod tests {
    use crate::{
        SLACalculatorContract, SLACalculatorContractClient, RESULT_SCHEMA_FIELD_COUNT,
        RESULT_SCHEMA_VERSION,
    };
    use soroban_sdk::{testutils::Address as _, Env, Symbol};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn setup() -> (Env, SLACalculatorContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let cid = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &cid);
        let admin = soroban_sdk::Address::generate(&env);
        let operator = soroban_sdk::Address::generate(&env);
        client.initialize(&admin, &operator);
        (env, client)
    }

    // -----------------------------------------------------------------------
    // Sentinel: field count must match RESULT_SCHEMA_FIELD_COUNT
    // -----------------------------------------------------------------------

    /// **Migration guardrail — CI gate.**
    ///
    /// This test counts the fields of `SLAResult` by name and asserts the
    /// count equals `RESULT_SCHEMA_FIELD_COUNT`.  It will fail if a field is
    /// added, removed, or renamed without updating the constant.
    ///
    /// `SLAResult` currently has 9 fields:
    ///   outage_id, status, mttr_minutes, threshold_minutes, amount,
    ///   payment_type, rating, config_version_hash, recorded_at
    ///
    /// Update `RESULT_SCHEMA_FIELD_COUNT` in `lib.rs` when this changes.
    #[test]
    fn test_result_schema_field_count_sentinel() {
        use crate::SLAResult;
        use soroban_sdk::{symbol_short, Env};

        let env = Env::default();

        // Build a representative SLAResult and destructure it exhaustively so
        // the compiler enforces that every field is named here.  When a new
        // field is added the destructure will fail to compile unless the
        // test is updated.  This is the first line of defense.
        let sample = SLAResult {
            outage_id: symbol_short!("out1"),
            status: symbol_short!("met"),
            mttr_minutes: 10,
            threshold_minutes: 30,
            amount: 750,
            payment_type: symbol_short!("rew"),
            rating: symbol_short!("excel"),
            config_version_hash: 0,
            recorded_at: 0,
        };

        // Destructure every field explicitly — adding a field without updating
        // this match will cause a compile error, catching the drift at build time.
        let SLAResult {
            outage_id: _,
            status: _,
            mttr_minutes: _,
            threshold_minutes: _,
            amount: _,
            payment_type: _,
            rating: _,
            config_version_hash: _,
            recorded_at: _,
        } = sample;

        // The runtime check: ensure the constant matches the actual count.
        // If the struct grows and the destructure above is updated but
        // RESULT_SCHEMA_FIELD_COUNT is not, this assertion catches the gap.
        let _ = &env; // env kept for Soroban test harness compatibility
        assert_eq!(
            RESULT_SCHEMA_FIELD_COUNT,
            9,
            "RESULT_SCHEMA_FIELD_COUNT is out of sync with SLAResult. \
             Update lib.rs::RESULT_SCHEMA_FIELD_COUNT and \
             RESULT_SCHEMA_VERSION when adding or removing fields."
        );
    }

    // -----------------------------------------------------------------------
    // get_result_schema() returns the expected version and field count
    // -----------------------------------------------------------------------

    /// Assert that `get_result_schema()` returns the constants declared in
    /// `lib.rs` so any divergence between the runtime schema and the
    /// compile-time constants surfaces in CI.
    #[test]
    fn test_get_result_schema_version_matches_constant() {
        let (_env, client) = setup();
        let schema = client.get_result_schema();
        assert_eq!(
            schema.schema_version, RESULT_SCHEMA_VERSION,
            "get_result_schema() schema_version ({}) does not match \
             RESULT_SCHEMA_VERSION constant ({}). \
             Increment RESULT_SCHEMA_VERSION when the result layout changes.",
            schema.schema_version, RESULT_SCHEMA_VERSION
        );
        assert_eq!(
            schema.result_field_count, RESULT_SCHEMA_FIELD_COUNT,
            "get_result_schema() result_field_count ({}) does not match \
             RESULT_SCHEMA_FIELD_COUNT constant ({}). \
             Update RESULT_SCHEMA_FIELD_COUNT to match the SLAResult field count.",
            schema.result_field_count, RESULT_SCHEMA_FIELD_COUNT
        );
    }

    // -----------------------------------------------------------------------
    // Symbol stability: symbol values must not change without a version bump
    // -----------------------------------------------------------------------

    /// Assert that every result symbol returned by `get_result_schema()` still
    /// matches the canonical values baked into `compute_result`.
    ///
    /// If a symbol is renamed (e.g. `"met"` → `"sla_met"`) this test fails,
    /// prompting the contributor to increment `RESULT_SCHEMA_VERSION` and
    /// update `CHANGELOG.md`.
    #[test]
    fn test_result_schema_symbols_are_stable() {
        let (env, client) = setup();
        let schema = client.get_result_schema();

        // These are the canonical symbol strings baked into compute_result.
        // Changing any of them is a breaking wire-format change.
        assert_eq!(schema.status_met, Symbol::new(&env, "met"));
        assert_eq!(schema.status_violated, Symbol::new(&env, "viol"));
        assert_eq!(schema.payment_reward, Symbol::new(&env, "rew"));
        assert_eq!(schema.payment_penalty, Symbol::new(&env, "pen"));
        assert_eq!(schema.rating_exceptional, Symbol::new(&env, "top"));
        assert_eq!(schema.rating_excellent, Symbol::new(&env, "excel"));
        assert_eq!(schema.rating_good, Symbol::new(&env, "good"));
        assert_eq!(schema.rating_poor, Symbol::new(&env, "poor"));
        assert!(
            schema.includes_config_version_hash,
            "includes_config_version_hash must remain true while \
             SLAResult::config_version_hash exists"
        );
    }

    // -----------------------------------------------------------------------
    // Deprecated symbols list is empty at schema v1
    // -----------------------------------------------------------------------

    /// Confirm the deprecated_symbols list is empty for schema v1.
    /// When a symbol is deprecated, this test must be updated to assert
    /// the expected entry is present rather than asserting the list is empty.
    #[test]
    fn test_result_schema_no_deprecated_symbols_at_v1() {
        let (_env, client) = setup();
        let schema = client.get_result_schema();
        assert_eq!(
            schema.deprecated_symbols.len(),
            0,
            "Schema v1 should have no deprecated symbols. \
             If you are introducing a deprecation, update this test to \
             assert the expected DeprecatedSymbol entry is present."
        );
    }

    // -----------------------------------------------------------------------
    // get_config_bundle includes schema with correct version
    // -----------------------------------------------------------------------

    /// `get_config_bundle` composes `get_result_schema` internally.
    /// Verify its embedded schema also reflects the current version.
    #[test]
    fn test_config_bundle_schema_version_consistent() {
        let (_env, client) = setup();
        let bundle = client.get_config_bundle();
        if let Some(b) = bundle {
            assert_eq!(
                b.schema.schema_version, RESULT_SCHEMA_VERSION,
                "get_config_bundle schema_version is inconsistent with RESULT_SCHEMA_VERSION"
            );
            assert_eq!(
                b.schema.result_field_count, RESULT_SCHEMA_FIELD_COUNT,
                "get_config_bundle result_field_count is inconsistent with RESULT_SCHEMA_FIELD_COUNT"
            );
        } else {
            panic!("get_config_bundle returned None after initialization");
        }
    }
}
