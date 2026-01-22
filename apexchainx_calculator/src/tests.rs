#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _};
use soroban_sdk::{Env};

#[test]
fn test_initialize_and_get_admin() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &contract_id);

    let admin = soroban_sdk::Address::generate(&env);

    client.initialize(&admin);

    let stored_admin = client.get_admin();
    assert_eq!(stored_admin, admin);
}

#[test]
#[should_panic]
fn test_double_initialize_panics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &contract_id);

    let admin = soroban_sdk::Address::generate(&env);

    client.initialize(&admin);
    client.initialize(&admin); // should panic
}

#[test]
fn test_default_config_initialization() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &contract_id);

    let admin = soroban_sdk::Address::generate(&env);
    client.initialize(&admin);

    // Test Critical severity config
    let critical_config = client.get_config(Severity::Critical);
    assert_eq!(critical_config.threshold_minutes, 15);
    assert_eq!(critical_config.penalty_per_minute, 10_0000000);
    assert_eq!(critical_config.reward_base, 500_0000000);

    // Test High severity config
    let high_config = client.get_config(Severity::High);
    assert_eq!(high_config.threshold_minutes, 30);
    assert_eq!(high_config.penalty_per_minute, 5_0000000);
    assert_eq!(high_config.reward_base, 200_0000000);

    // Test Medium severity config
    let medium_config = client.get_config(Severity::Medium);
    assert_eq!(medium_config.threshold_minutes, 60);
    assert_eq!(medium_config.penalty_per_minute, 2_0000000);
    assert_eq!(medium_config.reward_base, 100_0000000);

    // Test Low severity config
    let low_config = client.get_config(Severity::Low);
    assert_eq!(low_config.threshold_minutes, 120);
    assert_eq!(low_config.penalty_per_minute, 1_0000000);
    assert_eq!(low_config.reward_base, 50_0000000);
}

#[test]
fn test_admin_can_update_config() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &contract_id);

    let admin = soroban_sdk::Address::generate(&env);
    client.initialize(&admin);

    // Update critical config as admin
    client.update_config(
        Severity::Critical,
        20,           // new threshold: 20 minutes
        15_0000000,   // new penalty: $15 per minute
        750_0000000,  // new reward: $750 base
    );

    // Verify the update
    let updated_config = client.get_config(Severity::Critical);
    assert_eq!(updated_config.threshold_minutes, 20);
    assert_eq!(updated_config.penalty_per_minute, 15_0000000);
    assert_eq!(updated_config.reward_base, 750_0000000);

    // Verify other configs remain unchanged
    let high_config = client.get_config(Severity::High);
    assert_eq!(high_config.threshold_minutes, 30);
    assert_eq!(high_config.penalty_per_minute, 5_0000000);
}

#[test]
#[should_panic]
fn test_non_admin_cannot_update_config() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &contract_id);

    let admin = soroban_sdk::Address::generate(&env);
    client.initialize(&admin);

    // Create a different address that's not the admin
    let non_admin = soroban_sdk::Address::generate(&env);

    // Mock auth for non-admin trying to update config
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &non_admin,
        nonce: 0,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_config",
            args: (
                Severity::Critical,
                25_u32,
                20_0000000_i128,
                1000_0000000_i128,
            ).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    // This should panic because non-admin is trying to update
    client.update_config(
        Severity::Critical,
        25,
        20_0000000,
        1000_0000000,
    );
}