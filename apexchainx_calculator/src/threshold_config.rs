//! SLA threshold boundary condition tests.
//!
//! This module tests edge cases around threshold configuration and SLA
//! calculation results. It verifies that extreme threshold values (zero,
//! near-zero) produce the correct contract behaviour: zero-threshold
//! configurations are rejected at validation time, and the minimum valid
//! threshold (1 minute) creates a razor-thin boundary for SLA outcomes.
//!
//! # Test Scenarios
//!
//! - `test_zero_threshold_rejected_by_validation`: `set_config` with
//!   `threshold_minutes = 0` must be rejected with `InvalidThreshold`
//!   (error code 8). This is the primary guard against a zero-threshold
//!   ever reaching storage — a zero value would make the `performance_ratio`
//!   calculation in `compute_result` divide by zero.
//!
//! - `test_zero_threshold_cannot_enter_storage`: Confirms the stored
//!   configuration is unchanged after a rejected zero-threshold call,
//!   asserting the no-partial-state-change guarantee.
//!
//! - `test_near_zero_threshold_one_minute`: A 1-minute threshold creates a
//!   razor-thin boundary where MTTR of 1 minute meets the SLA but MTTR of
//!   2 minutes violates it.

#[cfg(test)]
mod threshold_tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

    use crate::{SLACalculatorContract, SLACalculatorContractClient, SLAError};

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

    /// `set_config` with `threshold_minutes = 0` must be rejected with
    /// `InvalidThreshold` (error code 8).
    ///
    /// A zero threshold is unsafe: `compute_result` divides by
    /// `threshold_minutes` when computing the performance ratio for reward
    /// tier selection. The `validate_config` guard ensures this value can
    /// never reach on-chain storage, so this test pins that contract.
    ///
    /// Replaces the former `test_zero_threshold_always_violated` which
    /// incorrectly assumed `set_config` would succeed with threshold = 0.
    #[test]
    fn test_zero_threshold_rejected_by_validation() {
        let env = Env::default();
        let (admin, _operator, client) = setup(&env);

        let result = client.try_set_config(
            &admin,
            &symbol_short!("low"),
            &0,
            &10,
            &600,
        );

        assert_eq!(
            result,
            Err(Ok(SLAError::InvalidThreshold)),
            "set_config with threshold_minutes=0 must return InvalidThreshold (code 8)"
        );
    }

    /// After a rejected zero-threshold call the stored config must be
    /// unchanged, confirming the no-partial-state-change guarantee.
    #[test]
    fn test_zero_threshold_cannot_enter_storage() {
        let env = Env::default();
        let (admin, _operator, client) = setup(&env);

        // Record the default low config before the attempted write.
        let before = client.get_config(&symbol_short!("low"));

        // Attempt the invalid write.
        let _ = client.try_set_config(
            &admin,
            &symbol_short!("low"),
            &0,
            &10,
            &600,
        );

        // The stored config must be identical to what it was before.
        let after = client.get_config(&symbol_short!("low"));
        assert_eq!(
            before, after,
            "stored config must be unchanged after a rejected zero-threshold set_config"
        );
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
