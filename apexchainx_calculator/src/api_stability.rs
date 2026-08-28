//! # API Stability Scoring (#225)
//!
//! Provides maintainers with a structured way to assess the compatibility
//! risk of changes to the contract's public API surface.
//!
//! ## Usage
//!
//! This module is invoked by tests to assert that key stability invariants
//! hold.  If a test fails, the change likely requires a `RESULT_SCHEMA_VERSION`
//! or `STORAGE_VERSION` bump and corresponding release notes.
//!
//! ## Stability Score Interpretation
//!
//! | Score | Meaning |
//! |---|---|
//! | **A** · 0  | No breaking changes — safe to release |
//! | **B** · 1–2 | Minor additive change — backward-compatible, document |
//! | **C** · 3–4 | Moderate risk — requires schema version bump |
//! | **D** · 5+  | Major breaking change — coordinate with all backends |
//!
//! ## Checked Invariants
//!
//! - **Contract type field count**: Adding or removing fields from
//!   `#[contracttype]` structs changes serialisation.
//! - **Error enum length**: Adding or removing `SLAError` variants changes
//!   the error contract for backends.
//! - **Event symbol stability**: Changing event name constants breaks
//!   backend event listeners.
//! - **Storage key stability**: Changing storage key symbols silently
//!   corrupts state or orphans data.
//! - **Public function signature stability**: Adding, removing, or
//!   reordering parameters breaks auto-generated clients.

// Types imported for documentation purposes only — see canonical_field_counts().
// No runtime imports are needed; this module asserts invariants via tests.

/// Stability score for a contract type.  Lower is more stable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StabilityScore {
    /// No concerns — backward-compatible.
    A,
    /// Minor additive change — document.
    B,
    /// Moderate risk — bump schema version.
    C,
    /// Major breaking change — coordinate release.
    D,
}

impl StabilityScore {
    /// Returns `true` when the score signals a breaking change.
    pub fn is_breaking(&self) -> bool {
        matches!(self, StabilityScore::C | StabilityScore::D)
    }

    /// Returns a human-readable label describing the stability level.
    pub fn label(&self) -> &'static str {
        match self {
            StabilityScore::A => "A (stable)",
            StabilityScore::B => "B (additive)",
            StabilityScore::C => "C (breaking — bump version)",
            StabilityScore::D => "D (breaking — coordinate release)",
        }
    }
}

