//! SLA threshold boundary condition tests.
//!
//! This module tests edge cases around threshold configuration and SLA
//! calculation results. It verifies that extreme threshold values (zero,
//! near-zero) produce correct SLA outcomes.
//!
//! # Test Scenarios
//!
//! - `test_near_zero_threshold_one_minute`: A 1-minute threshold creates a
//!   razor-thin boundary where MTTR of 1 minute meets the SLA but MTTR of
//!   2 minutes violates it.

#[cfg(test)]
mod threshold_tests {
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
    #[should_panic]
    fn test_stranger_cannot_set_config() {
        let env = Env::default();
        let (_admin, _operator, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.set_config(
            &stranger,
            &symbol_short!("low"),
            &1,
            &5,
            &50,
        );
    }

    #[test]
    #[should_panic]
    fn test_admin_cannot_calculate_sla() {
        let env = Env::default();
        let (admin, _operator, client) = setup(&env);
        // admin is not the operator
        client.calculate_sla(
            &admin,
            &symbol_short!("THR_ADMIN"),
            &symbol_short!("low"),
            &1,
        );
    }

    #[test]
    fn test_zero_threshold_always_violated() {
        let env = Env::default();
        let (admin, operator, client) = setup(&env);
        client.set_config(
            &admin,
            &symbol_short!("low"),
            &0,
            &10,
            &100,
        );
        let result = client.calculate_sla(
            &operator,
            &symbol_short!("OUT1"),
            &symbol_short!("low"),
            &1,
        );
        assert_eq!(result.status, symbol_short!("viol"));
    }

    #[test]
    fn test_near_zero_threshold_one_minute() {
        let env = Env::default();
        let (admin, operator, client) = setup(&env);
        client.set_config(
            &admin,
            &symbol_short!("low"),
            &1,
            &5,
            &50,
        );
        let met = client.calculate_sla(
            &operator,
            &symbol_short!("OUT2"),
            &symbol_short!("low"),
            &1,
        );
        assert_eq!(met.status, symbol_short!("met"));

        let viol = client.calculate_sla(
            &operator,
            &symbol_short!("OUT3"),
            &symbol_short!("low"),
            &2,
        );
        assert_eq!(viol.status, symbol_short!("viol"));
    }
}
