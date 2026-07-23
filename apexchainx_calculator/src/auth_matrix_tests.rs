#[cfg(test)]
mod auth_matrix_tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};
    use crate::{SLACalculatorContract, SLACalculatorContractClient};

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
        let (_, operator, client) = setup(&env);
        // operator role holds; auth resolved by Soroban test framework.
        client.calculate_sla(&operator, &symbol_short!("OUT1"), &symbol_short!("high"), &10);
    }

    #[test]
    fn test_only_admin_can_set_config() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        client.set_config(
            &admin,
            &symbol_short!("high"),
            &30,
            &50,
            &500,
        );
    }

    #[test]
    fn test_only_admin_can_pause() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        client.pause(&admin);
        client.unpause(&admin);
    }

    #[test]
    fn test_only_admin_can_set_operator() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);
        client.set_operator(&admin, &new_op);
    }

    #[test]
    fn test_repeated_calls_by_same_operator_succeed() {
        let env = Env::default();
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
        client.initialize(&admin, &operator);
        // Tighten auths for the negative path: clear any implicit auth the
        // test framework grants generated addresses so that only an explicit
        // MockAuth would satisfy the call. The stranger has none, so the call
        // must surface an error – exactly the role-isolation we want to assert.
        env.mock_auths(&[]);
        let result = client.try_calculate_sla(
            &stranger,
            &symbol_short!("OUT"),
            &symbol_short!("high"),
            &5,
        );
        assert!(result.is_err());
    }

    // ── Auth-gated negative coverage ─────────────────────────────────────
    //
    // The single `test_unauthorized_caller_cannot_calculate` above covered
    // one stranger scenario. These tests expand coverage to the full auth
    // matrix so every admin-gated and operator-gated function is verified
    // against non-role callers. They use the same `#[should_panic]` idiom
    // as the rest of the suite – the contract returns `Unauthorized`, which
    // the generated client surfaces as a panic.

    #[test]
    #[should_panic]
    fn test_stranger_cannot_calculate_sla() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.calculate_sla(
            &stranger,
            &symbol_short!("U_CALC"),
            &symbol_short!("high"),
            &10,
        );
    }

    #[test]
    #[should_panic]
    fn test_admin_cannot_calculate_sla() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        // admin holds the admin role, NOT the operator role.
        client.calculate_sla(
            &admin,
            &symbol_short!("A_CALC"),
            &symbol_short!("high"),
            &10,
        );
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_set_config() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.set_config(
            &stranger,
            &symbol_short!("high"),
            &30,
            &50,
            &500,
        );
    }

    #[test]
    #[should_panic]
    fn test_operator_cannot_set_config() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);
        client.set_config(
            &operator,
            &symbol_short!("high"),
            &30,
            &50,
            &500,
        );
    }
    #[test]
    #[should_panic]
    fn test_stranger_cannot_pause() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.pause(&stranger);
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_set_operator() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        let new_op = Address::generate(&env);
        client.set_operator(&stranger, &new_op);
    }

    /// Verifies that a caller who holds the admin role but has NOT provided
    /// Soroban auth receives a clean `Unauthorized` error (via try_*) rather
    /// than a panic. This ensures the `require_auth()` gate is exercised and
    /// the equality check is not the sole authentication mechanism.
    #[test]
    fn test_no_role_equivalence_bypass_does_not_panic() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        // Clear all auths so the admin address is NOT authorised for Soroban
        // auth, even though it holds the admin role in storage.
        env.mock_auths(&[]);

        // admin role is set, but auth is not provided → should return error, not panic
        let result = client.try_set_config(
            &admin,
            &symbol_short!("high"),
            &30,
            &50,
            &500,
        );
        assert!(result.is_err());
    }
}
