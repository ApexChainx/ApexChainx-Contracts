use soroban_sdk::{contracttype, Address};

use crate::{PauseInfo, SLAConfigSnapshot, SLAResultSchema, SLAStats};

/// Combined audit-state envelope for one-shot backend bootstrap reads (#107).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditState {
    pub admin: Address,
    pub operator: Address,
    pub pending_admin: Option<Address>,
    pub pending_operator: Option<Address>,
    pub paused: bool,
    pub pause_info: Option<PauseInfo>,
    pub config_snapshot: SLAConfigSnapshot,
    pub stats: SLAStats,
    pub history_len: u32,
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
        assert!(state.pause_info.is_none());
        assert_eq!(state.history_len, 0);
    }

    #[test]
    fn test_audit_state_matches_individual_getters() {
        let (_env, client, _admin, _operator) = setup();
        let state = client.get_full_audit_state();
        assert_eq!(state.admin, client.get_admin());
        assert_eq!(state.operator, client.get_operator());
        assert_eq!(state.paused, client.is_paused());
        assert_eq!(state.pause_info, client.get_pause_info());
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
}