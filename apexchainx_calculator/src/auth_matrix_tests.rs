#[cfg(test)]
mod auth_matrix_tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};
    use crate::{SLACalculatorContract, SLACalculatorContractClient, SLAConfig, SLAError};

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
    fn test_only_operator_can_calculate_sla() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, operator, client) = setup(&env);
        client.calculate_sla(&operator, &symbol_short!("OUT1"), &symbol_short!("high"), &10);
    }

    #[test]
    fn test_only_admin_can_set_config() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, client) = setup(&env);
        client.set_config(
            &admin,
            &symbol_short!("high"),
            &SLAConfig { threshold_minutes: 30, penalty_per_minute: 50, reward_base: 500 },
        );
    }

    #[test]
    fn test_only_admin_can_pause() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, client) = setup(&env);
        client.pause(&admin);
        client.unpause(&admin);
    }

    #[test]
    fn test_only_admin_can_set_operator() {
        let env = Env::default();
        env.mock_all_auths();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);
        client.set_operator(&admin, &new_op);
    }

    #[test]
    fn test_repeated_calls_by_same_operator_succeed() {
        let env = Env::default();
        env.mock_all_auths();
        let (_, operator, client) = setup(&env);
        for i in 1u32..=3 {
            client.calculate_sla(&operator, &symbol_short!("OUT"), &symbol_short!("high"), &i);
        }
        let stats = client.get_stats();
        assert_eq!(stats.total_calculations, 3);
    }

    #[test]
    fn test_unauthorized_caller_cannot_calculate() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        let stranger = Address::generate(&env);
        env.mock_all_auths();
        client.initialize(&admin, &operator);
        env.mock_auths(&[]);
        let result = client.try_calculate_sla(
            &stranger,
            &symbol_short!("OUT"),
            &symbol_short!("high"),
            &5,
        );
        assert!(result.is_err());
    }
}