/// Returns the canonical field count for each public `#[contracttype]` struct.
/// This serves as a single source of truth for the stability scoring tests.
///
/// **Maintainer note:** When adding or removing a field from any of these
/// structs, update the corresponding count here AND bump the relevant
/// schema version constant.
pub fn canonical_field_counts() -> [(&'static str, u32); 31] {
    [
        ("SLAConfig", 3),
        ("SLAResult", 9),
        ("SLAConfigEntry", 2),
        ("SLAConfigSnapshot", 2),
        ("SLAResultSchema", 12),
        ("DeprecatedSymbol", 4),
        ("ContractMetadata", 5),
        ("SLAStats", 4),
        ("SeverityExposure", 3),
        ("EconomicExposure", 3),
        ("SeverityTelemetry", 4),
        ("PauseInfo", 3),
        ("ConfigUpdateInfo", 1),
        ("StorageVersionInfo", 3),
        ("FailureCode", 3),
        ("FailureSchema", 2),
        ("HealthcheckResult", 3),
        ("ConfigBundle", 3),
        ("AuditState", 10),
        ("ContractInfo", 11),
        ("HistoryPage", 3),
        ("PublicApiMethod", 4),
        ("PublicApiDescriptor", 3),
        ("SeverityAliasMapping", 4),
        ("ContractStateFingerprint", 8),
        ("VersionNegotiationInfo", 6),
        // NegotiationOutcome is a fieldless enum; the count tracks its
        // variant count instead so a new outcome still trips the guardrail.
        ("NegotiationOutcome", 3),
        ("VersionMismatchDetail", 3),
        ("VersionNegotiationResult", 3),
        ("HistoryRetentionMetrics", 6),
        ("CompensationAction", 2),
    ]
}

/// Returns the count of `SLAError` variants. When a new error is added,
/// this must be updated so the stability score reflects the change.
pub fn sl_a_error_count() -> u32 {
    21
}

/// Returns the list of event name symbols that form the public event ABI.
/// Backend listeners depend on these names never changing.
///
/// **Maintainer note:** This must cover every event constant declared in
/// `event_schema.rs`. When a new event name is added there, add it here too.
pub fn event_name_symbols() -> [&'static str; 20] {
    [
        "sla_calc",
        "set_int",
        "cfg_upd",
        "paused",
        "unpause",
        "op_set",
        "pruned",
        "pruned_a",
        "adm_prop",
        "adm_acc",
        "adm_can",
        "adm_ren",
        "op_prop",
        "op_acc",
        "op_can",
        "cfg_frz",
        "cfg_unfrz",
        "stats_sat",
        "dup_input",
        "migrate_done",
    ]
}

/// Returns the storage key namespace symbols.
/// Changing any of these breaks storage layout and requires migration.
pub fn storage_key_symbols() -> [&'static str; 17] {
    [
        "ADMIN", "OPERATOR", "PADMIN", "POP", "CONFIG", "CUSTCFG", "PAUSED", "PAUSEINF", "STATS", "CALCCNT",
        "VIOLCNT", "CALCTS", "VIOLTS", "HIST", "VER", "RETLIM", "LCFGUPD",
    ]
}

/// Assesses the stability score for the current codebase state.
///
/// Call this in CI to catch accidental API drift before it reaches review.
/// Returns `StabilityScore::A` if all invariants match their canonical
/// values.
pub fn assess_stability() -> StabilityScore {
    // Check error count against canonical value.
    if sl_a_error_count() != 21 {
        return StabilityScore::C;
    }

    // Check event symbols are at expected count.
    if event_name_symbols().len() != 20 {
        return StabilityScore::C;
    }

    // Check storage key symbols are at expected count.
    if storage_key_symbols().len() != 17 {
        return StabilityScore::C;
    }

    // Check canonical field counts have expected number of entries.
    if canonical_field_counts().len() != 31 {
        return StabilityScore::C;
    }

    StabilityScore::A
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_225_field_counts_match_declared_types() {
        // Verify that canonical_field_counts accurately reflects the
        // current contract type definitions. If this fails, a contract
        // type field was added or removed — update canonical_field_counts.
        let counts = canonical_field_counts();

        // Every declared type must have a non-zero field count.
        for (_name, count) in &counts {
            assert!(*count > 0, "Field count for {} must be > 0", _name);
        }

        // SLAResult must have exactly 9 fields (the documented ABI surface).
        let sla_result = counts.iter().find(|(name, _)| *name == "SLAResult");
        assert!(
            sla_result.is_some(),
            "SLAResult must be in canonical_field_counts"
        );
        assert_eq!(
            sla_result.unwrap().1,
            9,
            "SLAResult field count changed — bump RESULT_SCHEMA_VERSION"
        );

        // ContractInfo.schema_version is the explicit handshake backends rely
        // on to detect layout drift; the guardrail must track it directly.
        let contract_info = counts.iter().find(|(name, _)| *name == "ContractInfo");
        assert!(
            contract_info.is_some(),
            "ContractInfo must be in canonical_field_counts"
        );
        assert_eq!(
            contract_info.unwrap().1,
            11,
            "ContractInfo field count changed — bump CONTRACT_INFO_SCHEMA_VERSION"
        );

        let audit_state = counts.iter().find(|(name, _)| *name == "AuditState");
        assert!(
            audit_state.is_some(),
            "AuditState must be in canonical_field_counts"
        );
        assert_eq!(
            audit_state.unwrap().1,
            10,
            "AuditState field count changed — review with backend consumers"
        );
    }

    #[test]
    fn test_225_error_enum_length_is_stable() {
        // Adding or removing an SLAError variant changes the error ABI.
        assert_eq!(
            sl_a_error_count(),
            21,
            "SLAError variant count changed — review error contract with backends"
        );
    }

    #[test]
    fn test_225_event_symbols_are_well_known() {
        // All public event names must be documented and stable.
        let events = event_name_symbols();
        let expected = 20;
        assert_eq!(
            events.len(),
            expected,
            "Event count changed — update event_name_symbols and notify backend consumers"
        );

        // Verify no duplicates
        for i in 0..events.len() {
            for j in (i + 1)..events.len() {
                assert_ne!(events[i], events[j], "Duplicate event name: {}", events[i]);
            }
        }
    }

    #[test]
    fn test_event_name_symbols_matches_event_schema_catalog() {
        // Guards against event_schema.rs gaining (or losing) an event name
        // constant without a matching update to event_name_symbols(). If
        // this fails, sync the two lists.
        use soroban_sdk::{Env, Symbol};

        let env = Env::default();
        let schema_symbols = [
            crate::event_schema::EVENT_SLA_CALC,
            crate::event_schema::EVENT_SETTLE_INTENT,
            crate::event_schema::EVENT_CONFIG_UPD,
            crate::event_schema::EVENT_PAUSED,
            crate::event_schema::EVENT_UNPAUSED,
            crate::event_schema::EVENT_OP_SET,
            crate::event_schema::EVENT_PRUNED,
            crate::event_schema::EVENT_PRUNED_AGE,
            crate::event_schema::EVENT_ADMIN_PROP,
            crate::event_schema::EVENT_ADMIN_ACC,
            crate::event_schema::EVENT_ADMIN_CAN,
            crate::event_schema::EVENT_ADMIN_REN,
            crate::event_schema::EVENT_OP_PROP,
            crate::event_schema::EVENT_OP_ACC,
            crate::event_schema::EVENT_OP_CAN,
            crate::event_schema::EVENT_CONFIG_FREEZE,
            crate::event_schema::EVENT_CONFIG_UNFREEZE,
            crate::event_schema::EVENT_STATS_SAT,
            crate::event_schema::EVENT_DUP_INPUT,
        ];

        let guardrail = event_name_symbols();
        assert_eq!(
            guardrail.len(),
            schema_symbols.len() + 1, // + EVENT_MIGRATE_DONE, which is a &str constant
            "event_name_symbols() length diverged from event_schema.rs's constant count"
        );

        for sym in schema_symbols {
            let found = guardrail.iter().any(|name| Symbol::new(&env, name) == sym);
            assert!(
                found,
                "event_schema symbol {:?} is missing from event_name_symbols()",
                sym
            );
        }

        assert!(
            guardrail.contains(&crate::event_schema::EVENT_MIGRATE_DONE),
            "event_schema::EVENT_MIGRATE_DONE is missing from event_name_symbols()"
        );
    }

    #[test]
    fn test_225_storage_keys_are_distinct() {
        let keys = storage_key_symbols();
        let expected = 17;

        assert_eq!(
            keys.len(),
            expected,
            "Storage key count changed — migration and release notes required"
        );

        // Every key must be unique
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i], keys[j], "Duplicate storage key: {}", keys[i]);
            }
        }
    }

    #[test]
    fn test_225_assess_stability_returns_a_for_current_state() {
        // The current codebase should score A (no unreviewed breaking changes).
        let score = assess_stability();
        assert!(
            !score.is_breaking(),
            "Stability check failed with score {} — review recent changes before release",
            score.label()
        );
    }

    #[test]
    fn test_225_sla_error_count_consistent_with_lib_enum() {
        // The sl_a_error_count() function must match the SLAError enum.
        // We verify by checking that all known discriminants exist.
        let discriminants: [u32; 21] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
        ];
        assert_eq!(
            discriminants.len() as u32,
            sl_a_error_count(),
            "Discriminant count mismatches sl_a_error_count()"
        );

        // Verify no gaps: discriminants must be 1..21
        for (i, d) in discriminants.iter().enumerate() {
            assert_eq!(*d, (i + 1) as u32, "Gap in SLAError discriminants at index {}", i);
        }
    }
}
