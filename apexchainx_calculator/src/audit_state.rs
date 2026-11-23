//! Combined audit-state envelope for one-shot backend bootstrap reads.

use soroban_sdk::{contracttype, Address, Vec};

use crate::{PauseInfo, SLAConfigSnapshot, SLAResultSchema, SLAStats};

/// Combined audit-state envelope for one-shot backend bootstrap reads (#107).
///
/// Groups all contract state (roles, config, stats, history, schema) into a
/// single read for backend consumers.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditState {
    /// Current admin address.
    pub admin: Address,
    /// Current operator address.
    pub operator: Address,
    /// Pending admin address (two-step transfer), if any.
    pub pending_admin: Option<Address>,
    /// Pending operator address (two-step handoff), if any.
    pub pending_operator: Option<Address>,
    /// Whether the contract is currently paused.
    pub paused: bool,
    /// Whether the configuration is currently frozen. Freeze is a first-class
    /// governance posture: while frozen, `set_config` (and admin role changes)
    /// are rejected. It must be visible to `get_full_audit_state` so a backend
    /// polling the audit endpoint can distinguish frozen from thawed without
    /// attempting a write and hitting `ConfigFrozen` (#577).
    pub config_frozen: bool,
    /// Pause metadata when paused, empty otherwise. Follows the crate-wide
    /// optional-state convention for `#[contracttype]` fields (CODING_STYLE.md
    /// Part 3, #493): `Option<T>` cannot be a `#[contracttype]` field, so a
    /// `Vec<T>` stands in with a max-length invariant — empty = absent,
    /// single-element = present. INVARIANT: `pause_info.len() <= 1`; the
    /// getter surfaces (`get_pause_info`) use `Option<PauseInfo>` directly.
    pub pause_info: Vec<PauseInfo>,
    /// Ordered snapshot of all severity configurations.
    pub config_snapshot: SLAConfigSnapshot,
    /// Cumulative SLA performance statistics.
    pub stats: SLAStats,
    /// Number of history entries currently stored on-chain.
    pub history_len: u32,
    /// Result schema descriptor with symbol mappings.
    pub result_schema: SLAResultSchema,
}

#[cfg(test)]
mod tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

    use crate::{SLACalculatorContract, SLACalculatorContractClient};

    fn setup() -> (Env, SLACalculatorContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        (env, client, admin, operator)
    }

    #[test]
    fn test_audit_state_available_after_init() {
        let (_env, client, admin, operator) = setup();
        let state = client.get_full_audit_state();
        assert_eq!(state.admin, admin);
        assert_eq!(state.operator, operator);
        assert!(!state.paused);
        assert!(state.pause_info.is_empty());
        assert_eq!(state.history_len, 0);
    }

    #[test]
    fn test_audit_state_matches_individual_getters() {
        let (_env, client, _admin, _operator) = setup();
        let state = client.get_full_audit_state();
        assert_eq!(state.admin, client.get_admin());
        assert_eq!(state.operator, client.get_operator());
        assert_eq!(state.paused, client.is_paused());
        assert_eq!(state.config_frozen, client.is_config_frozen());
        assert_eq!(state.pause_info.first(), client.get_pause_info());
        assert_eq!(state.config_snapshot, client.get_config_snapshot());
        assert_eq!(state.stats, client.get_stats());
        assert_eq!(state.result_schema, client.get_result_schema());
    }

    #[test]
    fn test_audit_state_reflects_stats_and_history_len() {
        let (_env, client, _admin, operator) = setup();
        client.calculate_sla(&operator, &symbol_short!("OUT1"), &symbol_short!("high"), &10);
        client.calculate_sla(&operator, &symbol_short!("OUT2"), &symbol_short!("high"), &60);

        let state = client.get_full_audit_state();
        assert_eq!(state.history_len, 2);
        assert_eq!(state.stats.total_calculations, 2);
    }

    // #577 – the freeze posture must be visible to the audit endpoint and stay
    // consistent with the `is_config_frozen` getter across a freeze/thaw cycle.
    #[test]
    fn test_audit_state_reports_freeze_posture() {
        let (_env, client, admin, _operator) = setup();

        let state = client.get_full_audit_state();
        assert!(!state.config_frozen, "thawed by default after initialize");
        assert_eq!(state.config_frozen, client.is_config_frozen());

        client.freeze_config(&admin);
        let state = client.get_full_audit_state();
        assert!(
            state.config_frozen,
            "freeze must be visible to the audit endpoint"
        );
        assert_eq!(state.config_frozen, client.is_config_frozen());

        client.unfreeze_config(&admin);
        let state = client.get_full_audit_state();
        assert!(!state.config_frozen, "unfreeze must clear the freeze posture");
        assert_eq!(state.config_frozen, client.is_config_frozen());
    }

    // #493 – the pause_info Vec-stands-in-for-Option invariant: empty when
    // unpaused, exactly one element when paused, and always consistent with
    // the Option-typed getter surface.
    #[test]
    fn test_audit_state_pause_info_invariant_across_pause_cycle() {
        let (env, client, admin, _operator) = setup();

        // Unpaused: empty, matching get_pause_info() == None.
        let state = client.get_full_audit_state();
        assert!(!state.paused);
        assert!(state.pause_info.is_empty());
        assert!(state.pause_info.len() <= 1);
        assert_eq!(state.pause_info.first(), client.get_pause_info());

        // Paused: exactly one element, matching get_pause_info() == Some(_).
        client.pause(&admin, &soroban_sdk::String::from_str(&env, "maintenance"));
        let state = client.get_full_audit_state();
        assert!(state.paused);
        assert_eq!(state.pause_info.len(), 1);
        assert!(state.pause_info.len() <= 1);
        assert_eq!(state.pause_info.first(), client.get_pause_info());

        // Unpaused again: back to empty.
        client.unpause(&admin);
        let state = client.get_full_audit_state();
        assert!(!state.paused);
        assert!(state.pause_info.is_empty());
        assert_eq!(state.pause_info.first(), client.get_pause_info());
    }
}
