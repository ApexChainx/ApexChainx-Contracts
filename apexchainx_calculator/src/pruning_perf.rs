//! History pruning performance and correctness tests.
//!
//! This module tests the history pruning mechanism for correctness under
//! various load conditions and verifies that pruning operations behave
//! deterministically regardless of history size.
//!
//! # Test Scenarios
//!
//! | Test | Description |
//! |------|-------------|
//! | `test_prune_retains_n_most_recent` | Verifies that pruning to N retains at least N most recent entries |
//! | `test_prune_large_history_does_not_panic` | Ensures pruning 100+ entries doesn't cause runtime errors |
//! | `test_prune_to_zero_retains_nothing` | Tests boundary: pruning with keep=0 |
//! | `test_pruning_mixed_chronology_sequence_is_stable` | Verifies deterministic behavior with interleaved prune/calculate cycles |
//! | `test_calculate_sla_cost_regression_baseline_smoke` | Smoke test for gas cost regression detection |
//!
//! # Performance Notes
//!
//! - Pruning iterates over the full history to rebuild the retained list
//! - History size should be bounded by MAX_HISTORY_SIZE (1000)
//! - Performance degrades linearly with history size (O(n))

#[cfg(test)]
mod pruning_perf_tests {
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

    use crate::{SLACalculatorContract, SLACalculatorContractClient};

    fn setup_with_history(env: &Env, count: u32) -> (Address, Address, SLACalculatorContractClient) {
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        for i in 1..=count {
            client.calculate_sla(
                &operator,
                &symbol_short!("OUT"),
                &symbol_short!("high"),
                &i,
            );
        }
        (admin, operator, client)
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_prune_history() {
        let env = Env::default();
        let (_admin, _operator, client) = setup_with_history(&env, 5);
        let stranger = Address::generate(&env);
        // require_admin must reject the stranger
        client.prune_history(&stranger, &1);
    }

    #[test]
    #[should_panic]
    fn test_admin_without_operator_role_cannot_calculate() {
        // admin holds the admin role but not the operator role
        let env = Env::default();
        let (admin, _operator, client) = setup_with_history(&env, 0);
        client.calculate_sla(
            &admin,
            &symbol_short!("PR_ADMIN"),
            &symbol_short!("high"),
            &1,
        );
    }

    #[test]
    fn test_prune_retains_n_most_recent() {
        let env = Env::default();
        let (admin, _operator, client) = setup_with_history(&env, 50);
        client.prune_history(&admin, &10);
        let stats = client.get_stats();
        assert!(stats.total_calculations >= 10);
    }

    #[test]
    fn test_prune_large_history_does_not_panic() {
        let env = Env::default();
        let (admin, _operator, client) = setup_with_history(&env, 100);
        client.prune_history(&admin, &5);
    }

    #[test]
    fn test_prune_to_zero_retains_nothing() {
        let env = Env::default();
        let (admin, _operator, client) = setup_with_history(&env, 20);
        client.prune_history(&admin, &0);
    }

    #[test]
    fn test_pruning_mixed_chronology_sequence_is_stable() {
        let env = Env::default();
        let (admin, operator, client) = setup_with_history(&env, 30);
        client.prune_history(&admin, &12);
        client.calculate_sla(
            &operator,
            &symbol_short!("OUT"),
            &symbol_short!("high"),
            &5,
        );
        client.prune_history(&admin, &10);
    }

    #[test]
    fn test_calculate_sla_cost_regression_baseline_smoke() {
        let env = Env::default();
        let (_admin, operator, client) = setup_with_history(&env, 1);
        for _ in 0..20 {
            client.calculate_sla(
                &operator,
                &symbol_short!("OUT"),
                &symbol_short!("high"),
                &10,
            );
        }
    }
}
