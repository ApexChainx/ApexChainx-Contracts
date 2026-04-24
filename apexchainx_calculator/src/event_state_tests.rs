#[cfg(test)]
mod event_state_tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, testutils::Events, Address, Env};
    use crate::{SLACalculatorContract, SLACalculatorContractClient, SLAConfig};

    fn setup(env: &Env) -> (Address, Address, SLACalculatorContractClient) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        (admin, operator, client)
    }

    #[test]
    fn test_calculate_sla_emits_event_matching_result() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);
        let result = client.calculate_sla(
            &operator,
            &symbol_short!("OUT1"),
            &symbol_short!("high"),
            &10,
        );
        let events = env.events().all();
        assert!(!events.is_empty());
        // Stats state should reflect the calculation
        let stats = client.get_stats();
        assert_eq!(stats.total_calculations, 1);
        if result.status == symbol_short!("viol") {
            assert_eq!(stats.total_violations, 1);
        }
    }

    #[test]
    fn test_set_config_emits_event() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let before = env.events().all().len();
        client.set_config(
            &admin,
            &symbol_short!("high"),
            &SLAConfig { threshold_minutes: 45, penalty_per_minute: 60, reward_base: 800 },
        );
        let after = env.events().all().len();
        assert!(after > before);
    }

    #[test]
    fn test_pause_state_consistent_with_event() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        client.pause(&admin);
        let events = env.events().all();
        assert!(!events.is_empty());
        client.unpause(&admin);
        let events_after = env.events().all();
        assert!(events_after.len() > events.len());
    }
}
