#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Env, Symbol};

fn setup_contract(env: &Env) -> SLACalculatorContractClient {
    let contract_id = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(env, &contract_id);

    let admin = soroban_sdk::Address::generate(env);
    client.initialize(&admin);

    // Setup configs like backend
    client.set_config(&admin, &symbol_short!("critical"), &15, &100, &750);
    client.set_config(&admin, &symbol_short!("high"), &30, &50, &750);
    client.set_config(&admin, &symbol_short!("medium"), &60, &25, &750);
    client.set_config(&admin, &symbol_short!("low"), &120, &10, &600);

    client
}

#[test]
fn test_critical_violation() {
    let env = Env::default();
    let client = setup_contract(&env);

    let result = client.calculate_sla(
        &symbol_short!("OUT1"),
        &symbol_short!("critical"),
        &25,
    );

    assert_eq!(result.status, symbol_short!("violated"));
    assert_eq!(result.amount, -1000); // (25 - 15) * 100
}

#[test]
fn test_high_exceptional_reward() {
    let env = Env::default();
    let client = setup_contract(&env);

    let result = client.calculate_sla(
        &symbol_short!("OUT2"),
        &symbol_short!("high"),
        &10,
    );

    assert_eq!(result.status, symbol_short!("met"));
    assert_eq!(result.amount, 1500); // 750 * 2
    assert_eq!(result.rating, symbol_short!("exceptional"));
}

#[test]
fn test_medium_good_reward() {
    let env = Env::default();
    let client = setup_contract(&env);

    let result = client.calculate_sla(
        &symbol_short!("OUT3"),
        &symbol_short!("medium"),
        &50,
    );

    assert_eq!(result.status, symbol_short!("met"));
    assert_eq!(result.rating, symbol_short!("good"));
}
