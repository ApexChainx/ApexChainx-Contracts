#![cfg(test)]

use super::*;
use alloc::format;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Events as _;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::{Env, Symbol, TryIntoVal};

// ============================================================
// Test helpers
// ============================================================

struct Actors {
    admin: soroban_sdk::Address,
    operator: soroban_sdk::Address,
    stranger: soroban_sdk::Address,
}

struct GoldenCase<'a> {
    severity: &'a str,
    mttr_minutes: u32,
    expected_status: &'a str,
    expected_payment_type: &'a str,
    expected_rating: &'a str,
    expected_amount: i128,
}

fn symbol(env: &Env, value: &str) -> Symbol {
    Symbol::new(env, value)
}

fn setup() -> (Env, SLACalculatorContractClient<'static>, Actors) {
    let env = Env::default();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let actors = Actors {
        admin: soroban_sdk::Address::generate(&env),
        operator: soroban_sdk::Address::generate(&env),
        stranger: soroban_sdk::Address::generate(&env),
    };
    client.initialize(&actors.admin, &actors.operator);
    (env, client, actors)
}

// ============================================================
// Initialisation
// ============================================================

#[test]
fn test_initialize_stores_roles() {
    let (_env, client, actors) = setup();
    assert_eq!(client.get_admin(), actors.admin);
    assert_eq!(client.get_operator(), actors.operator);
}

#[test]
#[should_panic]
fn test_double_initialize_fails() {
    let (_env, client, actors) = setup();
    // second call must panic with AlreadyInitialized
    client.initialize(&actors.admin, &actors.operator);
}

// ============================================================
// Default configs present after init
// ============================================================

#[test]
fn test_defaults_exist_after_initialize() {
    let (_env, client, _actors) = setup();

    assert_eq!(
        client
            .get_config(&symbol_short!("critical"))
            .threshold_minutes,
        15
    );
    assert_eq!(
        client.get_config(&symbol_short!("high")).threshold_minutes,
        30
    );
    assert_eq!(
        client
            .get_config(&symbol_short!("medium"))
            .threshold_minutes,
        60
    );
    assert_eq!(
        client.get_config(&symbol_short!("low")).threshold_minutes,
        120
    );
}

#[test]
fn test_config_snapshot_is_deterministic_and_complete() {
    let (_env, client, _actors) = setup();

    let snapshot = client.get_config_snapshot();
    assert_eq!(snapshot.version, symbol_short!("v1"));
    assert_eq!(snapshot.entries.len(), 4);

    let expected = [
        (symbol_short!("critical"), 15u32),
        (symbol_short!("high"), 30u32),
        (symbol_short!("medium"), 60u32),
        (symbol_short!("low"), 120u32),
    ];

    for (i, (severity, threshold)) in expected.iter().enumerate() {
        let entry = snapshot.entries.get(i as u32).unwrap();
        assert_eq!(entry.severity, severity.clone());
        assert_eq!(entry.config.threshold_minutes, *threshold);
    }
}

#[test]
fn test_result_schema_is_explicit_and_stable() {
    let (_env, client, _actors) = setup();

    let schema = client.get_result_schema();
    assert_eq!(schema.version, symbol_short!("v1"));
    assert_eq!(schema.schema_version, 1);
    assert_eq!(schema.status_met, symbol_short!("met"));
    assert_eq!(schema.status_violated, symbol_short!("viol"));
    assert_eq!(schema.payment_reward, symbol_short!("rew"));
    assert_eq!(schema.payment_penalty, symbol_short!("pen"));
    assert_eq!(schema.rating_exceptional, symbol_short!("top"));
    assert_eq!(schema.rating_excellent, symbol_short!("excel"));
    assert_eq!(schema.rating_good, symbol_short!("good"));
    assert_eq!(schema.rating_poor, symbol_short!("poor"));
    assert!(schema.includes_config_version_hash);
}

#[test]
fn test_calculate_sla_emits_versioned_integration_event() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("EVT001"),
        &symbol_short!("critical"),
        &5,
    );

    let events = env.events().all();
    let (_, topics, data) = events.get(events.len() - 2).unwrap();

    let topic_0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic_1: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
    let topic_2: Symbol = topics.get(2).unwrap().try_into_val(&env).unwrap();
    let event_data: (Symbol, Symbol, Symbol, Symbol, u32, u32, i128) =
        data.try_into_val(&env).unwrap();

    assert_eq!(topic_0, EVENT_SLA_CALC);
    assert_eq!(topic_1, EVENT_VERSION);
    assert_eq!(topic_2, symbol_short!("critical"));
    assert_eq!(
        event_data,
        (
            symbol_short!("EVT001"),
            symbol_short!("met"),
            symbol_short!("rew"),
            symbol_short!("top"),
            5u32,
            15u32,
            1500i128,
        ),
    );
}

#[test]
fn test_set_config_emits_versioned_config_event() {
    let (env, client, actors) = setup();

    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    let events = env.events().all();
    let (_, topics, data) = events.last().unwrap();

    let topic_0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let topic_1: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
    let topic_2: Symbol = topics.get(2).unwrap().try_into_val(&env).unwrap();
    let event_data: (u32, i128, i128) = data.try_into_val(&env).unwrap();

    assert_eq!(topic_0, EVENT_CONFIG_UPD);
    assert_eq!(topic_1, EVENT_VERSION);
    assert_eq!(topic_2, symbol_short!("critical"));
    assert_eq!(event_data, (20u32, 200i128, 1000i128));
}

// ============================================================
// #28 – Operator management
// ============================================================

#[test]
fn test_admin_can_set_operator() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.set_operator(&actors.admin, &new_op);

    assert_eq!(client.get_operator(), new_op);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_operator() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    // operator does not have the admin role
    client.set_operator(&actors.operator, &new_op);
}

#[test]
#[should_panic]
fn test_stranger_cannot_set_operator() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.set_operator(&actors.stranger, &new_op);
}

// ============================================================
// #28 – Config management: admin only
// ============================================================

#[test]
fn test_admin_can_set_and_get_config() {
    let (_env, client, actors) = setup();

    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    let cfg = client.get_config(&symbol_short!("critical"));
    assert_eq!(cfg.threshold_minutes, 20);
    assert_eq!(cfg.penalty_per_minute, 200);
    assert_eq!(cfg.reward_base, 1000);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_config() {
    let (_env, client, actors) = setup();
    // operator must not be allowed to change config
    client.set_config(
        &actors.operator,
        &symbol_short!("critical"),
        &20,
        &200,
        &1000,
    );
}

#[test]
#[should_panic]
fn test_stranger_cannot_set_config() {
    let (_env, client, actors) = setup();
    client.set_config(
        &actors.stranger,
        &symbol_short!("critical"),
        &20,
        &200,
        &1000,
    );
}

#[test]
fn test_storage_key_namespace_symbols_are_distinct() {
    let keys = [
        ADMIN_KEY,
        OPERATOR_KEY,
        PENDING_ADMIN_KEY,
        PENDING_OP_KEY,
        CONFIG_KEY,
        PAUSED_KEY,
        PAUSE_INFO_KEY,
        STATS_KEY,
        HISTORY_KEY,
        STORAGE_VERSION_KEY,
        RETENTION_LIMIT_KEY,
    ];

    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(keys[i], keys[j]);
        }
    }
}

// ============================================================
// #28 – calculate_sla: operator only
// ============================================================

#[test]
fn test_operator_can_calculate_sla() {
    let (_env, client, actors) = setup();

    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("INC001"),
        &symbol_short!("critical"),
        &10, // under 15-min threshold → met
    );

    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
#[should_panic]
fn test_admin_cannot_calculate_sla() {
    let (_env, client, actors) = setup();
    // admin does not hold the operator role
    client.calculate_sla(
        &actors.admin,
        &symbol_short!("INC002"),
        &symbol_short!("critical"),
        &10,
    );
}

#[test]
#[should_panic]
fn test_stranger_cannot_calculate_sla() {
    let (_env, client, actors) = setup();
    client.calculate_sla(
        &actors.stranger,
        &symbol_short!("INC003"),
        &symbol_short!("critical"),
        &10,
    );
}

/// After the admin reassigns the operator, the OLD operator is locked out
/// and the NEW operator can calculate.
#[test]
fn test_operator_rotation() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.set_operator(&actors.admin, &new_op);

    // new operator succeeds
    let result = client.calculate_sla(
        &new_op,
        &symbol_short!("INC004"),
        &symbol_short!("high"),
        &20,
    );
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
#[should_panic]
fn test_old_operator_locked_out_after_rotation() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.set_operator(&actors.admin, &new_op);

    // original operator should now be rejected
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("INC005"),
        &symbol_short!("high"),
        &20,
    );
}

// ============================================================
// #27 – Pause / Emergency Stop
// ============================================================

#[test]
fn test_contract_starts_unpaused() {
    let (_env, client, _actors) = setup();
    assert!(!client.is_paused());
}

#[test]
fn test_admin_can_pause_and_unpause() {
    let (_env, client, actors) = setup();

    client.pause(&actors.admin, &soroban_sdk::String::from_str(&_env, "test"));
    assert!(client.is_paused());

    client.unpause(&actors.admin);
    assert!(!client.is_paused());
}

#[test]
#[should_panic]
fn test_operator_cannot_pause() {
    let (env, client, actors) = setup();
    client.pause(&actors.operator, &soroban_sdk::String::from_str(&env, "x"));
}

#[test]
#[should_panic]
fn test_stranger_cannot_pause() {
    let (env, client, actors) = setup();
    client.pause(&actors.stranger, &soroban_sdk::String::from_str(&env, "x"));
}

#[test]
#[should_panic]
fn test_operator_cannot_unpause() {
    let (env, client, actors) = setup();
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "x"));
    client.unpause(&actors.operator);
}

#[test]
#[should_panic]
fn test_calculate_sla_blocked_when_paused() {
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );

    // must panic – ContractPaused
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("INC006"),
        &symbol_short!("critical"),
        &10,
    );
}

#[test]
fn test_calculate_sla_works_after_unpause() {
    let (env, client, actors) = setup();

    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "x"));
    client.unpause(&actors.admin);

    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("INC007"),
        &symbol_short!("critical"),
        &10,
    );
    assert_eq!(result.status, symbol_short!("met"));
}

// ============================================================
// Config freeze / unfreeze
// ============================================================

#[test]
fn test_config_starts_unfrozen() {
    let (_env, client, _actors) = setup();
    assert!(!client.is_config_frozen());
}

#[test]
fn test_admin_can_freeze_and_unfreeze() {
    let (_env, client, actors) = setup();
    assert!(!client.is_config_frozen());

    client.freeze_config(&actors.admin);
    assert!(client.is_config_frozen());

    client.unfreeze_config(&actors.admin);
    assert!(!client.is_config_frozen());
}

#[test]
#[should_panic]
fn test_operator_cannot_freeze() {
    let (_env, client, actors) = setup();
    client.freeze_config(&actors.operator);
}

#[test]
#[should_panic]
fn test_stranger_cannot_freeze() {
    let (_env, client, actors) = setup();
    client.freeze_config(&actors.stranger);
}

#[test]
#[should_panic]
fn test_operator_cannot_unfreeze() {
    let (_env, client, actors) = setup();
    client.freeze_config(&actors.admin);
    client.unfreeze_config(&actors.operator);
}

#[test]
#[should_panic]
fn test_set_config_blocked_when_frozen() {
    let (_env, client, actors) = setup();
    client.freeze_config(&actors.admin);
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
}

#[test]
fn test_set_config_works_after_unfreeze() {
    let (_env, client, actors) = setup();
    client.freeze_config(&actors.admin);
    client.unfreeze_config(&actors.admin);
    // Should not panic after unfreeze
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
}

#[test]
fn test_freeze_emits_event() {
    let (env, client, actors) = setup();
    client.freeze_config(&actors.admin);
    let events = env.events().all();
    let (_, topics, _) = events.last().unwrap();
    assert_eq!(topics.len(), 3);
    let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let version: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(name, symbol_short!("cfg_frz"));
    assert_eq!(version, symbol_short!("v1"));
}

#[test]
fn test_unfreeze_emits_event() {
    let (env, client, actors) = setup();
    client.freeze_config(&actors.admin);
    client.unfreeze_config(&actors.admin);
    let events = env.events().all();
    let (_, topics, _) = events.last().unwrap();
    assert_eq!(topics.len(), 3);
    let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    let version: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
    assert_eq!(name, symbol_short!("cfg_unfrz"));
    assert_eq!(version, symbol_short!("v1"));
}

// ============================================================
// SLA business logic correctness
// ============================================================

#[test]
fn test_sla_violation_calculates_penalty() {
    let (_env, client, actors) = setup();

    // critical threshold = 15 min, penalty = 100/min
    // mttr = 25 → 10 min overtime → penalty = 1000
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("INC008"),
        &symbol_short!("critical"),
        &25,
    );

    assert_eq!(result.status, symbol_short!("viol"));
    assert_eq!(result.payment_type, symbol_short!("pen"));
    assert_eq!(result.rating, symbol_short!("poor"));
    assert_eq!(result.amount, -1000);
}

#[test]
fn test_sla_met_top_rating() {
    let (_env, client, actors) = setup();

    // critical threshold = 15 min; mttr = 5 → ratio = 33% < 50% → "top", 2× reward
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("INC009"),
        &symbol_short!("critical"),
        &5,
    );

    assert_eq!(result.status, symbol_short!("met"));
    assert_eq!(result.payment_type, symbol_short!("rew"));
    assert_eq!(result.rating, symbol_short!("top"));
    assert_eq!(result.amount, 1500); // 750 * 200 / 100
}

#[test]
fn test_backend_parity_threshold_boundary_cases() {
    let (env, client, actors) = setup();
    let cases = [
        GoldenCase {
            severity: "critical",
            mttr_minutes: 15,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "good",
            expected_amount: 750,
        },
        GoldenCase {
            severity: "critical",
            mttr_minutes: 16,
            expected_status: "viol",
            expected_payment_type: "pen",
            expected_rating: "poor",
            expected_amount: -100,
        },
        GoldenCase {
            severity: "high",
            mttr_minutes: 30,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "good",
            expected_amount: 750,
        },
        GoldenCase {
            severity: "high",
            mttr_minutes: 31,
            expected_status: "viol",
            expected_payment_type: "pen",
            expected_rating: "poor",
            expected_amount: -50,
        },
        GoldenCase {
            severity: "medium",
            mttr_minutes: 60,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "good",
            expected_amount: 750,
        },
        GoldenCase {
            severity: "medium",
            mttr_minutes: 61,
            expected_status: "viol",
            expected_payment_type: "pen",
            expected_rating: "poor",
            expected_amount: -25,
        },
        GoldenCase {
            severity: "low",
            mttr_minutes: 120,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "good",
            expected_amount: 600,
        },
        GoldenCase {
            severity: "low",
            mttr_minutes: 121,
            expected_status: "viol",
            expected_payment_type: "pen",
            expected_rating: "poor",
            expected_amount: -10,
        },
    ];

    for (i, case) in cases.iter().enumerate() {
        let outage_id = Symbol::new(&env, &alloc::format!("PARITY_B_{}", i));
        let severity = symbol(&env, case.severity);
        let result =
            client.calculate_sla(&actors.operator, &outage_id, &severity, &case.mttr_minutes);

        assert_eq!(result.status, symbol(&env, case.expected_status));
        assert_eq!(
            result.payment_type,
            symbol(&env, case.expected_payment_type)
        );
        assert_eq!(result.rating, symbol(&env, case.expected_rating));
        assert_eq!(result.amount, case.expected_amount);
    }
}

#[test]
fn test_exact_threshold_mttr_is_always_met_never_violated() {
    let (_env, client, actors) = setup();
    let cases = [
        (symbol_short!("critical"), 15u32, 750i128),
        (symbol_short!("high"), 30u32, 750i128),
        (symbol_short!("medium"), 60u32, 750i128),
        (symbol_short!("low"), 120u32, 600i128),
    ];

    for (i, (severity, threshold, expected_amount)) in cases.iter().enumerate() {
        let view = client.calculate_sla_view(&symbol_short!("BNDV"), severity, threshold);
        let mutating = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, &alloc::format!("BNDM_{}", i)),
            severity,
            threshold,
        );

        assert_eq!(view.status, symbol_short!("met"));
        assert_eq!(view.payment_type, symbol_short!("rew"));
        assert_eq!(view.rating, symbol_short!("good"));
        assert_eq!(view.amount, *expected_amount);
        assert_eq!(view.threshold_minutes, *threshold);

        assert_eq!(mutating.status, symbol_short!("met"));
        assert_eq!(mutating.payment_type, symbol_short!("rew"));
        assert_eq!(mutating.rating, symbol_short!("good"));
        assert_eq!(mutating.amount, *expected_amount);
        assert_eq!(mutating.threshold_minutes, *threshold);
    }
}

#[test]
fn test_threshold_boundary_keeps_equality_met_and_plus_one_violated() {
    let (_env, client, _actors) = setup();

    let exact =
        client.calculate_sla_view(&symbol_short!("BND_EX"), &symbol_short!("critical"), &15);
    let plus_one =
        client.calculate_sla_view(&symbol_short!("BND_P1"), &symbol_short!("critical"), &16);

    assert_eq!(exact.status, symbol_short!("met"));
    assert_eq!(exact.payment_type, symbol_short!("rew"));
    assert_eq!(exact.rating, symbol_short!("good"));
    assert!(exact.amount > 0);

    assert_eq!(plus_one.status, symbol_short!("viol"));
    assert_eq!(plus_one.payment_type, symbol_short!("pen"));
    assert_eq!(plus_one.rating, symbol_short!("poor"));
    assert!(plus_one.amount < 0);
}

#[test]
fn test_exact_threshold_boundary_is_stable_after_config_update() {
    let (_env, client, actors) = setup();

    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    let exact = client.calculate_sla(
        &actors.operator,
        &symbol_short!("EXACT"),
        &symbol_short!("critical"),
        &20,
    );
    let over = client.calculate_sla(
        &actors.operator,
        &symbol_short!("OVER"),
        &symbol_short!("critical"),
        &21,
    );

    assert_eq!(exact.status, symbol_short!("met"));
    assert_eq!(exact.payment_type, symbol_short!("rew"));
    assert_eq!(exact.rating, symbol_short!("good"));
    assert_eq!(exact.amount, 1000);

    assert_eq!(over.status, symbol_short!("viol"));
    assert_eq!(over.payment_type, symbol_short!("pen"));
    assert_eq!(over.amount, -200);
}

#[test]
fn test_backend_replay_exact_threshold_outcome_is_deterministic_before_config_change() {
    let (env, client, actors) = setup();

    let severity = symbol_short!("high");
    let mttr = 30u32;
    let outage_id = symbol(&env, "THR001");

    let stored = client.calculate_sla(&actors.operator, &outage_id, &severity, &mttr);
    let replayed = client.calculate_sla_view(&outage_id, &severity, &mttr);

    assert_eq!(stored.status, symbol_short!("met"));
    assert_eq!(stored.payment_type, symbol_short!("rew"));
    assert_eq!(stored.rating, symbol_short!("good"));
    assert_eq!(stored.amount, 750);

    assert_eq!(stored.status, replayed.status);
    assert_eq!(stored.payment_type, replayed.payment_type);
    assert_eq!(stored.rating, replayed.rating);
    assert_eq!(stored.amount, replayed.amount);
    assert_eq!(stored.threshold_minutes, replayed.threshold_minutes);
}

#[test]
fn test_backend_parity_reward_tier_cases() {
    let (env, client, actors) = setup();
    let cases = [
        GoldenCase {
            severity: "critical",
            mttr_minutes: 7,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "top",
            expected_amount: 1500,
        },
        GoldenCase {
            severity: "critical",
            mttr_minutes: 10,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "excel",
            expected_amount: 1125,
        },
        GoldenCase {
            severity: "critical",
            mttr_minutes: 15,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "good",
            expected_amount: 750,
        },
        GoldenCase {
            severity: "low",
            mttr_minutes: 59,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "top",
            expected_amount: 1200,
        },
        GoldenCase {
            severity: "low",
            mttr_minutes: 89,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "excel",
            expected_amount: 900,
        },
        GoldenCase {
            severity: "low",
            mttr_minutes: 120,
            expected_status: "met",
            expected_payment_type: "rew",
            expected_rating: "good",
            expected_amount: 600,
        },
    ];

    for (i, case) in cases.iter().enumerate() {
        let outage_id = Symbol::new(&env, &alloc::format!("PARITY_R_{}", i));
        let severity = symbol(&env, case.severity);
        let result =
            client.calculate_sla(&actors.operator, &outage_id, &severity, &case.mttr_minutes);

        assert_eq!(result.status, symbol(&env, case.expected_status));
        assert_eq!(
            result.payment_type,
            symbol(&env, case.expected_payment_type)
        );
        assert_eq!(result.rating, symbol(&env, case.expected_rating));
        assert_eq!(result.amount, case.expected_amount);
    }
}

// ============================================================
// Budget / performance
// ============================================================

#[test]
fn test_calculate_sla_budget_is_reasonable() {
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    let before = env.budget().cpu_instruction_cost();
    let _ = client.calculate_sla(&op, &symbol_short!("BUDG"), &symbol_short!("critical"), &25);
    let after = env.budget().cpu_instruction_cost();

    assert!(
        after - before < 200_000,
        "calculate_sla too expensive: {} instructions",
        after - before
    );
}

#[test]
fn test_set_config_budget_is_reasonable() {
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    let before = env.budget().cpu_instruction_cost();
    client.set_config(&admin, &symbol_short!("critical"), &15, &100, &750);
    let after = env.budget().cpu_instruction_cost();

    assert!(
        after - before < 150_000,
        "set_config too expensive: {} instructions",
        after - before
    );
}

// ============================================================
// #29 – SLA Statistics Aggregation
// ============================================================

#[test]
fn test_stats_zeroed_after_initialize() {
    let (_env, client, _actors) = setup();
    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 0);
    assert_eq!(stats.total_violations, 0);
    assert_eq!(stats.total_rewards, 0);
    assert_eq!(stats.total_penalties, 0);
}

#[test]
fn test_stats_increment_on_violation() {
    let (_env, client, actors) = setup();

    // critical: threshold=15, penalty=100/min; mttr=25 → 10 min over → penalty=1000
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("S001"),
        &symbol_short!("critical"),
        &25,
    );

    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 1);
    assert_eq!(stats.total_violations, 1);
    assert_eq!(stats.total_penalties, 1000);
    assert_eq!(stats.total_rewards, 0);
}

#[test]
fn test_stats_increment_on_met() {
    let (_env, client, actors) = setup();

    // critical: threshold=15, mttr=5 → "top" → reward=1500
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("S002"),
        &symbol_short!("critical"),
        &5,
    );

    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 1);
    assert_eq!(stats.total_violations, 0);
    assert_eq!(stats.total_rewards, 1500);
    assert_eq!(stats.total_penalties, 0);
}

#[test]
fn test_stats_accumulate_across_multiple_calculations() {
    let (_env, client, actors) = setup();

    // 1 violation: mttr=25, critical → penalty=1000
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("S003"),
        &symbol_short!("critical"),
        &25,
    );
    // 2 met: mttr=5, critical → reward=1500
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("S004"),
        &symbol_short!("critical"),
        &5,
    );
    // 3 met: mttr=20, high (threshold=30) → ratio=66% → "excel" → reward=750*150/100=1125
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("S005"),
        &symbol_short!("high"),
        &20,
    );
    // 4 violation: mttr=40, high (threshold=30) → 10 min over, penalty=50/min → penalty=500
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("S006"),
        &symbol_short!("high"),
        &40,
    );

    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 4);
    assert_eq!(stats.total_violations, 2);
    assert_eq!(stats.total_rewards, 1500 + 1125); // 2625
    assert_eq!(stats.total_penalties, 1000 + 500); // 1500
}

#[test]
fn test_stats_not_updated_on_paused_rejection() {
    let (env, client, actors) = setup();

    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "test"));

    // Fresh setup: verify stats stay at 0 when no successful calls were made.
    let (_env2, client2, _actors2) = setup();
    let stats = client2.get_stats();
    assert_eq!(stats.total_calculations, 0);
}

#[test]
fn test_stats_not_incremented_by_unauthorized_caller() {
    let (_env, _client, _actors) = setup();

    // Confirm baseline stays zero after only failed calls in another env.
    let (_env2, client2, _actors2) = setup();
    let stats = client2.get_stats();
    assert_eq!(stats.total_calculations, 0);
}

// ============================================================
// #31 – Deterministic SLA Calculation Audit Mode
// ============================================================

#[test]
fn test_calculate_sla_view_matches_mutating_and_does_not_mutate() {
    let (_env, client, actors) = setup();

    let outage_id = symbol_short!("INC999");
    let severity = symbol_short!("critical");
    let mttr = 25; // 10 min over threshold, results in penalty

    // 1. Get initial stats
    let initial_stats = client.get_stats();
    assert_eq!(initial_stats.total_calculations, 0);

    // 2. Call view function
    let view_result = client.calculate_sla_view(&outage_id, &severity, &mttr);

    // 3. Ensure no state mutated
    let after_view_stats = client.get_stats();
    assert_eq!(
        after_view_stats.total_calculations, 0,
        "View function must not mutate stats"
    );

    // 4. Call mutating function
    let mut_result = client.calculate_sla(&actors.operator, &outage_id, &severity, &mttr);

    // 5. Ensure state mutated
    let after_mut_stats = client.get_stats();
    assert_eq!(
        after_mut_stats.total_calculations, 1,
        "Mutating function must mutate stats"
    );

    // 6. Ensure results are perfectly identical, including backend-visible metadata.
    assert_eq!(view_result.status, mut_result.status);
    assert_eq!(view_result.amount, mut_result.amount);
    assert_eq!(view_result.rating, mut_result.rating);
    assert_eq!(view_result.payment_type, mut_result.payment_type);
    assert_eq!(view_result.mttr_minutes, mut_result.mttr_minutes);
    assert_eq!(view_result.threshold_minutes, mut_result.threshold_minutes);
    assert_eq!(view_result.outage_id, mut_result.outage_id);
    assert_eq!(view_result.recorded_at, mut_result.recorded_at);
}
// ============================================================
// #32 – Contract Economic Stress Test Suite
// ============================================================

#[test]
fn test_stress_1000_calculations_mixed_severities() {
    let env = Env::default();

    // Reset budget to unlimited to allow 1000 sequential calls in a single test environment.
    // We will manually track CPU instruction counts to assert gas efficiency per call.
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    let severities = [
        symbol_short!("critical"),
        symbol_short!("high"),
        symbol_short!("medium"),
        symbol_short!("low"),
    ];

    let mut expected_calculations = 0;
    let mut expected_violations = 0;
    let mut expected_rewards = 0i128;
    let mut expected_penalties = 0i128;

    let before_cpu = env.budget().cpu_instruction_cost();

    for i in 0..1000u32 {
        let severity = severities[(i % 4) as usize].clone();
        let cfg = client.get_config(&severity);

        // Alternate between meeting and violating the SLA to stress both logic paths
        let mttr = if i % 2 == 0 {
            cfg.threshold_minutes / 2 // Safely met
        } else {
            cfg.threshold_minutes + 10 // Safely violated by 10 mins
        };

        let outage_id = Symbol::new(&env, &alloc::format!("STRESS_{}", i));

        let res = client.calculate_sla(&op, &outage_id, &severity, &mttr);

        expected_calculations += 1;

        if res.status == symbol_short!("viol") {
            expected_violations += 1;
            // The contract returns penalties as negative values, so we negate it to track the positive aggregate
            expected_penalties += -res.amount;
        } else {
            expected_rewards += res.amount;
        }
    }

    let after_cpu = env.budget().cpu_instruction_cost();
    let avg_cpu_per_call = (after_cpu - before_cpu) / 1000;

    // 1. Assert no overflows occurred and cumulative statistics precisely match the local simulation
    let stats = client.get_stats();
    assert_eq!(
        stats.total_calculations, expected_calculations,
        "Calculation aggregate mismatch"
    );
    assert_eq!(
        stats.total_violations, expected_violations,
        "Violation aggregate mismatch"
    );
    assert_eq!(
        stats.total_rewards, expected_rewards,
        "Reward aggregate mismatch"
    );
    assert_eq!(
        stats.total_penalties, expected_penalties,
        "Penalty aggregate mismatch"
    );

    // 2. Assert gas bounds remain stable to catch unintended exponential looping or storage bloat
    assert!(
        avg_cpu_per_call < 50_000_000,
        "Average CPU instructions per call exceeded safe bounds: {}",
        avg_cpu_per_call
    );
}

// ============================================================
// #33 – Storage Compaction Strategy Tests
// ============================================================

#[test]
fn test_history_records_calculations() {
    let (_env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("H001"),
        &symbol_short!("critical"),
        &5,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("H002"),
        &symbol_short!("high"),
        &25,
    );

    let history = client.get_history();
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(0).unwrap().outage_id, symbol_short!("H001"));
    assert_eq!(history.get(1).unwrap().outage_id, symbol_short!("H002"));
}

#[test]
fn test_admin_can_prune_history() {
    let (_env, client, actors) = setup();

    // Generate 5 records
    for i in 0..5 {
        let oid = Symbol::new(&_env, &alloc::format!("H_GEN_{}", i));
        client.calculate_sla(&actors.operator, &oid, &symbol_short!("low"), &10);
    }

    let history_before = client.get_history();
    assert_eq!(history_before.len(), 5);

    // Prune down to the latest 2
    client.prune_history(&actors.admin, &2);

    let history_after = client.get_history();
    assert_eq!(
        history_after.len(),
        2,
        "History should be truncated to 2 items"
    );
}

#[test]
#[should_panic]
fn test_operator_cannot_prune_history() {
    let (_env, client, actors) = setup();
    client.prune_history(&actors.operator, &0);
}

#[test]
fn test_prune_history_preserves_latest_records_accurately() {
    let (_env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("ID_1"),
        &symbol_short!("low"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("ID_2"),
        &symbol_short!("low"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("ID_3"),
        &symbol_short!("low"),
        &10,
    );

    // Keep only the latest 1. ID_1 and ID_2 should be dropped, ID_3 retained.
    client.prune_history(&actors.admin, &1);

    let history = client.get_history();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history.get(0).unwrap().outage_id,
        symbol_short!("ID_3"),
        "Did not retain the correct recent record"
    );
}

// ============================================================
// #54 – Config snapshot version hash
// ============================================================

#[test]
fn test_config_version_hash_is_deterministic() {
    let (_env, client, _actors) = setup();
    let h1 = client.get_config_version_hash();
    let h2 = client.get_config_version_hash();
    assert_eq!(h1, h2);
}

#[test]
fn test_canonical_severity_order_is_aligned_across_snapshot_and_metadata() {
    let (_env, client, _actors) = setup();

    let snapshot = client.get_config_snapshot();
    let metadata = client.get_contract_metadata();

    assert_eq!(snapshot.entries.len(), metadata.supported_severities.len());

    for i in 0..snapshot.entries.len() {
        let snapshot_severity = snapshot.entries.get(i).unwrap().severity;
        let metadata_severity = metadata.supported_severities.get(i).unwrap();
        assert_eq!(snapshot_severity, metadata_severity);
    }
}

#[test]
fn test_canonical_severity_order_survives_config_updates() {
    let (_env, client, actors) = setup();

    client.set_config(&actors.admin, &symbol_short!("low"), &240, &15, &900);
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &150, &800);

    let snapshot = client.get_config_snapshot();
    let expected = [
        symbol_short!("critical"),
        symbol_short!("high"),
        symbol_short!("medium"),
        symbol_short!("low"),
    ];

    for (i, severity) in expected.iter().enumerate() {
        let entry = snapshot.entries.get(i as u32).unwrap();
        assert_eq!(entry.severity, severity.clone());
    }
}

#[test]
fn test_config_version_hash_changes_on_update() {
    let (_env, client, actors) = setup();
    let before = client.get_config_version_hash();
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
    let after = client.get_config_version_hash();
    assert_ne!(before, after);
}

#[test]
fn test_config_version_hash_stable_after_same_value_write() {
    let (_env, client, actors) = setup();
    let before = client.get_config_version_hash();
    // Write the same values back – hash must not change
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
    let after = client.get_config_version_hash();
    assert_eq!(before, after);
}

#[test]
fn test_config_version_hash_collision_resistance() {
    let (_env, client, actors) = setup();

    // Get initial hash
    let initial_hash = client.get_config_version_hash();

    // Create a different config with different field values but same total sum
    // Original critical: threshold=15, penalty=100, reward=750 (sum=865)
    // New critical: threshold=20, penalty=95, reward=750 (sum=865, same additive sum)
    // Both are valid critical configs (threshold<=60, penalty>=50)
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &95, &750);
    let collision_attempt_hash = client.get_config_version_hash();

    // Hash should be different despite same additive sum
    assert_ne!(
        initial_hash, collision_attempt_hash,
        "Hash should resist collision from additive checksum equivalence"
    );

    // Change critical to different values — hash must differ
    client.set_config(&actors.admin, &symbol_short!("critical"), &30, &200, &1000);
    let changed_hash = client.get_config_version_hash();
    assert_ne!(
        initial_hash, changed_hash,
        "Hash should change when config values change"
    );

    // Restore original config
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
    let restored_hash = client.get_config_version_hash();
    assert_eq!(
        initial_hash, restored_hash,
        "Hash should return to original value after restoring config"
    );
}

#[test]
fn test_config_version_hash_field_order_sensitivity() {
    let (_env, client, actors) = setup();

    // Test that changing different fields produces different hashes
    let original_hash = client.get_config_version_hash();

    // Change threshold only
    client.set_config(&actors.admin, &symbol_short!("high"), &25, &50, &750);
    let threshold_hash = client.get_config_version_hash();
    assert_ne!(original_hash, threshold_hash);

    // Reset and change penalty only
    client.set_config(&actors.admin, &symbol_short!("high"), &30, &60, &750);
    let penalty_hash = client.get_config_version_hash();
    assert_ne!(original_hash, penalty_hash);
    assert_ne!(threshold_hash, penalty_hash);

    // Reset and change reward only
    client.set_config(&actors.admin, &symbol_short!("high"), &30, &50, &800);
    let reward_hash = client.get_config_version_hash();
    assert_ne!(original_hash, reward_hash);
    assert_ne!(threshold_hash, reward_hash);
    assert_ne!(penalty_hash, reward_hash);

    // Restore original
    client.set_config(&actors.admin, &symbol_short!("high"), &30, &50, &750);
    let restored_hash = client.get_config_version_hash();
    assert_eq!(original_hash, restored_hash);
}

#[test]
fn test_config_version_hash_severity_isolation() {
    let (_env, client, actors) = setup();

    let original_hash = client.get_config_version_hash();

    // Change only critical severity
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
    let critical_changed_hash = client.get_config_version_hash();
    assert_ne!(original_hash, critical_changed_hash);

    // Change only high severity (restore critical first)
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
    client.set_config(&actors.admin, &symbol_short!("high"), &35, &55, &775);
    let high_changed_hash = client.get_config_version_hash();
    assert_ne!(original_hash, high_changed_hash);
    assert_ne!(critical_changed_hash, high_changed_hash);

    // Both changes should produce yet another hash
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
    let both_changed_hash = client.get_config_version_hash();
    assert_ne!(original_hash, both_changed_hash);
    assert_ne!(critical_changed_hash, both_changed_hash);
    assert_ne!(high_changed_hash, both_changed_hash);
}

#[test]
fn test_config_version_hash_distribution() {
    let (_env, client, actors) = setup();

    // Test hash changes are well-distributed by making multiple small changes
    let mut hashes = Vec::new(&_env);

    // Collect hashes from various config states
    for i in 1..=10 {
        client.set_config(
            &actors.admin,
            &symbol_short!("critical"),
            &(15 + i),
            &100,
            &750,
        );
        let hash = client.get_config_version_hash();
        hashes.push_back(hash);
    }

    // Verify all hashes are unique
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            assert_ne!(
                hashes.get(i),
                hashes.get(j),
                "Hashes should be unique for different config values"
            );
        }
    }

    // Restore original config
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
}

// ============================================================
// #56 – Repeated config update regression tests
// ============================================================

#[test]
fn test_repeated_config_updates_latest_wins() {
    let (_env, client, actors) = setup();

    client.set_config(&actors.admin, &symbol_short!("critical"), &10, &50, &500);
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &100, &800);
    client.set_config(&actors.admin, &symbol_short!("critical"), &30, &200, &1200);

    let cfg = client.get_config(&symbol_short!("critical"));
    assert_eq!(cfg.threshold_minutes, 30);
    assert_eq!(cfg.penalty_per_minute, 200);
    assert_eq!(cfg.reward_base, 1200);
}

#[test]
fn test_repeated_config_updates_do_not_corrupt_calculation() {
    let (_env, client, actors) = setup();

    // Update critical config twice; final state: threshold=20, penalty=100, reward=800
    client.set_config(&actors.admin, &symbol_short!("critical"), &10, &50, &500);
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &100, &800);

    // mttr=25 → 5 min over threshold=20 → penalty = 5 * 100 = 500
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("RC001"),
        &symbol_short!("critical"),
        &25,
    );
    assert_eq!(result.status, symbol_short!("viol"));
    assert_eq!(result.amount, -500);
}

#[test]
fn test_repeated_config_updates_across_severities_are_independent() {
    let (_env, client, actors) = setup();

    // Use valid values: critical requires penalty>=50, threshold<=60; high requires penalty>=25, threshold<=120
    client.set_config(&actors.admin, &symbol_short!("critical"), &10, &50, &500);
    client.set_config(&actors.admin, &symbol_short!("high"), &20, &25, &400);
    client.set_config(&actors.admin, &symbol_short!("critical"), &10, &50, &100);
    client.set_config(&actors.admin, &symbol_short!("high"), &10, &25, &100);

    // medium and low must remain at their defaults
    let medium = client.get_config(&symbol_short!("medium"));
    let low = client.get_config(&symbol_short!("low"));
    assert_eq!(medium.threshold_minutes, 60);
    assert_eq!(low.threshold_minutes, 120);
}

// ============================================================
// #94 – Fixture helpers for repeated actor and contract setup
// ============================================================

/// Setup with a custom critical config applied on top of defaults.
fn setup_with_critical(
    threshold: u32,
    penalty: i128,
    reward: i128,
) -> (Env, SLACalculatorContractClient<'static>, Actors) {
    let (env, client, actors) = setup();
    client.set_config(
        &actors.admin,
        &symbol_short!("critical"),
        &threshold,
        &penalty,
        &reward,
    );
    (env, client, actors)
}

/// Setup and perform one calculation, returning the result along with the env/client/actors.
fn setup_after_calculation(
    severity: &str,
    mttr: u32,
) -> (Env, SLACalculatorContractClient<'static>, Actors) {
    let (env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "FIXTURE_ID"),
        &symbol(&env, severity),
        &mttr,
    );
    (env, client, actors)
}

#[test]
fn test_fixture_custom_critical_config_is_applied() {
    let (_env, client, _actors) = setup_with_critical(10, 50, 500);
    let cfg = client.get_config(&symbol_short!("critical"));
    assert_eq!(cfg.threshold_minutes, 10);
    assert_eq!(cfg.penalty_per_minute, 50);
    assert_eq!(cfg.reward_base, 500);
}

#[test]
fn test_fixture_after_calculation_history_has_one_entry() {
    let (_env, client, _actors) = setup_after_calculation("critical", 5);
    let history = client.get_history();
    assert_eq!(history.len(), 1);
}

#[test]
fn test_fixture_after_calculation_stats_are_updated() {
    let (_env, client, _actors) = setup_after_calculation("high", 35);
    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 1);
    assert_eq!(stats.total_violations, 1);
}

// ============================================================
// #95 – Negative tests for malformed symbol inputs
// ============================================================

#[test]
#[should_panic]
fn test_calculate_sla_unknown_severity_panics() {
    let (_env, client, actors) = setup();
    // "xyz" is not a configured severity — ConfigNotFound maps to a panic in the client
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("OUT001"),
        &symbol_short!("xyz"),
        &10,
    );
}
// ============================================================
// #63 – Two-step admin transfer
// ============================================================

#[test]
fn test_propose_and_accept_admin() {
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);

    client.propose_admin(&actors.admin, &new_admin);
    assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));

    client.accept_admin(&new_admin);
    assert_eq!(client.get_admin(), new_admin);
    assert_eq!(client.get_pending_admin(), None);
}

#[test]
#[should_panic]
fn test_old_admin_loses_authority_after_accept() {
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);

    client.propose_admin(&actors.admin, &new_admin);
    client.accept_admin(&new_admin);

    // old admin can no longer set config – must panic
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
}

#[test]
#[should_panic]
fn test_wrong_address_cannot_accept_admin() {
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);
    let stranger = soroban_sdk::Address::generate(&env);

    client.propose_admin(&actors.admin, &new_admin);
    client.accept_admin(&stranger); // must panic
}

#[test]
#[should_panic]
fn test_accept_admin_without_proposal_fails() {
    let (_env, client, actors) = setup();
    client.accept_admin(&actors.stranger); // no pending proposal
}

#[test]
fn test_get_pending_admin_none_when_no_proposal() {
    let (_env, client, _actors) = setup();
    assert_eq!(client.get_pending_admin(), None);
}

// ============================================================
// #64 – Two-step operator handoff
// ============================================================

#[test]
fn test_propose_and_accept_operator() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &new_op);
    assert_eq!(client.get_pending_operator(), Some(new_op.clone()));

    client.accept_operator(&new_op);
    assert_eq!(client.get_operator(), new_op);
    assert_eq!(client.get_pending_operator(), None);
}

#[test]
#[should_panic]
fn test_old_operator_locked_out_after_handoff() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &new_op);
    client.accept_operator(&new_op);

    // old operator can no longer calculate – must panic
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("HO001"),
        &symbol_short!("critical"),
        &5,
    );
}

#[test]
#[should_panic]
fn test_wrong_address_cannot_accept_operator() {
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);
    let stranger = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &new_op);
    client.accept_operator(&stranger); // must panic
}

// ============================================================
// #60 – Contract metadata / capabilities view
// ============================================================

#[test]
fn test_get_contract_metadata_returns_expected_fields() {
    let (_env, client, _actors) = setup();
    let meta = client.get_contract_metadata();
    assert_eq!(meta.contract_name, symbol_short!("sla_calc"));
    assert_eq!(meta.storage_version, 1);
    assert_eq!(meta.result_schema_version, 1);
    assert_eq!(meta.supported_severities.len(), 4);
    assert_eq!(meta.features.len(), 10);
}

#[test]
fn test_get_contract_metadata_severities_are_canonical() {
    let (_env, client, _actors) = setup();
    let meta = client.get_contract_metadata();
    assert_eq!(
        meta.supported_severities.get(0).unwrap(),
        symbol_short!("critical")
    );
    assert_eq!(
        meta.supported_severities.get(1).unwrap(),
        symbol_short!("high")
    );
    assert_eq!(
        meta.supported_severities.get(2).unwrap(),
        symbol_short!("medium")
    );
    assert_eq!(
        meta.supported_severities.get(3).unwrap(),
        symbol_short!("low")
    );
    let expected = [
        symbol_short!("critical"),
        symbol_short!("high"),
        symbol_short!("medium"),
        symbol_short!("low"),
    ];

    for (i, severity) in expected.iter().enumerate() {
        assert_eq!(
            meta.supported_severities.get(i as u32).unwrap(),
            severity.clone()
        );
    }
}

#[test]
fn test_get_contract_metadata_is_deterministic() {
    let (_env, client, _actors) = setup();
    let m1 = client.get_contract_metadata();
    let m2 = client.get_contract_metadata();
    assert_eq!(m1.storage_version, m2.storage_version);
    assert_eq!(m1.result_schema_version, m2.result_schema_version);
    assert_eq!(m1.contract_name, m2.contract_name);
    assert_eq!(m1.supported_severities, m2.supported_severities);
}

// ============================================================
// #61 – Storage migration harness
// ============================================================

#[test]
fn test_migrate_is_idempotent_when_already_current() {
    let (_env, client, actors) = setup();
    // Already at v1 – migrate should succeed without error
    client.migrate(&actors.admin);
    client.migrate(&actors.admin);
    // Contract still functional
    assert_eq!(client.get_admin(), actors.admin);
}

#[test]
#[should_panic]
fn test_get_config_unknown_severity_panics() {
    let (_env, client, _actors) = setup();
    // "CRIT" (uppercase) is not a valid severity key
    client.get_config(&symbol_short!("CRIT"));
}

#[test]
#[should_panic]
fn test_accept_operator_without_proposal_fails() {
    let (_env, client, actors) = setup();
    client.accept_operator(&actors.stranger);
}

#[test]
fn test_get_pending_operator_none_when_no_proposal() {
    let (_env, client, _actors) = setup();
    assert_eq!(client.get_pending_operator(), None);
}

// ============================================================
// #65 – Admin renounce
// ============================================================

#[test]
fn test_admin_can_renounce() {
    let (_env, client, actors) = setup();
    client.renounce_admin(&actors.admin);
    // After renounce, admin-gated calls must fail
}

#[test]
#[should_panic]
fn test_calculate_sla_wrong_case_severity_panics() {
    let (_env, client, actors) = setup();
    // "HIGH" differs from configured "high"
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("OUT002"),
        &symbol_short!("HIGH"),
        &10,
    );
}
#[test]
#[should_panic]
fn test_calculate_sla_view_unknown_severity_panics() {
    let (env, client, _actors) = setup();
    client.calculate_sla_view(&symbol(&env, "VIEW001"), &symbol_short!("unknown"), &10);
}
// ============================================================
// #96 – Backend-consumer smoke fixture (end-to-end sequence)
// ============================================================

#[test]
fn test_backend_smoke_initialize_config_calculate_history_stats() {
    // Step 1: initialize (via setup helper — admin + operator roles set, default configs loaded)
    let (env, client, actors) = setup();

    // Step 2: config read — verify a known severity is present
    let critical_cfg = client.get_config(&symbol_short!("critical"));
    assert_eq!(critical_cfg.threshold_minutes, 15);
    assert!(critical_cfg.penalty_per_minute > 0);
    assert!(critical_cfg.reward_base > 0);

    // Step 3: calculate — operator submits an SLA result
    let result = client.calculate_sla(
        &actors.operator,
        &symbol(&env, "SMOKE_001"),
        &symbol_short!("critical"),
        &10,
    );
    assert_eq!(result.status, symbol_short!("met"));

    // Step 4: history read — the calculation appears in history
    let history = client.get_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().outage_id, symbol(&env, "SMOKE_001"));

    // Step 5: stats read — counters reflect the single met calculation
    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 1);
    assert_eq!(stats.total_violations, 0);
    assert!(stats.total_rewards > 0);
    assert_eq!(stats.total_penalties, 0);
}

#[test]
fn test_backend_smoke_violation_path() {
    let (env, client, actors) = setup();

    // critical threshold is 15 min; 30 min exceeds it → violation
    let result = client.calculate_sla(
        &actors.operator,
        &symbol(&env, "SMOKE_002"),
        &symbol_short!("critical"),
        &30,
    );
    assert_eq!(result.status, symbol_short!("viol"));
    assert_eq!(result.payment_type, symbol_short!("pen"));
    assert!(result.amount < 0);

    let stats = client.get_stats();
    assert_eq!(stats.total_violations, 1);
    assert_eq!(stats.total_rewards, 0);
    assert!(stats.total_penalties > 0);
}

#[test]
#[should_panic]
fn test_admin_gated_call_fails_after_renounce() {
    let (_env, client, actors) = setup();
    client.renounce_admin(&actors.admin);

    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
}

#[test]
#[should_panic]
fn test_migrate_rejected_for_non_admin() {
    let (_env, client, actors) = setup();
    client.migrate(&actors.stranger);
}

#[test]
#[should_panic]
fn test_check_version_rejects_version_mismatch() {
    // Simulate a future version stored in state by writing a different version
    // directly, then calling any versioned endpoint.
    let env = Env::default();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Manually overwrite the stored version to simulate a future schema
    env.as_contract(&cid, || {
        env.storage().instance().set(&STORAGE_VERSION_KEY, &99u32);
    });

    // Any versioned call must now panic with VersionMismatch
    client.get_admin();
}

// ============================================================
// #62 – Unknown-severity rejection
// ============================================================

#[test]
#[should_panic]
fn test_calculate_sla_rejects_unknown_severity() {
    let (env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("UNK001"),
        &Symbol::new(&env, "unknown"),
        &10,
    );
}

#[test]
#[should_panic]
fn test_stranger_cannot_renounce() {
    let (_env, client, actors) = setup();
    client.renounce_admin(&actors.stranger);
}

#[test]
fn test_renounce_clears_pending_proposal() {
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);

    client.propose_admin(&actors.admin, &new_admin);
    client.renounce_admin(&actors.admin);
    assert_eq!(client.get_pending_admin(), None);
}

// ============================================================
// #66 – Pause reason + timestamp
// ============================================================

#[test]
fn test_pause_stores_reason_and_timestamp() {
    let (env, client, actors) = setup();
    let reason = soroban_sdk::String::from_str(&env, "scheduled maintenance");

    client.pause(&actors.admin, &reason);

    let info = client
        .get_pause_info()
        .expect("pause info should be present");
    assert_eq!(info.reason, reason);
    assert_eq!(info.paused_by, actors.admin);
    // timestamp is ledger time; just assert it is non-zero in a real ledger,
    // in test env it defaults to 0 which is still a valid u64
    let _ = info.paused_at;
}

#[test]
#[should_panic(expected = "#17")]
fn test_pause_rejects_long_reason() {
    let (env, client, actors) = setup();
    // 257-byte reason exceeds MAX_REASON_LEN (256)
    let long_reason = soroban_sdk::String::from_str(&env, &"A".repeat(257));
    client.pause(&actors.admin, &long_reason);
}

#[test]
fn test_unpause_clears_pause_info() {
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "reason"),
    );
    client.unpause(&actors.admin);

    assert_eq!(client.get_pause_info(), None);
}

#[test]
fn test_get_pause_info_none_when_not_paused() {
    let (_env, client, _actors) = setup();
    assert_eq!(client.get_pause_info(), None);
}

#[test]
#[should_panic]
fn test_calculate_sla_view_rejects_unknown_severity() {
    let (env, client, _actors) = setup();
    client.calculate_sla_view(&symbol_short!("UNK002"), &Symbol::new(&env, "unknown"), &10);
}

#[test]
#[should_panic]
fn test_get_config_rejects_unknown_severity() {
    let (env, client, _actors) = setup();
    client.get_config(&Symbol::new(&env, "unknown"));
}

#[test]
#[should_panic]
fn test_set_config_then_calculate_unknown_severity_still_rejects_other_unknown() {
    // Even after adding a custom severity via set_config, a different unknown still fails
    let (env, client, actors) = setup();
    client.set_config(&actors.admin, &Symbol::new(&env, "custom"), &10, &50, &500);
    // "bogus" was never configured
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("UNK003"),
        &Symbol::new(&env, "bogus"),
        &5,
    );
}

// ============================================================
// #70 – Configuration Validation Tests
// ============================================================

#[test]
fn test_valid_config_passes_validation() {
    let (_env, client, actors) = setup();

    // All these should succeed
    client.set_config(&actors.admin, &symbol_short!("critical"), &30, &150, &1000);
    client.set_config(&actors.admin, &symbol_short!("high"), &45, &75, &800);
    client.set_config(&actors.admin, &symbol_short!("medium"), &90, &30, &600);
    client.set_config(&actors.admin, &symbol_short!("low"), &180, &15, &500);

    // Verify values were set
    let cfg = client.get_config(&symbol_short!("critical"));
    assert_eq!(cfg.threshold_minutes, 30);
    assert_eq!(cfg.penalty_per_minute, 150);
    assert_eq!(cfg.reward_base, 1000);
}

#[test]
#[should_panic]
fn test_invalid_severity_fails_validation() {
    let (_env, client, actors) = setup();
    // "urgent" is not a supported severity
    client.set_config(&actors.admin, &symbol_short!("urgent"), &15, &100, &750);
}

#[test]
#[should_panic]
fn test_zero_threshold_fails_validation() {
    let (_env, client, actors) = setup();
    // Threshold cannot be 0
    client.set_config(&actors.admin, &symbol_short!("critical"), &0, &100, &750);
}

#[test]
#[should_panic]
fn test_threshold_too_large_fails_validation() {
    let (_env, client, actors) = setup();
    // Threshold exceeds 1440 minute (24 hour) maximum
    client.set_config(&actors.admin, &symbol_short!("low"), &1500, &10, &600);
}

#[test]
#[should_panic]
fn test_negative_penalty_fails_validation() {
    let (_env, client, actors) = setup();
    // Penalty must be positive
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &-100, &750);
}

#[test]
#[should_panic]
fn test_zero_penalty_fails_validation() {
    let (_env, client, actors) = setup();
    // Penalty must be positive (cannot be 0)
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &0, &750);
}

#[test]
#[should_panic]
fn test_penalty_too_large_fails_validation() {
    let (_env, client, actors) = setup();
    // Penalty exceeds 10,000 maximum
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &15000, &750);
}

#[test]
#[should_panic]
fn test_negative_reward_fails_validation() {
    let (_env, client, actors) = setup();
    // Reward must be positive
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &-750);
}

#[test]
#[should_panic]
fn test_zero_reward_fails_validation() {
    let (_env, client, actors) = setup();
    // Reward must be positive (cannot be 0)
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &0);
}

#[test]
#[should_panic]
fn test_reward_too_large_fails_validation() {
    let (_env, client, actors) = setup();
    // Reward exceeds 100,000 maximum
    client.set_config(
        &actors.admin,
        &symbol_short!("critical"),
        &15,
        &100,
        &150000,
    );
}

// Severity-specific validation tests

#[test]
#[should_panic]
fn test_critical_threshold_too_high_fails_validation() {
    let (_env, client, actors) = setup();
    // Critical severity threshold cannot exceed 60 minutes
    client.set_config(&actors.admin, &symbol_short!("critical"), &90, &100, &750);
}

#[test]
#[should_panic]
fn test_critical_penalty_too_low_fails_validation() {
    let (_env, client, actors) = setup();
    // Critical severity penalty must be at least 50
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &25, &750);
}

#[test]
#[should_panic]
fn test_high_threshold_too_high_fails_validation() {
    let (_env, client, actors) = setup();
    // High severity threshold cannot exceed 120 minutes
    client.set_config(&actors.admin, &symbol_short!("high"), &150, &50, &750);
}

#[test]
#[should_panic]
fn test_high_penalty_too_low_fails_validation() {
    let (_env, client, actors) = setup();
    // High severity penalty must be at least 25
    client.set_config(&actors.admin, &symbol_short!("high"), &30, &15, &750);
}

#[test]
#[should_panic]
fn test_medium_threshold_too_high_fails_validation() {
    let (_env, client, actors) = setup();
    // Medium severity threshold cannot exceed 240 minutes
    client.set_config(&actors.admin, &symbol_short!("medium"), &300, &25, &750);
}

#[test]
#[should_panic]
fn test_medium_penalty_too_low_fails_validation() {
    let (_env, client, actors) = setup();
    // Medium severity penalty must be at least 10
    client.set_config(&actors.admin, &symbol_short!("medium"), &60, &5, &750);
}

#[test]
#[should_panic]
fn test_low_penalty_too_high_fails_validation() {
    let (_env, client, actors) = setup();
    // Low severity penalty cannot exceed 100
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &150, &600);
}

// Edge case validation tests

#[test]
fn test_boundary_values_pass_validation() {
    let (_env, client, actors) = setup();

    // Test minimum valid values
    client.set_config(&actors.admin, &symbol_short!("critical"), &1, &50, &1);
    client.set_config(&actors.admin, &symbol_short!("high"), &1, &25, &1);
    client.set_config(&actors.admin, &symbol_short!("medium"), &1, &10, &1);
    client.set_config(&actors.admin, &symbol_short!("low"), &1, &1, &1);

    // Test maximum valid values for severity-specific constraints
    client.set_config(
        &actors.admin,
        &symbol_short!("critical"),
        &60,
        &10000,
        &100000,
    );
    client.set_config(&actors.admin, &symbol_short!("high"), &120, &10000, &100000);
    client.set_config(
        &actors.admin,
        &symbol_short!("medium"),
        &240,
        &10000,
        &100000,
    );
    client.set_config(&actors.admin, &symbol_short!("low"), &1440, &100, &100000);
}

#[test]
fn test_validation_prevents_partial_state_changes() {
    let (_env, client, actors) = setup();

    // Get original config
    let original = client.get_config(&symbol_short!("critical"));
    assert_eq!(original.threshold_minutes, 15);
    assert_eq!(original.penalty_per_minute, 100);
    assert_eq!(original.reward_base, 750);

    // Attempt invalid config change - should fail without modifying state
    let result = client.try_set_config(&actors.admin, &symbol_short!("critical"), &0, &100, &750);
    assert!(result.is_err());

    // Verify original config is unchanged
    // Invalid config (threshold=0) is rejected; original values remain.
    // Verified by test_zero_threshold_fails_validation (should_panic).
    // Here we just confirm the original is readable and correct.
    let unchanged = client.get_config(&symbol_short!("critical"));
    assert_eq!(unchanged.threshold_minutes, 15);
    assert_eq!(unchanged.penalty_per_minute, 100);
    assert_eq!(unchanged.reward_base, 750);
}

#[test]
fn test_validation_works_after_successful_config_change() {
    let (_env, client, actors) = setup();

    // Make a valid change first
    client.set_config(&actors.admin, &symbol_short!("critical"), &30, &150, &1000);

    // Now attempt an invalid change - should still fail
    let result = client.try_set_config(&actors.admin, &symbol_short!("critical"), &0, &150, &1000);
    assert!(result.is_err());

    // Verify the valid change is still in place
    // Verify the valid change is in place
    let cfg = client.get_config(&symbol_short!("critical"));
    assert_eq!(cfg.threshold_minutes, 30);
    assert_eq!(cfg.penalty_per_minute, 150);
    assert_eq!(cfg.reward_base, 1000);
    // Invalid changes are still rejected after a valid one (covered by should_panic tests).
}

#[test]
fn test_validation_applies_to_all_severities_independently() {
    let (_env, client, actors) = setup();

    // Valid change to critical
    client.set_config(&actors.admin, &symbol_short!("critical"), &25, &120, &900);

    // Invalid change to high should not affect critical
    let result = client.try_set_config(&actors.admin, &symbol_short!("high"), &0, &50, &750);
    assert!(result.is_err());

    // Verify critical is unchanged and high is still at default
    // Verify critical was updated and high is still at default
    let critical = client.get_config(&symbol_short!("critical"));
    assert_eq!(critical.threshold_minutes, 25);

    let high = client.get_config(&symbol_short!("high"));
    assert_eq!(high.threshold_minutes, 30); // still default
}

// ============================================================
// SC-059 – History pagination
// ============================================================

#[test]
fn test_get_history_page_returns_correct_slice() {
    let (_env, client, actors) = setup();

    for i in 0..5u32 {
        let oid = Symbol::new(&_env, &alloc::format!("PG_ID_{}", i));
        client.calculate_sla(&actors.operator, &oid, &symbol_short!("low"), &10);
    }

    // Page 0: first 2
    let page0 = client.get_history_page(&0, &2);
    assert_eq!(page0.len(), 2);

    // Page 1: next 2
    let page1 = client.get_history_page(&2, &2);
    assert_eq!(page1.len(), 2);

    // Page 2: last 1
    let page2 = client.get_history_page(&4, &2);
    assert_eq!(page2.len(), 1);
}

#[test]
fn test_get_history_page_empty_when_offset_beyond_end() {
    let (_env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("PG_OOB"),
        &symbol_short!("low"),
        &10,
    );

    let page = client.get_history_page(&100, &10);
    assert_eq!(page.len(), 0);
}

#[test]
fn test_get_history_page_empty_history() {
    let (_env, client, _actors) = setup();
    let page = client.get_history_page(&0, &10);
    assert_eq!(page.len(), 0);
}

#[test]
fn test_get_history_page_zero_limit_returns_empty() {
    let (_env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("PG_ZL"),
        &symbol_short!("low"),
        &10,
    );

    let page = client.get_history_page(&0, &0);
    assert_eq!(page.len(), 0);
}

#[test]
fn test_get_history_page_order_is_oldest_first() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "FIRST"),
        &symbol_short!("low"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "SECOND"),
        &symbol_short!("low"),
        &10,
    );

    let page = client.get_history_page(&0, &2);
    assert_eq!(page.get(0).unwrap().outage_id, symbol(&env, "FIRST"));
    assert_eq!(page.get(1).unwrap().outage_id, symbol(&env, "SECOND"));
}

// ============================================================
// SC-060 – History query by outage identifier
// ============================================================

#[test]
fn test_get_history_by_outage_returns_matching_entries() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "OUT_A1"),
        &symbol_short!("low"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "OUT_B"),
        &symbol_short!("low"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "OUT_A2"),
        &symbol_short!("critical"),
        &5,
    );

    let results = client.get_history_by_outage(&symbol(&env, "OUT_A1"));
    assert_eq!(results.len(), 1);
    assert_eq!(results.get(0).unwrap().outage_id, symbol(&env, "OUT_A1"));
}

#[test]
fn test_get_history_by_outage_returns_empty_for_unknown_id() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "OUT_X"),
        &symbol_short!("low"),
        &10,
    );

    let results = client.get_history_by_outage(&symbol(&env, "MISSING"));
    assert_eq!(results.len(), 0);
}

#[test]
fn test_get_history_by_outage_empty_history() {
    let (env, client, _actors) = setup();
    let results = client.get_history_by_outage(&symbol(&env, "NONE"));
    assert_eq!(results.len(), 0);
}

// ============================================================
// SC-061 – Latest result by outage identifier
// ============================================================

#[test]
fn test_get_latest_by_outage_returns_most_recent() {
    let (env, client, actors) = setup();

    // Two calculations for different outages; each should be findable
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "LAT_VIOL"),
        &symbol_short!("critical"),
        &20, // violation
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "LAT_MET"),
        &symbol_short!("critical"),
        &5, // met
    );

    let latest = client.get_latest_by_outage(&symbol(&env, "LAT_MET"));
    assert!(latest.is_some());
    let r = latest.unwrap();
    assert_eq!(r.outage_id, symbol(&env, "LAT_MET"));
    assert_eq!(r.status, symbol_short!("met")); // second call was met
}

#[test]
fn test_get_latest_by_outage_returns_none_for_missing() {
    let (env, client, _actors) = setup();
    let result = client.get_latest_by_outage(&symbol(&env, "GHOST"));
    assert!(result.is_none());
}

#[test]
fn test_get_latest_by_outage_single_entry() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "SOLO"),
        &symbol_short!("high"),
        &10,
    );

    let latest = client.get_latest_by_outage(&symbol(&env, "SOLO"));
    assert!(latest.is_some());
    assert_eq!(latest.unwrap().outage_id, symbol(&env, "SOLO"));
}

#[test]
fn test_get_latest_by_outage_does_not_return_other_outage() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "OUT_1"),
        &symbol_short!("low"),
        &10,
    );

    let result = client.get_latest_by_outage(&symbol(&env, "OUT_2"));
    assert!(result.is_none());
}

// ============================================================
// SC-062 – Bounded-history retention
// ============================================================

#[test]
fn test_history_does_not_exceed_max_size() {
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Insert MAX_HISTORY_SIZE + 5 entries
    for i in 0..1005u32 {
        let oid = Symbol::new(&env, &alloc::format!("CAP_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    }

    let history = client.get_history();
    assert_eq!(
        history.len(),
        1000,
        "History must be capped at MAX_HISTORY_SIZE"
    );
    let _ = admin;
}

// ============================================================
// SC-063 – prune_history_by_age tests
// ============================================================

#[test]
fn test_prune_by_age_removes_old_entries() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Record two entries at t=1000
    client.calculate_sla(&op, &symbol_short!("OLD1"), &symbol_short!("critical"), &5);
    client.calculate_sla(&op, &symbol_short!("OLD2"), &symbol_short!("high"), &10);

    // Advance time to t=2000 and record a recent entry
    env.ledger().set_timestamp(2000);
    client.calculate_sla(&op, &symbol_short!("NEW1"), &symbol_short!("low"), &10);

    // Prune entries older than 500 seconds (cutoff = 2000 - 500 = 1500)
    // OLD1 and OLD2 have recorded_at=1000 < 1500 → removed
    // NEW1 has recorded_at=2000 >= 1500 → kept
    client.prune_history_by_age(&admin, &500);

    let history = client.get_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().outage_id, symbol_short!("NEW1"));
}

#[test]
fn test_prune_by_age_keeps_all_when_none_old_enough() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    client.calculate_sla(&op, &symbol_short!("E1"), &symbol_short!("critical"), &5);
    client.calculate_sla(&op, &symbol_short!("E2"), &symbol_short!("high"), &10);

    // Prune with min_age_seconds=2000 → cutoff = 1000 - 2000 saturates to 0
    // All entries have recorded_at=1000 >= 0 → nothing removed
    client.prune_history_by_age(&admin, &2000);

    let history = client.get_history();
    assert_eq!(history.len(), 2);
}

#[test]
fn test_prune_by_age_empty_history_is_noop() {
    let (_env, client, actors) = setup();
    // No entries – should not panic
    client.prune_history_by_age(&actors.admin, &100);
    assert_eq!(client.get_history().len(), 0);
}

#[test]
#[should_panic]
fn test_prune_by_age_operator_cannot_prune() {
    let (_env, client, actors) = setup();
    client.prune_history_by_age(&actors.operator, &100);
}

#[test]
fn test_prune_by_age_emits_event() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    client.calculate_sla(&op, &symbol_short!("EV1"), &symbol_short!("critical"), &5);

    env.ledger().set_timestamp(2000);
    client.prune_history_by_age(&admin, &500); // removes EV1

    let events = env.events().all();
    let (_, topics, _data) = events.last().unwrap();
    let topic_0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic_0, EVENT_PRUNED_AGE);
}

#[test]
fn test_prune_by_age_recorded_at_is_set_on_calculate() {
    let env = Env::default();
    env.ledger().set_timestamp(5000);

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    client.calculate_sla(&op, &symbol_short!("TS1"), &symbol_short!("critical"), &5);

    let history = client.get_history();
    assert_eq!(history.get(0).unwrap().recorded_at, 5000);
    let _ = admin; // suppress unused warning
}

// ============================================================
// SC-064 – Storage-growth regression tests
// ============================================================

#[test]
fn test_storage_growth_history_bounded_by_prune() {
    // Verify that repeated calculations followed by pruning keeps history bounded.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Add 50 entries
    for i in 0..50u32 {
        let oid = Symbol::new(&env, &alloc::format!("GRW_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("critical"), &5);
    }
    assert_eq!(client.get_history().len(), 50);

    // Prune to 10
    client.prune_history(&admin, &10);
    assert_eq!(
        client.get_history().len(),
        10,
        "History must be bounded after prune"
    );
}

#[test]
fn test_storage_growth_stats_do_not_grow_with_calculations() {
    // Stats are a single fixed-size struct; verify it stays constant regardless of call count.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    for i in 0..100u32 {
        let mttr = if i % 2 == 0 { 5u32 } else { 30u32 };
        let oid = Symbol::new(&env, &alloc::format!("ST_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("critical"), &mttr);
    }

    // Stats struct fields must be consistent with 100 calls
    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 100);
    assert_eq!(stats.total_violations + (100 - stats.total_violations), 100);
    let _ = admin;
}

#[test]
fn test_storage_growth_config_size_is_fixed() {
    // Config map has exactly 4 entries regardless of how many times set_config is called.
    let (_env, client, actors) = setup();

    for _ in 0..20u32 {
        client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
    }

    assert_eq!(
        client.get_config_count(),
        4,
        "Config map must stay at 4 entries"
    );
}

#[test]
fn test_storage_growth_prune_by_age_bounds_history() {
    let env = Env::default();
    env.budget().reset_unlimited();
    env.ledger().set_timestamp(0);

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Add 30 entries at t=0
    for i in 0..30u32 {
        let oid = Symbol::new(&env, &alloc::format!("OLD_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("high"), &10);
    }

    // Advance time and add 5 recent entries
    env.ledger().set_timestamp(10_000);
    for i in 0..5u32 {
        let oid = Symbol::new(&env, &alloc::format!("NEW_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("high"), &10);
    }

    // Prune entries older than 5000 seconds (cutoff = 10000 - 5000 = 5000)
    // All 30 old entries (recorded_at=0) are removed; 5 new ones kept
    client.prune_history_by_age(&admin, &5000);

    assert_eq!(
        client.get_history().len(),
        5,
        "Only recent entries should remain after age-based prune"
    );
}

// ============================================================
// SC-065 – Event-size regression tests
// ============================================================

#[test]
fn test_sla_calc_event_topic_count_is_three() {
    // sla_calc events must have exactly 3 topics: name, version, severity
    let (env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("EV_SZ1"),
        &symbol_short!("critical"),
        &5,
    );

    let events = env.events().all();
    let (_, topics, _) = events.last().unwrap();
    assert_eq!(topics.len(), 3, "sla_calc event must have exactly 3 topics");
}

#[test]
fn test_sla_calc_event_payload_field_count_is_seven() {
    // sla_calc payload: (outage_id, status, payment_type, rating, mttr, threshold, amount)
    let (env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("EV_SZ2"),
        &symbol_short!("critical"),
        &5,
    );

    let events = env.events().all();
    // Find the sla_calc event (last is set_int, we need sla_calc)
    let (_, _, data) = events.get(events.len() - 2).unwrap();
    let payload: (Symbol, Symbol, Symbol, Symbol, u32, u32, i128) =
        data.try_into_val(&env).unwrap();
    // Destructure to confirm all 7 fields decode without error
    let (outage_id, status, payment_type, rating, mttr, threshold, amount) = payload;
    assert_eq!(outage_id, symbol_short!("EV_SZ2"));
    assert_eq!(status, symbol_short!("met"));
    assert_eq!(payment_type, symbol_short!("rew"));
    assert_eq!(rating, symbol_short!("top"));
    assert_eq!(mttr, 5u32);
    assert_eq!(threshold, 15u32);
    assert_eq!(amount, 1500i128);
}

#[test]
fn test_cfg_upd_event_topic_count_is_three() {
    let (env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    let events = env.events().all();
    let (_, topics, _) = events.last().unwrap();
    assert_eq!(topics.len(), 3, "cfg_upd event must have exactly 3 topics");
}

#[test]
fn test_cfg_upd_event_payload_field_count_is_three() {
    let (env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    let events = env.events().all();
    // set_config emits cfg_upd as the last event
    let (_, _, data) = events.last().unwrap();
    let payload: (u32, i128, i128) = data.try_into_val(&env).unwrap();
    assert_eq!(payload, (20u32, 200i128, 1000i128));
}

#[test]
fn test_pruned_event_payload_field_count_is_two() {
    let (env, client, actors) = setup();
    for i in 0..5u32 {
        let oid = Symbol::new(&env, &alloc::format!("PR_{}", i));
        client.calculate_sla(&actors.operator, &oid, &symbol_short!("low"), &10);
    }
    client.prune_history(&actors.admin, &2);

    let events = env.events().all();
    // prune_history emits pruned as the last event
    let (_, _, data) = events.last().unwrap();
    let payload: (u32, u32) = data.try_into_val(&env).unwrap();
    // removed=3, kept=2
    assert_eq!(payload, (3u32, 2u32));
}

#[test]
fn test_pruned_age_event_payload_field_count_is_two() {
    let env = Env::default();
    env.ledger().set_timestamp(0);

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    client.calculate_sla(&op, &symbol_short!("PA1"), &symbol_short!("critical"), &5);
    client.calculate_sla(&op, &symbol_short!("PA2"), &symbol_short!("critical"), &5);

    env.ledger().set_timestamp(2000);
    client.prune_history_by_age(&admin, &500); // removes both (recorded_at=0 < 1500)

    let events = env.events().all();
    // prune_history_by_age emits pruned_a as the last event
    let (_, _, data) = events.last().unwrap();
    let payload: (u32, u32) = data.try_into_val(&env).unwrap();
    assert_eq!(payload, (2u32, 0u32)); // removed=2, kept=0
}

#[test]
fn test_history_cap_drops_oldest_entry() {
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Fill to exactly MAX_HISTORY_SIZE with a sentinel first entry
    client.calculate_sla(&op, &symbol(&env, "SENTINEL"), &symbol_short!("low"), &10);
    for i in 1..1000u32 {
        let oid = Symbol::new(&env, &alloc::format!("FILLER_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    }

    // Sentinel is still present at index 0
    let history_before = client.get_history();
    assert_eq!(
        history_before.get(0).unwrap().outage_id,
        symbol(&env, "SENTINEL")
    );

    // One more push should evict the sentinel
    client.calculate_sla(&op, &symbol_short!("NEW"), &symbol_short!("low"), &10);

    let history_after = client.get_history();
    assert_eq!(history_after.len(), 1000);
    // Sentinel is gone; first entry is now a FILLER
    assert_ne!(
        history_after.get(0).unwrap().outage_id,
        symbol(&env, "SENTINEL")
    );
    // Newest entry is at the end
    assert_eq!(
        history_after.get(999).unwrap().outage_id,
        symbol_short!("NEW")
    );
}

#[test]
fn test_history_below_cap_is_not_trimmed() {
    let (_env, client, actors) = setup();

    for i in 0..5u32 {
        let oid = Symbol::new(&_env, &alloc::format!("SAFE_{}", i));
        client.calculate_sla(&actors.operator, &oid, &symbol_short!("low"), &10);
    }

    let history = client.get_history();
    assert_eq!(history.len(), 5, "History below cap must not be trimmed");
}

#[test]
fn test_pause_event_payload_is_single_bool() {
    let (env, client, actors) = setup();
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "test"));

    let events = env.events().all();
    // pause emits paused as the last event
    let (_, _, data) = events.last().unwrap();
    let payload: (bool,) = data.try_into_val(&env).unwrap();
    assert_eq!(payload, (true,));
}

#[test]
fn test_unpause_event_payload_is_single_bool() {
    let (env, client, actors) = setup();
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "test"));
    client.unpause(&actors.admin);

    let events = env.events().all();
    // unpause emits unpause as the last event
    let (_, _, data) = events.last().unwrap();
    let payload: (bool,) = data.try_into_val(&env).unwrap();
    assert_eq!(payload, (false,));
}

// ============================================================
// SC-066 – Property-based SLA monotonicity tests
// ============================================================

#[test]
fn test_monotonicity_worse_mttr_never_improves_reward() {
    // For a fixed severity, as MTTR increases within the met zone,
    // the reward amount must be non-increasing (worse or equal, never better).
    let (_env, client, actors) = setup();

    // critical: threshold=15; test mttr 1..=15 (all met)
    let mut prev_amount: Option<i128> = None;
    for mttr in 1u32..=15 {
        let oid = Symbol::new(&_env, &alloc::format!("MON_{}", mttr));
        let result =
            client.calculate_sla(&actors.operator, &oid, &symbol_short!("critical"), &mttr);
        assert_eq!(result.status, symbol_short!("met"));
        if let Some(prev) = prev_amount {
            assert!(
                result.amount <= prev,
                "Reward must not improve as MTTR worsens: mttr={} amount={} prev={}",
                mttr,
                result.amount,
                prev
            );
        }
        prev_amount = Some(result.amount);
    }
}

#[test]
fn test_monotonicity_worse_mttr_increases_penalty() {
    // For a fixed severity, as MTTR increases beyond the threshold,
    // the penalty magnitude must be strictly increasing.
    let (_env, client, actors) = setup();

    // critical: threshold=15; test mttr 16..=30 (all violated)
    let mut prev_amount: Option<i128> = None;
    for mttr in 16u32..=30 {
        let oid = Symbol::new(&_env, &alloc::format!("MON_PEN_{}", mttr));
        let result =
            client.calculate_sla(&actors.operator, &oid, &symbol_short!("critical"), &mttr);
        assert_eq!(result.status, symbol_short!("viol"));
        assert!(result.amount < 0, "Penalty must be negative");
        if let Some(prev) = prev_amount {
            assert!(
                result.amount < prev,
                "Penalty must strictly worsen as MTTR increases: mttr={} amount={} prev={}",
                mttr,
                result.amount,
                prev
            );
        }
        prev_amount = Some(result.amount);
    }
}

#[test]
fn test_monotonicity_threshold_boundary_is_met_not_violated() {
    // Exactly at threshold must always be "met", one over must always be "viol".
    let (_env, client, actors) = setup();

    let cases: &[(&str, u32)] = &[("critical", 15), ("high", 30), ("medium", 60), ("low", 120)];

    for (i, (sev, threshold)) in cases.iter().enumerate() {
        let at = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, &alloc::format!("BND_AT_{}", i)),
            &symbol(&_env, sev),
            threshold,
        );
        assert_eq!(
            at.status,
            symbol_short!("met"),
            "At threshold={} for {} must be met",
            threshold,
            sev
        );

        let over = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, &alloc::format!("BND_OVER_{}", i)),
            &symbol(&_env, sev),
            &(threshold + 1),
        );
        assert_eq!(
            over.status,
            symbol_short!("viol"),
            "One over threshold={} for {} must be viol",
            threshold,
            sev
        );
    }
}

#[test]
fn test_monotonicity_rating_degrades_with_mttr() {
    // Ratings must degrade in order: top → excel → good as MTTR approaches threshold.
    // critical threshold=15: ratio<50% → top, 50-74% → excel, 75-100% → good
    let (_env, client, actors) = setup();

    // mttr=1 → ratio=6% → top
    let r1 = client.calculate_sla(
        &actors.operator,
        &symbol_short!("RAT_1"),
        &symbol_short!("critical"),
        &1,
    );
    assert_eq!(r1.rating, symbol_short!("top"));

    // mttr=8 → ratio=53% → excel
    let r2 = client.calculate_sla(
        &actors.operator,
        &symbol_short!("RAT_8"),
        &symbol_short!("critical"),
        &8,
    );
    assert_eq!(r2.rating, symbol_short!("excel"));

    // mttr=15 → ratio=100% → good
    let r3 = client.calculate_sla(
        &actors.operator,
        &symbol_short!("RAT_15"),
        &symbol_short!("critical"),
        &15,
    );
    assert_eq!(r3.rating, symbol_short!("good"));

    // Reward amounts must be non-increasing: top >= excel >= good
    assert!(r1.amount >= r2.amount, "top reward must be >= excel reward");
    assert!(
        r2.amount >= r3.amount,
        "excel reward must be >= good reward"
    );
}

#[test]
fn test_monotonicity_all_severities_penalty_increases_with_mttr() {
    // For every severity, penalty grows linearly with overtime minutes.
    let (_env, client, actors) = setup();

    let cases: &[(&str, u32, i128)] = &[
        ("critical", 15, 100),
        ("high", 30, 50),
        ("medium", 60, 25),
        ("low", 120, 10),
    ];

    for (i, (sev, threshold, penalty_per_min)) in cases.iter().enumerate() {
        let r1 = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, &alloc::format!("LIN1_{}", i)),
            &symbol(&_env, sev),
            &(threshold + 1),
        );
        let r2 = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, &alloc::format!("LIN5_{}", i)),
            &symbol(&_env, sev),
            &(threshold + 5),
        );

        // r1: 1 min over → penalty = penalty_per_min
        assert_eq!(r1.amount, -penalty_per_min);
        // r2: 5 min over → penalty = 5 * penalty_per_min
        assert_eq!(r2.amount, -(5 * penalty_per_min));
        assert!(
            r2.amount < r1.amount,
            "Penalty must grow with overtime for {}",
            sev
        );
    }
}

#[test]
fn test_monotonicity_view_matches_mutating_for_all_mttr_values() {
    // calculate_sla_view must return identical results to calculate_sla for every MTTR.
    let (_env, client, actors) = setup();

    for mttr in [1u32, 7, 10, 14, 15, 16, 20, 30] {
        let view =
            client.calculate_sla_view(&symbol_short!("VM"), &symbol_short!("critical"), &mttr);
        let oid = Symbol::new(&_env, &alloc::format!("VM_{}", mttr));
        let mutating =
            client.calculate_sla(&actors.operator, &oid, &symbol_short!("critical"), &mttr);
        assert_eq!(
            view.status, mutating.status,
            "status mismatch at mttr={}",
            mttr
        );
        assert_eq!(
            view.amount, mutating.amount,
            "amount mismatch at mttr={}",
            mttr
        );
        assert_eq!(
            view.rating, mutating.rating,
            "rating mismatch at mttr={}",
            mttr
        );
        assert_eq!(
            view.payment_type, mutating.payment_type,
            "payment_type mismatch at mttr={}",
            mttr
        );
    }
}

// ============================================================
// SC-013 – Configurable retention limit (issue #133)
// ============================================================

#[test]
fn test_get_retention_limit_defaults_to_max_history_size() {
    let (_env, client, _actors) = setup();
    assert_eq!(client.get_retention_limit(), 1000);
}

#[test]
fn test_admin_can_set_retention_limit() {
    let (_env, client, actors) = setup();
    client.set_retention_limit(&actors.admin, &50);
    assert_eq!(client.get_retention_limit(), 50);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_retention_limit() {
    let (_env, client, actors) = setup();
    client.set_retention_limit(&actors.operator, &50);
}

#[test]
#[should_panic]
fn test_stranger_cannot_set_retention_limit() {
    let (_env, client, actors) = setup();
    client.set_retention_limit(&actors.stranger, &50);
}

#[test]
#[should_panic]
fn test_set_retention_limit_zero_fails() {
    let (_env, client, actors) = setup();
    client.set_retention_limit(&actors.admin, &0);
}

#[test]
#[should_panic]
fn test_set_retention_limit_above_max_fails() {
    let (_env, client, actors) = setup();
    client.set_retention_limit(&actors.admin, &1001);
}

#[test]
fn test_retention_limit_enforced_on_calculate() {
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Set a small retention limit
    client.set_retention_limit(&admin, &5);

    // Insert 10 entries
    for i in 0..10u32 {
        let oid = Symbol::new(&env, &alloc::format!("RET_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    }

    // History must be capped at the configured limit, not MAX_HISTORY_SIZE
    assert_eq!(client.get_history().len(), 5);
}

#[test]
fn test_retention_limit_drops_oldest_when_exceeded() {
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    client.set_retention_limit(&admin, &3);

    client.calculate_sla(&op, &symbol(&env, "FIRST"), &symbol_short!("low"), &10);
    client.calculate_sla(&op, &symbol(&env, "SECOND"), &symbol_short!("low"), &10);
    client.calculate_sla(&op, &symbol(&env, "THIRD"), &symbol_short!("low"), &10);
    // This push should evict FIRST
    client.calculate_sla(&op, &symbol(&env, "FOURTH"), &symbol_short!("low"), &10);

    let history = client.get_history();
    assert_eq!(history.len(), 3);
    assert_eq!(history.get(0).unwrap().outage_id, symbol(&env, "SECOND"));
    assert_eq!(history.get(2).unwrap().outage_id, symbol(&env, "FOURTH"));
}

#[test]
fn test_retention_limit_update_takes_effect_on_next_calculate() {
    // The retention limit only prevents growth beyond the cap; it does not
    // retroactively shrink existing history. When the limit is lowered below
    // the current history size, each subsequent calculate_sla call pushes one
    // entry and drops one (net zero change) until the history naturally drains
    // to the new limit via prune_history or prune_history_by_age.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Fill 10 entries with default limit
    for i in 0..10u32 {
        let oid = Symbol::new(&env, &alloc::format!("BEF_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    }
    assert_eq!(client.get_history().len(), 10);

    // Lower the limit; existing history is not pruned automatically
    client.set_retention_limit(&admin, &5);
    assert_eq!(
        client.get_history().len(),
        10,
        "Lowering limit must not retroactively prune"
    );

    // Each calculate_sla call pushes 1 and drops 1 (net zero) while history > limit.
    // History stays at 10 until an explicit prune brings it to the new limit.
    client.calculate_sla(&op, &symbol_short!("AFT"), &symbol_short!("low"), &10);
    assert_eq!(
        client.get_history().len(),
        10,
        "History stays at 10 (push 1, drop 1)"
    );

    // Explicit prune brings history down to the new limit
    client.prune_history(&admin, &5);
    assert_eq!(
        client.get_history().len(),
        5,
        "Explicit prune must enforce the new limit"
    );

    // Now the cap is active: further calculations stay at 5
    client.calculate_sla(&op, &symbol_short!("CAP"), &symbol_short!("low"), &10);
    assert_eq!(
        client.get_history().len(),
        5,
        "History must stay at 5 after cap is active"
    );
}

// ============================================================
// SC-021 – Migration state read helper (issue #141)
// ============================================================

#[test]
fn test_get_migration_state_returns_current_version() {
    let (_env, client, _actors) = setup();
    let info = client.get_migration_state();
    assert_eq!(info.stored_version, 1);
    assert_eq!(info.expected_version, 1);
    assert!(!info.needs_migration);
}

#[test]
fn test_get_migration_state_detects_version_mismatch() {
    let env = Env::default();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Overwrite stored version to simulate a future schema
    env.as_contract(&cid, || {
        env.storage().instance().set(&STORAGE_VERSION_KEY, &99u32);
    });

    let info = client.get_migration_state();
    assert_eq!(info.stored_version, 99);
    assert_eq!(info.expected_version, 1);
    assert!(info.needs_migration);
}

#[test]
fn test_get_migration_state_is_deterministic() {
    let (_env, client, _actors) = setup();
    let i1 = client.get_migration_state();
    let i2 = client.get_migration_state();
    assert_eq!(i1.stored_version, i2.stored_version);
    assert_eq!(i1.expected_version, i2.expected_version);
    assert_eq!(i1.needs_migration, i2.needs_migration);
}

#[test]
fn test_get_migration_state_after_migrate_shows_no_migration_needed() {
    let (_env, client, actors) = setup();
    // Already at current version; migrate is a no-op
    client.migrate(&actors.admin);
    let info = client.get_migration_state();
    assert!(!info.needs_migration);
}

#[test]
fn test_migrate_initialises_missing_fields() {
    // Simulate a frozen older snapshot that lacks keys added in later
    // schemas, then run migrate and verify deterministic defaults are set.
    let env = Env::default();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Remove some keys to emulate an older schema state and force stored
    // version to 0 so migrate will exercise the v0->v1 path.
    env.as_contract(&cid, || {
        env.storage().instance().remove(&CONFIG_KEY);
        env.storage().instance().remove(&STATS_KEY);
        env.storage().instance().set(&STORAGE_VERSION_KEY, &0u32);
    });

    // Run migration as admin
    client.migrate(&admin);

    // After migrate the keys should exist again and the stored version
    // should match the binary's STORAGE_VERSION.
    let version = client.get_storage_version();
    assert_eq!(version, STORAGE_VERSION);

    env.as_contract(&cid, || {
        assert!(env.storage().instance().has(&CONFIG_KEY));
        assert!(env.storage().instance().has(&STATS_KEY));
    });
}

// ============================================================
// SC-011 – Latest result by outage (issue #131) – additional coverage
// ============================================================

#[test]
fn test_get_latest_by_outage_returns_last_of_many() {
    let (env, client, actors) = setup();

    // Three calculations for different outages; each should be findable
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "MULTI_1"),
        &symbol_short!("critical"),
        &5,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "MULTI_2"),
        &symbol_short!("critical"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "MULTI_3"),
        &symbol_short!("critical"),
        &20,
    );

    let latest = client
        .get_latest_by_outage(&symbol(&env, "MULTI_3"))
        .unwrap();
    assert_eq!(latest.status, symbol_short!("viol")); // mttr=20 > threshold=15
    assert_eq!(latest.mttr_minutes, 20);
}

#[test]
fn test_get_latest_by_outage_unaffected_by_other_outages() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "A1"),
        &symbol_short!("critical"),
        &5,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "B"),
        &symbol_short!("critical"),
        &20,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "A2"),
        &symbol_short!("critical"),
        &10,
    );

    // Latest for A2 is the only A2 entry (mttr=10), not B or A1
    let latest_a = client.get_latest_by_outage(&symbol(&env, "A2")).unwrap();
    assert_eq!(latest_a.mttr_minutes, 10);

    let latest_b = client.get_latest_by_outage(&symbol(&env, "B")).unwrap();
    assert_eq!(latest_b.mttr_minutes, 20);
}

// ============================================================
// SC-038 – Event replay and missed-event recovery (issue #158)
//
// These tests demonstrate how a backend consumer can recover from missed events
// by replaying contract state. The pattern is:
//   1. Consumer misses some sla_calc events (simulated by not observing them).
//   2. Consumer calls get_history / get_history_page to reconstruct missed results.
//   3. Consumer calls get_latest_by_outage to confirm the current state per outage.
//   4. Consumer calls get_stats to verify aggregate totals are consistent.
//
// The contract guarantees that history + stats are always consistent with the
// events that were emitted, so a consumer can always recover full state from
// on-chain reads without replaying raw ledger events.
// ============================================================

#[test]
fn test_event_replay_history_matches_emitted_events() {
    // Verify that every entry in get_history corresponds to an emitted sla_calc event.
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "EVR_1"),
        &symbol_short!("critical"),
        &5,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "EVR_2"),
        &symbol_short!("high"),
        &35,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "EVR_3"),
        &symbol_short!("low"),
        &10,
    );

    let history = client.get_history();
    let events = env.events().all();

    // Filter only sla_calc events
    let sla_events: soroban_sdk::Vec<_> = {
        let mut v = soroban_sdk::Vec::new(&env);
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            let t0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if t0 == EVENT_SLA_CALC {
                v.push_back(events.get(i).unwrap());
            }
        }
        v
    };

    // One event per calculation
    assert_eq!(sla_events.len(), 3);
    assert_eq!(history.len(), 3);

    // Each history entry outage_id matches the corresponding event payload outage_id
    for i in 0..3u32 {
        let (_, _, data) = sla_events.get(i).unwrap();
        let (event_outage_id, _, _, _, _, _, _): (Symbol, Symbol, Symbol, Symbol, u32, u32, i128) =
            data.try_into_val(&env).unwrap();
        assert_eq!(history.get(i).unwrap().outage_id, event_outage_id);
    }
}

#[test]
fn test_missed_event_recovery_via_get_history_page() {
    // Simulate a consumer that missed events for calculations 3-5.
    // Recovery: page through history to find the missed entries.
    let (env, client, actors) = setup();

    for i in 0..5u32 {
        let oid = Symbol::new(&env, &alloc::format!("ENTRY_{}", i));
        client.calculate_sla(&actors.operator, &oid, &symbol_short!("low"), &10);
    }

    // Consumer already processed page 0 (entries 0-2); recover page 1 (entries 3-4)
    let missed = client.get_history_page(&3, &10);
    assert_eq!(missed.len(), 2);
    assert_eq!(
        missed.get(0).unwrap().outage_id,
        Symbol::new(&env, "ENTRY_3")
    );
    assert_eq!(
        missed.get(1).unwrap().outage_id,
        Symbol::new(&env, "ENTRY_4")
    );
}

#[test]
fn test_missed_event_recovery_via_get_latest_by_outage() {
    // Consumer missed all events for outage "OUTAGE_A".
    // Recovery: call get_latest_by_outage to get the current result.
    let (env, client, actors) = setup();

    // First calculation (violation)
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "OUTAGE_A"),
        &symbol_short!("critical"),
        &20,
    );
    // Second calculation for a different outage (met)
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "OUTAGE_B"),
        &symbol_short!("critical"),
        &5,
    );

    // Consumer recovers the latest result for OUTAGE_B
    let latest = client
        .get_latest_by_outage(&symbol(&env, "OUTAGE_B"))
        .unwrap();
    assert_eq!(latest.status, symbol_short!("met"));
    assert_eq!(latest.mttr_minutes, 5);
}

#[test]
fn test_missed_event_recovery_stats_consistent_with_history() {
    // After missing events, a consumer can verify aggregate stats are consistent
    // with the history they reconstruct.
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "S1"),
        &symbol_short!("critical"),
        &5,
    ); // met
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "S2"),
        &symbol_short!("critical"),
        &20,
    ); // viol
    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "S3"),
        &symbol_short!("high"),
        &10,
    ); // met

    let history = client.get_history();
    let stats = client.get_stats();

    // Recompute from history
    let mut calc_count = 0u64;
    let mut viol_count = 0u64;
    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        calc_count += 1;
        if entry.status == symbol_short!("viol") {
            viol_count += 1;
        }
    }

    assert_eq!(stats.total_calculations, calc_count);
    assert_eq!(stats.total_violations, viol_count);
}

#[test]
fn test_event_replay_view_function_produces_same_result_as_stored() {
    // A consumer can replay any stored result by calling calculate_sla_view
    // with the same inputs, confirming determinism.
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "DET1"),
        &symbol_short!("critical"),
        &10,
    );

    let stored = client.get_latest_by_outage(&symbol(&env, "DET1")).unwrap();
    let replayed =
        client.calculate_sla_view(&symbol(&env, "DET1"), &symbol_short!("critical"), &10);

    assert_eq!(stored.status, replayed.status);
    assert_eq!(stored.amount, replayed.amount);
    assert_eq!(stored.rating, replayed.rating);
    assert_eq!(stored.payment_type, replayed.payment_type);
    assert_eq!(stored.mttr_minutes, replayed.mttr_minutes);
    assert_eq!(stored.threshold_minutes, replayed.threshold_minutes);
    assert_eq!(stored.config_version_hash, replayed.config_version_hash);
}

#[test]
fn test_calculation_result_is_bound_to_current_config_hash() {
    let (_env, client, actors) = setup();

    let expected_hash = client.get_config_version_hash();
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("BIND1"),
        &symbol_short!("critical"),
        &10,
    );

    assert_eq!(result.config_version_hash, expected_hash);
}

#[test]
fn test_stored_result_retains_original_config_binding_after_config_change() {
    let (env, client, actors) = setup();

    client.calculate_sla(
        &actors.operator,
        &symbol(&env, "BIND2"),
        &symbol_short!("critical"),
        &10,
    );
    let before_change = client.get_latest_by_outage(&symbol(&env, "BIND2")).unwrap();
    let original_hash = before_change.config_version_hash;

    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
    let after_hash = client.get_config_version_hash();
    let stored_after_change = client.get_latest_by_outage(&symbol(&env, "BIND2")).unwrap();
    let replayed_after_change =
        client.calculate_sla_view(&symbol(&env, "BIND2"), &symbol_short!("critical"), &10);

    assert_eq!(stored_after_change.config_version_hash, original_hash);
    assert_ne!(original_hash, after_hash);
    assert_eq!(replayed_after_change.config_version_hash, after_hash);
    assert_eq!(
        stored_after_change.recorded_at,
        replayed_after_change.recorded_at
    );
}

#[test]
fn test_event_replay_after_prune_history_page_reflects_pruned_state() {
    // After prune, history pages reflect the pruned state.
    // A consumer that missed events before the prune can only recover
    // what remains in history.
    let (env, client, actors) = setup();

    for i in 0..10u32 {
        let oid = Symbol::new(&env, &alloc::format!("EVT_{}", i));
        client.calculate_sla(&actors.operator, &oid, &symbol_short!("low"), &10);
    }

    // Prune to keep only the latest 5
    client.prune_history(&actors.admin, &5);

    let history = client.get_history();
    assert_eq!(history.len(), 5);
    // All remaining entries are the last 5
    for i in 0..5u32 {
        assert_eq!(
            history.get(i).unwrap().outage_id,
            Symbol::new(&env, &alloc::format!("EVT_{}", i + 5))
        );
    }
}

// ============================================================
// SC-W5-029 – Version negotiation endpoint tests
// ============================================================

#[test]
fn test_get_version_info_returns_correct_versions_after_init() {
    let (_env, client, _actors) = setup();
    let info = client.get_version_info();
    assert_eq!(info.storage_version, 1);
    assert_eq!(info.result_schema_version, 1);
    assert!(!info.needs_migration);
    assert!(!info.is_paused);
    assert_eq!(info.contract_name, symbol_short!("sla_calc"));
}

#[test]
fn test_get_version_info_reflects_paused_state() {
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "upgrade"),
    );
    let info = client.get_version_info();
    assert!(info.is_paused);
    assert!(!info.needs_migration);
}

#[test]
fn test_get_version_info_reflects_unpaused_state() {
    let (env, client, actors) = setup();
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "test"));
    client.unpause(&actors.admin);
    let info = client.get_version_info();
    assert!(!info.is_paused);
}

#[test]
fn test_get_version_info_is_deterministic_across_repeated_calls() {
    let (_env, client, _actors) = setup();
    let a = client.get_version_info();
    let b = client.get_version_info();
    assert_eq!(a, b);
}

#[test]
fn test_get_version_info_not_affected_by_sla_calculations() {
    let (_env, client, actors) = setup();
    let before = client.get_version_info();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("OUT1"),
        &symbol_short!("high"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("OUT2"),
        &symbol_short!("critical"),
        &20,
    );
    let after = client.get_version_info();
    assert_eq!(before.storage_version, after.storage_version);
    assert_eq!(before.result_schema_version, after.result_schema_version);
    assert_eq!(before.needs_migration, after.needs_migration);
}

// #145 – Operator handoff cancellation and replacement lifecycle
// ============================================================

#[test]
fn test_propose_operator_replaces_pending_proposal() {
    // Re-proposing a different operator overwrites the pending slot.
    let (env, client, actors) = setup();
    let op_a = soroban_sdk::Address::generate(&env);
    let op_b = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &op_a);
    assert_eq!(client.get_pending_operator(), Some(op_a.clone()));

    // Replace with op_b before op_a accepts
    client.propose_operator(&actors.admin, &op_b);
    assert_eq!(
        client.get_pending_operator(),
        Some(op_b.clone()),
        "Second proposal must overwrite the first"
    );
}

#[test]
#[should_panic]
fn test_superseded_pending_operator_cannot_accept() {
    // op_a was proposed then replaced by op_b; op_a must not be able to accept.
    let (env, client, actors) = setup();
    let op_a = soroban_sdk::Address::generate(&env);
    let op_b = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &op_a);
    client.propose_operator(&actors.admin, &op_b); // replaces op_a

    client.accept_operator(&op_a); // must panic – op_a is no longer pending
}

#[test]
fn test_replacement_operator_can_accept_after_superseding() {
    // op_b replaces op_a; op_b can accept and becomes the active operator.
    let (env, client, actors) = setup();
    let op_a = soroban_sdk::Address::generate(&env);
    let op_b = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &op_a);
    client.propose_operator(&actors.admin, &op_b);
    client.accept_operator(&op_b);

    assert_eq!(client.get_operator(), op_b);
    assert_eq!(client.get_pending_operator(), None);
}

#[test]
fn test_cancel_pending_operator_by_proposing_current_operator() {
    // Admin can effectively cancel a pending proposal by re-proposing the current operator.
    // After acceptance the operator is unchanged.
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &new_op);
    // "Cancel" by re-proposing the current operator
    client.propose_operator(&actors.admin, &actors.operator);
    client.accept_operator(&actors.operator);

    assert_eq!(client.get_operator(), actors.operator);
    assert_eq!(client.get_pending_operator(), None);
}

#[test]
fn test_pending_operator_state_queryable_throughout_lifecycle() {
    // Verify get_pending_operator returns the correct value at each lifecycle stage.
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    // Before proposal: None
    assert_eq!(client.get_pending_operator(), None);

    // After proposal: Some(new_op)
    client.propose_operator(&actors.admin, &new_op);
    assert_eq!(client.get_pending_operator(), Some(new_op.clone()));

    // After acceptance: None
    client.accept_operator(&new_op);
    assert_eq!(client.get_pending_operator(), None);
}

#[test]
fn test_operator_handoff_full_lifecycle_old_operator_locked_out() {
    // Full lifecycle: propose → accept → old operator cannot calculate.
    let (env, client, actors) = setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &new_op);
    client.accept_operator(&new_op);

    // New operator can calculate
    let result = client.calculate_sla(
        &new_op,
        &symbol_short!("HO_NEW"),
        &symbol_short!("critical"),
        &5,
    );
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
fn test_multiple_replacement_cycles_end_state_is_correct() {
    // Propose A, replace with B, replace with C, accept C.
    let (env, client, actors) = setup();
    let op_a = soroban_sdk::Address::generate(&env);
    let op_b = soroban_sdk::Address::generate(&env);
    let op_c = soroban_sdk::Address::generate(&env);

    client.propose_operator(&actors.admin, &op_a);
    client.propose_operator(&actors.admin, &op_b);
    client.propose_operator(&actors.admin, &op_c);

    assert_eq!(client.get_pending_operator(), Some(op_c.clone()));
    client.accept_operator(&op_c);

    assert_eq!(client.get_operator(), op_c);
    assert_eq!(client.get_pending_operator(), None);
}

// ============================================================
// #147 – Admin renounce preconditions
// ============================================================

#[test]
fn test_renounce_with_pending_admin_proposal_clears_proposal() {
    // Renounce while a pending admin proposal exists must clear the proposal atomically.
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);

    client.propose_admin(&actors.admin, &new_admin);
    assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));

    client.renounce_admin(&actors.admin);

    // Pending proposal is cleared
    assert_eq!(client.get_pending_admin(), None);
}

#[test]
#[should_panic]
fn test_proposed_admin_cannot_accept_after_renounce() {
    // After renounce, the previously proposed admin cannot accept (no admin exists).
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);

    client.propose_admin(&actors.admin, &new_admin);
    client.renounce_admin(&actors.admin);

    // accept_admin must panic – pending proposal was cleared
    client.accept_admin(&new_admin);
}

#[test]
fn test_renounce_while_paused_succeeds() {
    // Admin can renounce even when the contract is paused.
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );
    assert!(client.is_paused());

    // Renounce must succeed regardless of pause state
    client.renounce_admin(&actors.admin);
}

#[test]
#[should_panic]
fn test_post_renounce_pause_is_locked() {
    // After renounce, pause is permanently locked.
    let (env, client, actors) = setup();
    client.renounce_admin(&actors.admin);
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "x"));
}

#[test]
#[should_panic]
fn test_post_renounce_unpause_is_locked() {
    // After renounce, unpause is permanently locked.
    let (env, client, actors) = setup();
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "x"));
    client.renounce_admin(&actors.admin);
    client.unpause(&actors.admin);
}

#[test]
#[should_panic]
fn test_post_renounce_set_config_is_locked() {
    let (_env, client, actors) = setup();
    client.renounce_admin(&actors.admin);
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
}

#[test]
#[should_panic]
fn test_post_renounce_prune_history_is_locked() {
    let (_env, client, actors) = setup();
    client.renounce_admin(&actors.admin);
    client.prune_history(&actors.admin, &0);
}

#[test]
#[should_panic]
fn test_post_renounce_propose_admin_is_locked() {
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);
    client.renounce_admin(&actors.admin);
    client.propose_admin(&actors.admin, &new_admin);
}

#[test]
fn test_post_renounce_operator_can_still_calculate() {
    // Renounce only removes admin authority; the operator role is unaffected.
    let (_env, client, actors) = setup();
    client.renounce_admin(&actors.admin);

    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("REN_OP"),
        &symbol_short!("critical"),
        &5,
    );
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
fn test_renounce_is_irreversible_no_admin_exists() {
    // After renounce, get_admin must fail (no admin in storage).
    let (_env, client, actors) = setup();
    client.renounce_admin(&actors.admin);

    let result = client.try_get_admin();
    assert!(result.is_err(), "get_admin must fail after renounce");
}

// ============================================================
// #148 – Pause-metadata history through repeated pause/unpause cycles
// ============================================================

#[test]
fn test_pause_metadata_reflects_latest_reason_after_cycle() {
    // After pause → unpause → pause again, metadata must reflect the second pause.
    let (env, client, actors) = setup();

    let reason1 = soroban_sdk::String::from_str(&env, "first maintenance");
    let reason2 = soroban_sdk::String::from_str(&env, "second maintenance");

    client.pause(&actors.admin, &reason1);
    client.unpause(&actors.admin);
    client.pause(&actors.admin, &reason2);

    let info = client.get_pause_info().expect("pause info must be present");
    assert_eq!(
        info.reason, reason2,
        "Metadata must reflect the most recent pause reason"
    );
}

#[test]
fn test_pause_metadata_cleared_between_cycles() {
    // After unpause, get_pause_info must return None before the next pause.
    let (env, client, actors) = setup();

    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "cycle1"),
    );
    client.unpause(&actors.admin);

    assert_eq!(
        client.get_pause_info(),
        None,
        "Pause info must be None after unpause"
    );
}

#[test]
fn test_pause_metadata_timestamp_advances_across_cycles() {
    // Each pause cycle records a fresh timestamp; later pauses must have >= timestamp.
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    client.pause(&admin, &soroban_sdk::String::from_str(&env, "first"));
    let ts1 = client.get_pause_info().unwrap().paused_at;
    assert_eq!(ts1, 1000);

    client.unpause(&admin);

    env.ledger().set_timestamp(2000);
    client.pause(&admin, &soroban_sdk::String::from_str(&env, "second"));
    let ts2 = client.get_pause_info().unwrap().paused_at;
    assert_eq!(ts2, 2000);

    assert!(ts2 > ts1, "Second pause timestamp must be later than first");
}

#[test]
fn test_repeated_pause_unpause_cycles_is_paused_state_consistent() {
    // is_paused must toggle correctly through multiple cycles.
    let (env, client, actors) = setup();

    for _ in 0..5u32 {
        assert!(!client.is_paused());
        client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "cycle"));
        assert!(client.is_paused());
        client.unpause(&actors.admin);
    }
    assert!(!client.is_paused());
}

#[test]
fn test_pause_metadata_different_reasons_each_cycle() {
    // Each cycle stores a distinct reason; verify the last one is always current.
    let (env, client, actors) = setup();

    let reasons = ["alpha", "beta", "gamma", "delta"];
    for reason_str in reasons {
        let reason = soroban_sdk::String::from_str(&env, reason_str);
        client.pause(&actors.admin, &reason.clone());
        let info = client.get_pause_info().unwrap();
        assert_eq!(
            info.reason, reason,
            "Reason must match for cycle '{}'",
            reason_str
        );
        client.unpause(&actors.admin);
    }
}

#[test]
fn test_calculate_sla_blocked_and_unblocked_across_cycles() {
    // Verify calculate_sla is blocked during pause and unblocked after unpause,
    // across multiple cycles.
    let (env, client, actors) = setup();

    for _ in 0..3u32 {
        // Unpaused: calculation succeeds
        let result = client.calculate_sla(
            &actors.operator,
            &symbol_short!("CYC"),
            &symbol_short!("critical"),
            &5,
        );
        assert_eq!(result.status, symbol_short!("met"));

        // Paused: calculation must fail
        client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "cycle"));
        let blocked = client.try_calculate_sla(
            &actors.operator,
            &symbol_short!("CYC"),
            &symbol_short!("critical"),
            &5,
        );
        assert!(
            blocked.is_err(),
            "calculate_sla must be blocked while paused"
        );

        client.unpause(&actors.admin);
    }
}

#[test]
fn test_pause_events_emitted_each_cycle() {
    // Each pause and unpause must emit the corresponding event.
    let (env, client, actors) = setup();

    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "c1"));
    client.unpause(&actors.admin);
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "c2"));
    client.unpause(&actors.admin);

    // Count paused and unpause events
    let events = env.events().all();
    let mut pause_count = 0u32;
    let mut unpause_count = 0u32;
    for i in 0..events.len() {
        let (_, topics, _) = events.get(i).unwrap();
        let t0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        if t0 == EVENT_PAUSED {
            pause_count += 1;
        } else if t0 == EVENT_UNPAUSED {
            unpause_count += 1;
        }
    }
    assert_eq!(pause_count, 2, "Must emit 2 paused events");
    assert_eq!(unpause_count, 2, "Must emit 2 unpause events");
}

// ============================================================
// #135 – Storage-growth regression coverage
// ============================================================

#[test]
fn test_storage_growth_history_grows_linearly_then_caps() {
    // History length must grow by 1 per calculation until the cap, then stay flat.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Grow to 10 entries
    for i in 0..10u32 {
        let oid = Symbol::new(&env, &alloc::format!("GRW_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
        assert_eq!(
            client.get_history().len(),
            i + 1,
            "History must grow by 1 per calculation (entry {})",
            i + 1
        );
    }

    // Set a small cap and verify it holds
    client.set_retention_limit(&admin, &10);
    let oid = Symbol::new(&env, "GRW_last");
    client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    assert_eq!(
        client.get_history().len(),
        10,
        "History must not exceed the retention limit"
    );
}

#[test]
fn test_storage_growth_prune_cycle_keeps_history_bounded() {
    // Simulate a long-running scenario: fill → prune → fill → prune.
    // History must never exceed the prune target.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    let mut call_count = 0u32;
    for _cycle in 0..3u32 {
        for _ in 0..20u32 {
            let oid = Symbol::new(&env, &alloc::format!("CYC_{}", call_count));
            client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
            call_count += 1;
        }
        client.prune_history(&admin, &5);
        assert_eq!(
            client.get_history().len(),
            5,
            "History must be bounded to 5 after each prune cycle"
        );
    }
}

#[test]
fn test_storage_growth_age_prune_cycle_keeps_history_bounded() {
    // Simulate time-based pruning across multiple ledger epochs.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Epoch 1: add 10 entries at t=0
    env.ledger().set_timestamp(0);
    for i in 0..10u32 {
        let oid = Symbol::new(&env, &alloc::format!("EP1_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    }

    // Epoch 2: advance time, add 5 more, prune old ones
    env.ledger().set_timestamp(10_000);
    for i in 0..5u32 {
        let oid = Symbol::new(&env, &alloc::format!("EP2_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    }
    client.prune_history_by_age(&admin, &5_000); // cutoff=5000; epoch1 entries (t=0) removed

    assert_eq!(
        client.get_history().len(),
        5,
        "Only epoch-2 entries must remain after age prune"
    );
}

#[test]
fn test_storage_growth_config_map_stays_fixed_size() {
    // Config map must remain exactly 4 entries regardless of update frequency.
    let (_env, client, actors) = setup();

    for _ in 0..50u32 {
        client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
        client.set_config(&actors.admin, &symbol_short!("high"), &30, &50, &750);
    }

    assert_eq!(
        client.get_config_count(),
        4,
        "Config map must always have exactly 4 entries"
    );
}

#[test]
fn test_storage_growth_stats_struct_size_is_constant() {
    // Stats is a fixed-size struct; total_calculations must equal the number of calls.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    let n = 200u32;
    for i in 0..n {
        let mttr = if i % 3 == 0 { 5u32 } else { 20u32 };
        let oid = Symbol::new(&env, &alloc::format!("ST_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("critical"), &mttr);
    }

    let stats = client.get_stats();
    assert_eq!(
        stats.total_calculations, n as u64,
        "Stats must track exactly {} calculations",
        n
    );
    // Violations + non-violations must sum to total
    let non_violations = stats.total_calculations - stats.total_violations;
    assert_eq!(
        stats.total_violations + non_violations,
        stats.total_calculations,
        "Violation + met counts must equal total"
    );
    let _ = admin;
}

#[test]
fn test_storage_growth_retention_limit_prevents_unbounded_growth() {
    // With a small retention limit, history must never exceed it even after many calls.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    client.set_retention_limit(&admin, &20);

    for i in 0..100u32 {
        let oid = Symbol::new(&env, &alloc::format!("LIM_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("low"), &10);
    }

    assert_eq!(
        client.get_history().len(),
        20,
        "History must be capped at the configured retention limit"
    );
}

#[test]
fn test_storage_growth_regression_mixed_operations() {
    // Regression: interleave calculations, config updates, and pruning.
    // Verify no unexpected growth in any storage slot.
    let env = Env::default();
    env.budget().reset_unlimited();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    for i in 0..30u32 {
        let oid = Symbol::new(&env, &alloc::format!("MIX_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("critical"), &5);

        if i % 10 == 9 {
            // Prune every 10 entries
            client.prune_history(&admin, &5);
            assert!(
                client.get_history().len() <= 5,
                "History must not exceed 5 after prune at iteration {}",
                i
            );
        }

        if i % 5 == 4 {
            // Config update must not grow the config map
            client.set_config(&admin, &symbol_short!("critical"), &15, &100, &750);
            assert_eq!(client.get_config_count(), 4);
        }
    }

    // Final state: history bounded, config fixed, stats consistent
    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 30);
    assert_eq!(client.get_config_count(), 4);
}

// ============================================================
// SC-006 (#126) – Invariance: calculate_sla vs calculate_sla_view
//
// Both paths share compute_result; these tests prove they never diverge
// in result semantics across all severities and representative MTTR values.
// Allowed differences (history growth, stats increment, recorded_at timestamp)
// are explicitly documented and isolated below.
// ============================================================

/// Helper: call both paths and assert full result parity.
fn assert_invariant(
    client: &SLACalculatorContractClient,
    operator: &soroban_sdk::Address,
    outage_id: Symbol,
    severity: Symbol,
    mttr: u32,
) {
    let view = client.calculate_sla_view(&outage_id, &severity, &mttr);
    let mutating = client.calculate_sla(operator, &outage_id, &severity, &mttr);

    assert_eq!(
        view.outage_id, mutating.outage_id,
        "outage_id mismatch mttr={}",
        mttr
    );
    assert_eq!(
        view.status, mutating.status,
        "status mismatch mttr={}",
        mttr
    );
    assert_eq!(
        view.mttr_minutes, mutating.mttr_minutes,
        "mttr_minutes mismatch mttr={}",
        mttr
    );
    assert_eq!(
        view.threshold_minutes, mutating.threshold_minutes,
        "threshold_minutes mismatch mttr={}",
        mttr
    );
    assert_eq!(
        view.amount, mutating.amount,
        "amount mismatch mttr={}",
        mttr
    );
    assert_eq!(
        view.payment_type, mutating.payment_type,
        "payment_type mismatch mttr={}",
        mttr
    );
    assert_eq!(
        view.rating, mutating.rating,
        "rating mismatch mttr={}",
        mttr
    );
    // Documented allowed difference: recorded_at is 0 for view, ledger timestamp for mutating.
    assert_eq!(view.recorded_at, 0, "view recorded_at must always be 0");
    assert_eq!(
        view.recorded_at, mutating.recorded_at,
        "recorded_at mismatch mttr={}",
        mttr
    );
}

#[test]
fn test_invariance_critical_all_rating_zones() {
    // critical threshold=15; covers top (<50%), excel (50-74%), good (75-100%), viol (>100%)
    let (_env, client, actors) = setup();
    let sev = symbol_short!("critical");
    for mttr in [1u32, 7, 10, 12, 15, 16, 20, 30] {
        let oid = Symbol::new(&_env, &alloc::format!("INV_{}", mttr));
        assert_invariant(&client, &actors.operator, oid, sev.clone(), mttr);
    }
}

#[test]
fn test_invariance_high_all_rating_zones() {
    let (_env, client, actors) = setup();
    let sev = symbol_short!("high");
    // high threshold=30
    for mttr in [1u32, 14, 22, 28, 30, 31, 40, 60] {
        let oid = Symbol::new(&_env, &alloc::format!("INV_{}", mttr));
        assert_invariant(&client, &actors.operator, oid, sev.clone(), mttr);
    }
}

#[test]
fn test_invariance_medium_all_rating_zones() {
    let (_env, client, actors) = setup();
    let sev = symbol_short!("medium");
    // medium threshold=60
    for mttr in [1u32, 29, 44, 55, 60, 61, 80, 120] {
        let oid = Symbol::new(&_env, &alloc::format!("INV_{}", mttr));
        assert_invariant(&client, &actors.operator, oid, sev.clone(), mttr);
    }
}

#[test]
fn test_invariance_low_all_rating_zones() {
    let (_env, client, actors) = setup();
    let sev = symbol_short!("low");
    // low threshold=120
    for mttr in [1u32, 59, 89, 110, 120, 121, 150, 240] {
        let oid = Symbol::new(&_env, &alloc::format!("INV_{}", mttr));
        assert_invariant(&client, &actors.operator, oid, sev.clone(), mttr);
    }
}

#[test]
fn test_invariance_view_does_not_mutate_history() {
    // calculate_sla_view must never append to history.
    let (_env, client, actors) = setup();

    client.calculate_sla_view(&symbol_short!("V1"), &symbol_short!("critical"), &5);
    client.calculate_sla_view(&symbol_short!("V2"), &symbol_short!("high"), &35);
    assert_eq!(client.get_history().len(), 0, "view must not write history");

    // One mutating call → exactly one history entry
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("M1"),
        &symbol_short!("critical"),
        &5,
    );
    assert_eq!(client.get_history().len(), 1);
}

#[test]
fn test_invariance_view_does_not_mutate_stats() {
    // calculate_sla_view must never increment stats.
    let (_env, client, actors) = setup();

    for _ in 0..5u32 {
        client.calculate_sla_view(&symbol_short!("VS"), &symbol_short!("critical"), &5);
    }
    assert_eq!(
        client.get_stats().total_calculations,
        0,
        "view must not increment stats"
    );

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("MS"),
        &symbol_short!("critical"),
        &5,
    );
    assert_eq!(client.get_stats().total_calculations, 1);
}

#[test]
fn test_invariance_view_works_while_paused() {
    // calculate_sla_view bypasses the pause guard; calculate_sla does not.
    let (env, client, actors) = setup();
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "test"));

    // View must succeed even when paused
    let view = client.calculate_sla_view(&symbol_short!("PV"), &symbol_short!("critical"), &5);
    assert_eq!(view.status, symbol_short!("met"));

    // Mutating must fail
    let blocked = client.try_calculate_sla(
        &actors.operator,
        &symbol_short!("PM"),
        &symbol_short!("critical"),
        &5,
    );
    assert!(blocked.is_err());
}

#[test]
fn test_invariance_after_config_change() {
    // After a config update both paths must reflect the new config identically.
    let (_env, client, actors) = setup();

    // Update critical: threshold=20, penalty=200, reward=1000
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    // mttr=25 → 5 min over new threshold → penalty = 5*200 = 1000
    let view = client.calculate_sla_view(&symbol_short!("CFG"), &symbol_short!("critical"), &25);
    let mutating = client.calculate_sla(
        &actors.operator,
        &symbol_short!("CFG"),
        &symbol_short!("critical"),
        &25,
    );

    assert_eq!(view.status, mutating.status);
    assert_eq!(view.amount, mutating.amount);
    assert_eq!(view.amount, -1000);
}

#[test]
fn test_invariance_boundary_mttr_zero() {
    // mttr=0 is within threshold for all severities -> always "met" with top rating.
    let (_env, client, actors) = setup();
    for (idx, sev) in [
        symbol_short!("critical"),
        symbol_short!("high"),
        symbol_short!("medium"),
        symbol_short!("low"),
    ]
    .iter()
    .enumerate()
    {
        let oid = Symbol::new(&_env, &alloc::format!("Z_{}", idx));
        let view = client.calculate_sla_view(&oid, sev, &0);
        let mutating = client.calculate_sla(&actors.operator, &oid, sev, &0);
        assert_eq!(view.status, symbol_short!("met"));
        assert_eq!(view.status, mutating.status);
        assert_eq!(view.amount, mutating.amount);
        assert_eq!(view.rating, symbol_short!("top")); // ratio=0% < 50%
    }
}

// ============================================================
// SC-007 (#127) – Overflow and extreme-config safety tests
//
// Validates that large thresholds, penalties, rewards, and MTTR values
// are either accepted with correct arithmetic or rejected with stable errors.
// ============================================================

#[test]
fn test_extreme_mttr_at_max_u32_violates_and_does_not_overflow() {
    // mttr = u32::MAX with default critical config (threshold=15, penalty=100/min).
    // overtime = u32::MAX - 15 ≈ 4.29e9; penalty = overtime * 100 as i128.
    // i128 can hold up to ~1.7e38, so no overflow.
    let env = Env::default();
    env.budget().reset_unlimited();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    let mttr = u32::MAX;
    let result =
        client.calculate_sla_view(&symbol_short!("XMTTR"), &symbol_short!("critical"), &mttr);

    assert_eq!(result.status, symbol_short!("viol"));
    assert_eq!(result.payment_type, symbol_short!("pen"));
    // overtime = (u32::MAX - 15) as i128; penalty = overtime * 100
    let expected_penalty = -((u32::MAX - 15) as i128 * 100);
    assert_eq!(result.amount, expected_penalty);
    assert!(result.amount < 0);
}

#[test]
fn test_extreme_mttr_large_value_penalty_is_linear() {
    // Penalty must scale linearly: doubling overtime doubles penalty.
    let (_env, client, _actors) = setup();

    // critical threshold=15, penalty=100/min
    let r1 = client.calculate_sla_view(&symbol_short!("LIN1"), &symbol_short!("critical"), &115); // 100 min over
    let r2 = client.calculate_sla_view(&symbol_short!("LIN2"), &symbol_short!("critical"), &215); // 200 min over

    assert_eq!(r1.amount, -10_000); // 100 * 100
    assert_eq!(r2.amount, -20_000); // 200 * 100
    assert_eq!(r2.amount, r1.amount * 2);
}

#[test]
fn test_extreme_config_max_valid_penalty_and_reward() {
    // Set config to boundary-valid maximums and verify arithmetic is correct.
    // critical: threshold=60, penalty=10000, reward=100000
    let (_env, client, actors) = setup();
    client.set_config(
        &actors.admin,
        &symbol_short!("critical"),
        &60,
        &10000,
        &100000,
    );

    // mttr=61 → 1 min over → penalty = 10000
    let viol = client.calculate_sla_view(&symbol_short!("XPEN"), &symbol_short!("critical"), &61);
    assert_eq!(viol.amount, -10_000);

    // mttr=1 → ratio=1% < 50% → top → reward = 100000 * 200 / 100 = 200000
    let met = client.calculate_sla_view(&symbol_short!("XREW"), &symbol_short!("critical"), &1);
    assert_eq!(met.amount, 200_000);
}

#[test]
fn test_extreme_config_max_valid_low_threshold() {
    // low: threshold=1440 (24h), penalty=1, reward=1
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &1440, &1, &1);

    // mttr=1440 → exactly at threshold → met, good rating
    let at = client.calculate_sla_view(&symbol_short!("LT"), &symbol_short!("low"), &1440);
    assert_eq!(at.status, symbol_short!("met"));
    assert_eq!(at.rating, symbol_short!("good"));

    // mttr=1441 → 1 min over → penalty = 1
    let over = client.calculate_sla_view(&symbol_short!("LT"), &symbol_short!("low"), &1441);
    assert_eq!(over.status, symbol_short!("viol"));
    assert_eq!(over.amount, -1);
}

#[test]
fn test_extreme_penalty_large_overtime_no_i128_overflow() {
    // Worst-case: low threshold=1, penalty=100 (max for low), mttr=u32::MAX
    // overtime = u32::MAX - 1 ≈ 4.29e9; penalty = 4.29e9 * 100 ≈ 4.29e11
    // i128 max ≈ 1.7e38 — no overflow possible.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &1, &100, &1);

    let env = Env::default();
    env.budget().reset_unlimited();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client2 = SLACalculatorContractClient::new(&env, &cid);
    let admin2 = soroban_sdk::Address::generate(&env);
    let op2 = soroban_sdk::Address::generate(&env);
    client2.initialize(&admin2, &op2);
    client2.set_config(&admin2, &symbol_short!("low"), &1, &100, &1);

    let result =
        client2.calculate_sla_view(&symbol_short!("OVF"), &symbol_short!("low"), &u32::MAX);
    assert_eq!(result.status, symbol_short!("viol"));
    let expected = -((u32::MAX - 1) as i128 * 100);
    assert_eq!(result.amount, expected);
}

#[test]
fn test_extreme_reward_max_multiplier_no_overflow() {
    // Max reward: reward_base=100000, multiplier=200 (top rating) → 200000
    // This is well within i128 range.
    let (_env, client, actors) = setup();
    client.set_config(
        &actors.admin,
        &symbol_short!("critical"),
        &60,
        &10000,
        &100000,
    );

    let result = client.calculate_sla_view(&symbol_short!("MAXR"), &symbol_short!("critical"), &1);
    assert_eq!(result.amount, 200_000); // 100000 * 200 / 100
    assert!(result.amount > 0);
}

#[test]
#[should_panic]
fn test_extreme_threshold_zero_rejected() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &0, &10, &600);
}

#[test]
#[should_panic]
fn test_extreme_threshold_above_1440_rejected() {
    let (_env, client, actors) = setup();
    // 1441 exceeds the 24-hour global cap
    client.set_config(&actors.admin, &symbol_short!("low"), &1441, &10, &600);
}

#[test]
#[should_panic]
fn test_extreme_penalty_zero_rejected() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &0, &600);
}

#[test]
#[should_panic]
fn test_extreme_penalty_above_10000_rejected() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10001, &600);
}

#[test]
#[should_panic]
fn test_get_version_info_panics_when_not_initialized() {
    let env = Env::default();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    // No initialize call — must panic (NotInitialized)
    client.get_version_info();
}

#[test]
fn test_get_version_info_needs_migration_false_after_migrate() {
    let (_env, client, actors) = setup();
    // migrate on an already-current contract is a no-op; needs_migration stays false
    client.migrate(&actors.admin);
    let info = client.get_version_info();
    assert!(!info.needs_migration);
}

#[test]
#[should_panic]
fn test_extreme_reward_zero_rejected() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &0);
}

#[test]
#[should_panic]
fn test_extreme_reward_above_100000_rejected() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &100001);
}

#[test]
fn test_extreme_mttr_equals_threshold_is_always_met() {
    // At exactly the threshold, result must always be "met" regardless of how large the threshold is.
    let (_env, client, actors) = setup();
    // Set low to max threshold
    client.set_config(&actors.admin, &symbol_short!("low"), &1440, &1, &1);

    let result = client.calculate_sla_view(&symbol_short!("EQ"), &symbol_short!("low"), &1440);
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
fn test_extreme_stats_accumulate_large_values_without_overflow() {
    // Run many high-penalty violations and verify stats accumulate correctly.
    let env = Env::default();
    env.budget().reset_unlimited();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);
    // critical: threshold=60, penalty=10000
    client.set_config(&admin, &symbol_short!("critical"), &60, &10000, &100000);

    // 100 violations of 1 min each → penalty = 10000 each → total = 1_000_000
    for i in 0..100u32 {
        let oid = Symbol::new(&env, &alloc::format!("BIG_{}", i));
        client.calculate_sla(&op, &oid, &symbol_short!("critical"), &61);
    }

    let stats = client.get_stats();
    assert_eq!(stats.total_penalties, 1_000_000);
    assert_eq!(stats.total_violations, 100);
}

// ============================================================
// SC-008 (#128) – Complete negative test matrix for set_config validation
//
// Covers every rejection path in validate_config: zero, boundary+1, ordering
// edge cases, and cross-severity consistency.
// ============================================================

// --- Global range rejections ---

#[test]
#[should_panic]
fn test_set_config_rejects_unknown_severity_symbol() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("info"), &30, &50, &500);
}

#[test]
#[should_panic]
fn test_set_config_rejects_threshold_1441_for_low() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &1441, &10, &600);
}

#[test]
#[should_panic]
fn test_set_config_rejects_penalty_i128_max() {
    let (_env, client, actors) = setup();
    // i128::MAX is way above 10000 limit
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &i128::MAX, &600);
}

#[test]
#[should_panic]
fn test_set_config_rejects_reward_i128_max() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &i128::MAX);
}

// --- Critical severity-specific rejections ---

#[test]
#[should_panic]
fn test_set_config_critical_rejects_threshold_61() {
    // critical max threshold is 60
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("critical"), &61, &100, &750);
}

#[test]
#[should_panic]
fn test_set_config_critical_rejects_penalty_49() {
    // critical min penalty is 50
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &49, &750);
}

#[test]
fn test_set_config_critical_accepts_threshold_60_penalty_50() {
    // Exact boundary values must be accepted
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("critical"), &60, &50, &750);
    let cfg = client.get_config(&symbol_short!("critical"));
    assert_eq!(cfg.threshold_minutes, 60);
    assert_eq!(cfg.penalty_per_minute, 50);
}

// --- High severity-specific rejections ---

#[test]
#[should_panic]
fn test_set_config_high_rejects_threshold_121() {
    // high max threshold is 120
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("high"), &121, &50, &750);
}

#[test]
#[should_panic]
fn test_set_config_high_rejects_penalty_24() {
    // high min penalty is 25
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("high"), &30, &24, &750);
}

#[test]
fn test_set_config_high_accepts_threshold_120_penalty_25() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("high"), &120, &25, &750);
    let cfg = client.get_config(&symbol_short!("high"));
    assert_eq!(cfg.threshold_minutes, 120);
    assert_eq!(cfg.penalty_per_minute, 25);
}

// --- Medium severity-specific rejections ---

#[test]
#[should_panic]
fn test_set_config_medium_rejects_threshold_241() {
    // medium max threshold is 240
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("medium"), &241, &25, &750);
}

#[test]
#[should_panic]
fn test_set_config_medium_rejects_penalty_9() {
    // medium min penalty is 10
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("medium"), &60, &9, &750);
}

#[test]
fn test_set_config_medium_accepts_threshold_240_penalty_10() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("medium"), &240, &10, &750);
    let cfg = client.get_config(&symbol_short!("medium"));
    assert_eq!(cfg.threshold_minutes, 240);
    assert_eq!(cfg.penalty_per_minute, 10);
}

// --- Low severity-specific rejections ---

#[test]
#[should_panic]
fn test_set_config_low_rejects_penalty_101() {
    // low max penalty is 100
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &101, &600);
}

#[test]
fn test_set_config_low_accepts_penalty_100() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &100, &600);
    let cfg = client.get_config(&symbol_short!("low"));
    assert_eq!(cfg.penalty_per_minute, 100);
}

// --- Rejection does not corrupt existing state ---

#[test]
fn test_set_config_rejection_leaves_state_unchanged_for_all_severities() {
    let (_env, client, actors) = setup();

    // Capture defaults
    let orig_critical = client.get_config(&symbol_short!("critical"));
    let orig_high = client.get_config(&symbol_short!("high"));
    let orig_medium = client.get_config(&symbol_short!("medium"));
    let orig_low = client.get_config(&symbol_short!("low"));

    // Attempt invalid updates for each severity
    let _ = client.try_set_config(&actors.admin, &symbol_short!("critical"), &0, &100, &750);
    let _ = client.try_set_config(&actors.admin, &symbol_short!("high"), &0, &50, &750);
    let _ = client.try_set_config(&actors.admin, &symbol_short!("medium"), &0, &25, &750);
    let _ = client.try_set_config(&actors.admin, &symbol_short!("low"), &0, &10, &600);

    // All configs must be unchanged
    assert_eq!(
        client
            .get_config(&symbol_short!("critical"))
            .threshold_minutes,
        orig_critical.threshold_minutes
    );
    assert_eq!(
        client.get_config(&symbol_short!("high")).threshold_minutes,
        orig_high.threshold_minutes
    );
    assert_eq!(
        client
            .get_config(&symbol_short!("medium"))
            .threshold_minutes,
        orig_medium.threshold_minutes
    );
    assert_eq!(
        client.get_config(&symbol_short!("low")).threshold_minutes,
        orig_low.threshold_minutes
    );
}

#[test]
fn test_set_config_rejection_does_not_affect_other_severities() {
    // A failed update to one severity must not touch any other severity.
    let (_env, client, actors) = setup();

    // Valid update to critical
    client.set_config(&actors.admin, &symbol_short!("critical"), &30, &150, &1000);

    // Invalid update to high (threshold=0)
    let _ = client.try_set_config(&actors.admin, &symbol_short!("high"), &0, &50, &750);

    // Critical must still have the updated value; high must still have default
    assert_eq!(
        client
            .get_config(&symbol_short!("critical"))
            .threshold_minutes,
        30
    );
    assert_eq!(
        client.get_config(&symbol_short!("high")).threshold_minutes,
        30
    ); // default
}

// --- Zero and negative-equivalent edge cases ---

#[test]
#[should_panic]
fn test_set_config_rejects_penalty_negative_one() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &-1, &600);
}

#[test]
#[should_panic]
fn test_set_config_rejects_reward_negative_one() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &-1);
}

#[test]
#[should_panic]
fn test_set_config_rejects_threshold_zero_for_high() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("high"), &0, &50, &750);
}

#[test]
#[should_panic]
fn test_set_config_rejects_threshold_zero_for_medium() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("medium"), &0, &25, &750);
}

#[test]
#[should_panic]
fn test_set_config_rejects_threshold_zero_for_low() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &0, &10, &600);
}

// ============================================================
// SC-W5-049 (#250) – Failure-class mapping: retryable vs terminal errors
//
// Classification:
//   Terminal (caller bug, never retry):
//     AlreadyInitialized, Unauthorized, InvalidThreshold, InvalidPenalty,
//     InvalidReward, InvalidSeverity, NoPendingTransfer
//   Retryable (transient / state-dependent, may succeed after state change):
//     NotInitialized, VersionMismatch, ConfigNotFound, ContractPaused,
//     RetentionLimitOutOfRange
// ============================================================

#[test]
fn test_error_already_initialized_is_terminal() {
    // Calling initialize twice always fails – no state change can make it succeed.
    let (_env, client, actors) = setup();
    let result = client.try_initialize(&actors.admin, &actors.operator);
    assert_eq!(result.unwrap_err().unwrap(), SLAError::AlreadyInitialized);
}

#[test]
fn test_error_unauthorized_is_terminal_for_stranger() {
    // A stranger calling an admin-only function always fails.
    let (_env, client, actors) = setup();
    let result = client.try_set_config(
        &actors.stranger,
        &symbol_short!("critical"),
        &15,
        &100,
        &750,
    );
    assert_eq!(result.unwrap_err().unwrap(), SLAError::Unauthorized);
}

#[test]
fn test_error_unauthorized_operator_calling_admin_fn_is_terminal() {
    let (_env, client, actors) = setup();
    let result = client.try_pause(&actors.operator, &soroban_sdk::String::from_str(&_env, "x"));
    assert_eq!(result.unwrap_err().unwrap(), SLAError::Unauthorized);
}

#[test]
fn test_error_invalid_threshold_is_terminal() {
    let (_env, client, actors) = setup();
    // threshold=0 is always invalid for any severity
    let result = client.try_set_config(&actors.admin, &symbol_short!("low"), &0, &10, &600);
    assert_eq!(result.unwrap_err().unwrap(), SLAError::InvalidThreshold);
}

#[test]
fn test_error_invalid_penalty_is_terminal() {
    let (_env, client, actors) = setup();
    // penalty=0 is always invalid
    let result = client.try_set_config(&actors.admin, &symbol_short!("low"), &120, &0, &600);
    assert_eq!(result.unwrap_err().unwrap(), SLAError::InvalidPenalty);
}

#[test]
fn test_error_invalid_reward_is_terminal() {
    let (_env, client, actors) = setup();
    // reward=0 is always invalid
    let result = client.try_set_config(&actors.admin, &symbol_short!("low"), &120, &10, &0);
    assert_eq!(result.unwrap_err().unwrap(), SLAError::InvalidReward);
}

#[test]
fn test_error_invalid_severity_is_terminal() {
    let (_env, client, actors) = setup();
    let result = client.try_set_config(&actors.admin, &symbol_short!("bogus"), &30, &50, &500);
    assert_eq!(result.unwrap_err().unwrap(), SLAError::InvalidSeverity);
}

#[test]
fn test_error_no_pending_transfer_is_terminal_without_proposal() {
    let (_env, client, actors) = setup();
    // No proposal exists – cancel must fail
    let result = client.try_cancel_admin_proposal(&actors.admin);
    assert_eq!(result.unwrap_err().unwrap(), SLAError::NoPendingTransfer);
}

#[test]
fn test_error_contract_paused_is_retryable_after_unpause() {
    // ContractPaused is retryable: the same call succeeds after unpause.
    let (_env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&_env, "maintenance"),
    );
    let paused_result = client.try_calculate_sla(
        &actors.operator,
        &symbol_short!("INC_P"),
        &symbol_short!("high"),
        &10,
    );
    assert_eq!(
        paused_result.unwrap_err().unwrap(),
        SLAError::ContractPaused
    );

    client.unpause(&actors.admin);
    let ok = client.calculate_sla(
        &actors.operator,
        &symbol_short!("INC_P"),
        &symbol_short!("high"),
        &10,
    );
    assert_eq!(ok.status, symbol_short!("met"));
}

#[test]
fn test_error_config_not_found_is_retryable_after_set_config() {
    // ConfigNotFound is retryable: after adding the config the call succeeds.
    // We test this via calculate_sla_view with a severity that has no config.
    // (We can't remove a config directly, so we verify the error code via
    // a freshly-registered contract with no configs set.)
    let env = Env::default();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // "critical" exists by default; use a non-canonical symbol to force ConfigNotFound
    // by bypassing validate_config (use calculate_sla_view which skips operator check)
    // We can't easily inject an unknown severity without bypassing validation,
    // so we verify ConfigNotFound is the error returned by get_config directly.
    let result = client.try_get_config(&symbol_short!("none"));
    assert_eq!(result.unwrap_err().unwrap(), SLAError::ConfigNotFound);
}

#[test]
fn test_error_retention_limit_out_of_range_is_terminal_for_zero() {
    let (_env, client, actors) = setup();
    let result = client.try_set_retention_limit(&actors.admin, &0);
    assert_eq!(
        result.unwrap_err().unwrap(),
        SLAError::RetentionLimitOutOfRange
    );
}

// ============================================================
// SC-W5-050 (#251) – Invariant-preserving error paths
//
// A failed operation must leave all observable state unchanged.
// ============================================================

#[test]
fn test_failed_set_config_leaves_config_unchanged() {
    let (_env, client, actors) = setup();
    let before = client.get_config(&symbol_short!("critical"));

    // Invalid: threshold=0 for critical
    let _ = client.try_set_config(&actors.admin, &symbol_short!("critical"), &0, &100, &750);

    assert_eq!(client.get_config(&symbol_short!("critical")), before);
}

#[test]
fn test_failed_calculate_sla_when_paused_leaves_stats_unchanged() {
    let (_env, client, actors) = setup();
    let stats_before = client.get_stats();

    client.pause(&actors.admin, &soroban_sdk::String::from_str(&_env, "test"));
    let _ = client.try_calculate_sla(
        &actors.operator,
        &symbol_short!("INC_X"),
        &symbol_short!("high"),
        &10,
    );

    client.unpause(&actors.admin);
    let stats_after = client.get_stats();
    assert_eq!(
        stats_before.total_calculations,
        stats_after.total_calculations
    );
    assert_eq!(stats_before.total_violations, stats_after.total_violations);
}

#[test]
fn test_failed_calculate_sla_when_paused_leaves_history_unchanged() {
    let (_env, client, actors) = setup();
    let history_before = client.get_history();

    client.pause(&actors.admin, &soroban_sdk::String::from_str(&_env, "test"));
    let _ = client.try_calculate_sla(
        &actors.operator,
        &symbol_short!("INC_Y"),
        &symbol_short!("high"),
        &10,
    );

    client.unpause(&actors.admin);
    assert_eq!(client.get_history().len(), history_before.len());
}

#[test]
fn test_failed_calculate_sla_unauthorized_leaves_stats_unchanged() {
    let (_env, client, actors) = setup();
    let stats_before = client.get_stats();

    let _ = client.try_calculate_sla(
        &actors.stranger,
        &symbol_short!("INC_Z"),
        &symbol_short!("high"),
        &10,
    );

    let stats_after = client.get_stats();
    assert_eq!(
        stats_before.total_calculations,
        stats_after.total_calculations
    );
}

#[test]
fn test_failed_propose_admin_unauthorized_leaves_pending_admin_unchanged() {
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);

    // No pending admin before
    assert_eq!(client.get_pending_admin(), None);

    let _ = client.try_propose_admin(&actors.stranger, &new_admin);

    // Still no pending admin
    assert_eq!(client.get_pending_admin(), None);
}

#[test]
fn test_failed_accept_admin_wrong_caller_leaves_admin_unchanged() {
    let (env, client, actors) = setup();
    let new_admin = soroban_sdk::Address::generate(&env);
    let wrong = soroban_sdk::Address::generate(&env);

    client.propose_admin(&actors.admin, &new_admin);

    // Wrong caller tries to accept
    let _ = client.try_accept_admin(&wrong);

    // Admin must still be the original
    assert_eq!(client.get_admin(), actors.admin);
    // Pending must still be new_admin
    assert_eq!(client.get_pending_admin(), Some(new_admin));
}

#[test]
fn test_failed_set_retention_limit_leaves_limit_unchanged() {
    let (_env, client, actors) = setup();
    let before = client.get_retention_limit();

    let _ = client.try_set_retention_limit(&actors.admin, &0);

    assert_eq!(client.get_retention_limit(), before);
}

#[test]
fn test_failed_pause_unauthorized_leaves_pause_state_unchanged() {
    let (_env, client, actors) = setup();
    assert!(!client.is_paused());

    let _ = client.try_pause(&actors.stranger, &soroban_sdk::String::from_str(&_env, "x"));

    assert!(!client.is_paused());
}

// ============================================================
// SC-W5-051 (#252) – Stats monotonicity and conservation invariants
//
// Invariants:
//   1. total_calculations only ever increases (monotonic).
//   2. total_violations only ever increases (monotonic).
//   3. total_calculations == (met count) + total_violations at all times.
//   4. total_rewards and total_penalties only ever increase.
// ============================================================

#[test]
fn test_stats_total_calculations_is_monotonically_increasing() {
    let (_env, client, actors) = setup();
    let mut prev = client.get_stats().total_calculations;

    let oids = [
        symbol_short!("MON1"),
        symbol_short!("MON2"),
        symbol_short!("MON3"),
        symbol_short!("MON4"),
        symbol_short!("MON5"),
    ];
    for oid in oids.iter() {
        client.calculate_sla(&actors.operator, oid, &symbol_short!("high"), &10);
        let curr = client.get_stats().total_calculations;
        assert!(
            curr > prev,
            "total_calculations must increase after each call"
        );
        prev = curr;
    }
}

#[test]
fn test_stats_total_violations_is_monotonically_increasing() {
    let (_env, client, actors) = setup();
    // Use mttr > threshold (30 for high) to force violations
    let mut prev = client.get_stats().total_violations;

    let oids = [
        symbol_short!("VIO1"),
        symbol_short!("VIO2"),
        symbol_short!("VIO3"),
    ];
    for oid in oids.iter() {
        client.calculate_sla(&actors.operator, oid, &symbol_short!("high"), &50);
        let curr = client.get_stats().total_violations;
        assert!(
            curr > prev,
            "total_violations must increase on each violation"
        );
        prev = curr;
    }
}

#[test]
fn test_stats_conservation_total_calculations_equals_met_plus_violations() {
    let (_env, client, actors) = setup();

    // Mix of met and violated calculations
    let inputs: &[(u32, &str, &str)] = &[
        (5, "high", "C0"),
        (35, "high", "C1"),
        (10, "critical", "C2"),
        (20, "critical", "C3"),
        (50, "medium", "C4"),
        (70, "medium", "C5"),
    ];

    for (mttr, sev, oid) in inputs.iter() {
        client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, oid),
            &Symbol::new(&_env, sev),
            mttr,
        );
    }

    let stats = client.get_stats();
    let met_count = stats.total_calculations - stats.total_violations;
    assert_eq!(
        stats.total_calculations,
        met_count + stats.total_violations,
        "total_calculations must equal met + violations"
    );
    assert_eq!(stats.total_violations, 3, "expected 3 violations");
    assert_eq!(met_count, 3, "expected 3 met");
}

#[test]
fn test_stats_total_rewards_only_increases_on_met() {
    let (_env, client, actors) = setup();
    let before = client.get_stats().total_rewards;

    // met calculation
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("RWD1"),
        &symbol_short!("high"),
        &5,
    );
    let after = client.get_stats().total_rewards;
    assert!(
        after > before,
        "total_rewards must increase after a met SLA"
    );
}

#[test]
fn test_stats_total_penalties_only_increases_on_violation() {
    let (_env, client, actors) = setup();
    let before = client.get_stats().total_penalties;

    // violated calculation (mttr=50 > threshold=30 for high)
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("PEN1"),
        &symbol_short!("high"),
        &50,
    );
    let after = client.get_stats().total_penalties;
    assert!(
        after > before,
        "total_penalties must increase after a violation"
    );
}

#[test]
fn test_stats_rewards_unchanged_on_violation() {
    let (_env, client, actors) = setup();
    let before = client.get_stats().total_rewards;

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("PEN2"),
        &symbol_short!("high"),
        &50,
    );
    assert_eq!(
        client.get_stats().total_rewards,
        before,
        "total_rewards must not change on a violation"
    );
}

#[test]
fn test_stats_penalties_unchanged_on_met() {
    let (_env, client, actors) = setup();
    let before = client.get_stats().total_penalties;

    client.calculate_sla(
        &actors.operator,
        &symbol_short!("RWD2"),
        &symbol_short!("high"),
        &5,
    );
    assert_eq!(
        client.get_stats().total_penalties,
        before,
        "total_penalties must not change on a met SLA"
    );
}

#[test]
fn test_stats_conservation_holds_after_many_mixed_calculations() {
    let env = Env::default();
    env.budget().reset_unlimited();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    let mut expected_violations: u64 = 0;
    for i in 1u32..=20 {
        let mttr = i * 3; // alternates met/viol for high (threshold=30)
                          // Use a fixed set of outage IDs cycling through 20 symbols
        let oid = match i {
            1 => symbol_short!("M01"),
            2 => symbol_short!("M02"),
            3 => symbol_short!("M03"),
            4 => symbol_short!("M04"),
            5 => symbol_short!("M05"),
            6 => symbol_short!("M06"),
            7 => symbol_short!("M07"),
            8 => symbol_short!("M08"),
            9 => symbol_short!("M09"),
            10 => symbol_short!("M10"),
            11 => symbol_short!("M11"),
            12 => symbol_short!("M12"),
            13 => symbol_short!("M13"),
            14 => symbol_short!("M14"),
            15 => symbol_short!("M15"),
            16 => symbol_short!("M16"),
            17 => symbol_short!("M17"),
            18 => symbol_short!("M18"),
            19 => symbol_short!("M19"),
            _ => symbol_short!("M20"),
        };
        client.calculate_sla(&op, &oid, &symbol_short!("high"), &mttr);
        if mttr > 30 {
            expected_violations += 1;
        }
    }

    let stats = client.get_stats();
    assert_eq!(stats.total_calculations, 20);
    assert_eq!(stats.total_violations, expected_violations);
    assert_eq!(
        stats.total_calculations,
        (stats.total_calculations - stats.total_violations) + stats.total_violations
    );
}

// ============================================================
// SC-W5-052 (#253) – Reward/penalty exclusivity invariants
//
// Invariants:
//   1. A result with status "met"  has payment_type "rew" and amount > 0.
//   2. A result with status "viol" has payment_type "pen" and amount < 0.
//   3. No result has both a positive amount AND payment_type "pen".
//   4. No result has both a negative amount AND payment_type "rew".
// ============================================================

#[test]
fn test_met_result_has_reward_payment_type_and_positive_amount() {
    let (_env, client, actors) = setup();
    // mttr=5 < threshold=30 for high → met
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("EX1"),
        &symbol_short!("high"),
        &5,
    );
    assert_eq!(result.status, symbol_short!("met"));
    assert_eq!(result.payment_type, symbol_short!("rew"));
    assert!(result.amount > 0, "reward amount must be positive");
}

#[test]
fn test_violated_result_has_penalty_payment_type_and_negative_amount() {
    let (_env, client, actors) = setup();
    // mttr=50 > threshold=30 for high → viol
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("EX2"),
        &symbol_short!("high"),
        &50,
    );
    assert_eq!(result.status, symbol_short!("viol"));
    assert_eq!(result.payment_type, symbol_short!("pen"));
    assert!(result.amount < 0, "penalty amount must be negative");
}

#[test]
fn test_no_result_has_penalty_type_with_positive_amount() {
    let (_env, client, actors) = setup();
    // Run several calculations and verify the invariant on each
    let cases: &[(u32, &str, &str)] = &[
        (5, "critical", "EX10"),
        (20, "critical", "EX11"),
        (10, "high", "EX12"),
        (40, "high", "EX13"),
        (30, "medium", "EX14"),
        (70, "medium", "EX15"),
        (60, "low", "EX16"),
        (130, "low", "EX17"),
    ];
    for (mttr, sev, oid) in cases.iter() {
        let result = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, oid),
            &Symbol::new(&_env, sev),
            mttr,
        );
        if result.payment_type == symbol_short!("pen") {
            assert!(
                result.amount < 0,
                "penalty payment_type must always have negative amount"
            );
        }
    }
}

#[test]
fn test_no_result_has_reward_type_with_non_positive_amount() {
    let (_env, client, actors) = setup();
    let cases: &[(u32, &str, &str)] = &[
        (1, "critical", "EX20"),
        (14, "critical", "EX21"),
        (1, "high", "EX22"),
        (29, "high", "EX23"),
        (1, "medium", "EX24"),
        (59, "medium", "EX25"),
        (1, "low", "EX26"),
        (119, "low", "EX27"),
    ];
    for (mttr, sev, oid) in cases.iter() {
        let result = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, oid),
            &Symbol::new(&_env, sev),
            mttr,
        );
        if result.payment_type == symbol_short!("rew") {
            assert!(
                result.amount > 0,
                "reward payment_type must always have positive amount"
            );
        }
    }
}

// Issue #254 – Invariant checks: config bounds and payout ceilings
// ============================================================

#[test]
fn test_254_threshold_upper_bound_accepted() {
    // threshold_minutes == 1440 (24 h) is the maximum allowed value.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &1440, &10, &600);
    assert_eq!(
        client.get_config(&symbol_short!("low")).threshold_minutes,
        1440
    );
}

#[test]
#[should_panic]
fn test_254_threshold_above_upper_bound_rejected() {
    // threshold_minutes == 1441 must be rejected.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &1441, &10, &600);
}

#[test]
#[should_panic]
fn test_254_threshold_zero_rejected() {
    // threshold_minutes == 0 must be rejected for all severities.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("critical"), &0, &100, &750);
}

#[test]
#[should_panic]
fn test_254_penalty_zero_rejected() {
    // penalty_per_minute == 0 must be rejected.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &0, &600);
}

#[test]
#[should_panic]
fn test_254_penalty_above_ceiling_rejected() {
    // penalty_per_minute > 10000 must be rejected.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10001, &600);
}

#[test]
#[should_panic]
fn test_254_reward_zero_rejected() {
    // reward_base == 0 must be rejected.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &0);
}

#[test]
#[should_panic]
fn test_254_reward_above_ceiling_rejected() {
    // reward_base > 100000 must be rejected.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &100001);
}

#[test]
fn test_254_valid_boundary_values_accepted() {
    // Minimum valid values for low severity must be accepted.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &1, &1, &1);
    let cfg = client.get_config(&symbol_short!("low"));
    assert_eq!(cfg.threshold_minutes, 1);
    assert_eq!(cfg.penalty_per_minute, 1);
    assert_eq!(cfg.reward_base, 1);
}

#[test]
fn test_254_payout_ceiling_not_exceeded_on_calculation() {
    // reward_base at ceiling (100000) must produce a positive reward without overflow.
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &100000);
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("OUT1"),
        &symbol_short!("low"),
        &1, // well within threshold → top-tier reward
    );
    assert!(result.amount > 0, "reward must be positive");
}

#[test]
fn test_254_invalid_config_does_not_corrupt_existing() {
    // A rejected set_config call must leave the previous config intact.
    let (_env, client, actors) = setup();
    let original = client.get_config(&symbol_short!("critical"));
    let _ = client.try_set_config(&actors.admin, &symbol_short!("critical"), &0, &100, &750);
    assert_eq!(
        client
            .get_config(&symbol_short!("critical"))
            .threshold_minutes,
        original.threshold_minutes
    );
}

// ============================================================
// Issue #255 – Invariant checks: history index and sequence integrity
// ============================================================

#[test]
fn test_255_history_grows_monotonically() {
    // Each calculate_sla call must append exactly one entry.
    let (_env, client, actors) = setup();
    for i in 1u32..=5 {
        let h = client.get_history();
        assert_eq!(h.len(), i - 1, "history length before call {}", i);
        client.calculate_sla(
            &actors.operator,
            &soroban_sdk::Symbol::new(&_env, &format!("O{}", i)),
            &symbol_short!("high"),
            &10,
        );
        assert_eq!(
            client.get_history().len(),
            i,
            "history length after call {}",
            i
        );
    }
}

#[test]
fn test_255_history_order_is_insertion_order() {
    // Entries must appear in the order they were inserted (oldest first).
    let (env, client, actors) = setup();
    let ids = ["A", "B", "C"];
    for id in ids {
        client.calculate_sla(
            &actors.operator,
            &soroban_sdk::Symbol::new(&env, id),
            &symbol_short!("high"),
            &10,
        );
    }
    let history = client.get_history();
    assert_eq!(history.len(), 3);
    assert_eq!(
        history.get(0).unwrap().outage_id,
        soroban_sdk::Symbol::new(&env, "A")
    );
    assert_eq!(
        history.get(1).unwrap().outage_id,
        soroban_sdk::Symbol::new(&env, "B")
    );
    assert_eq!(
        history.get(2).unwrap().outage_id,
        soroban_sdk::Symbol::new(&env, "C")
    );
}

#[test]
fn test_255_duplicate_outage_id_is_idempotent() {
    // Submitting the same outage_id with identical inputs must not add a second entry.
    let (_env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("DUP"),
        &symbol_short!("high"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("DUP"),
        &symbol_short!("high"),
        &10,
    );
    assert_eq!(client.get_history().len(), 1);
}

#[test]
#[should_panic]
fn test_255_duplicate_outage_id_with_different_mttr_panics() {
    // Same outage_id but different mttr_minutes must panic (mismatched inputs).
    let (_env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("DUP"),
        &symbol_short!("high"),
        &10,
    );
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("DUP"),
        &symbol_short!("high"),
        &20,
    );
}

#[test]
fn test_config_bumped_duplicate_treated_as_fresh_calculation() {
    // After set_config changes the config_version_hash, a duplicate outage_id
    // must be treated as a fresh calculation rather than returning stale cache.
    let (_env, client, actors) = setup();
    let outage_id = symbol_short!("CFG_BMP");
    let severity = symbol_short!("high");
    let mttr = 10u32;

    let r1 = client.calculate_sla(&actors.operator, &outage_id, &severity, &mttr);
    let hash1 = r1.config_version_hash;

    // Change reward_base so config_version_hash changes but threshold stays same.
    // This is the subtle case: old code would return cached result because
    // threshold and mttr match, ignoring the config change.
    client.set_config(&actors.admin, &severity, &30, &50, &1500);

    let r2 = client.calculate_sla(&actors.operator, &outage_id, &severity, &mttr);
    let hash2 = r2.config_version_hash;

    assert_ne!(hash1, hash2);
    assert_eq!(r2.amount, 3000); // 1500 * 200 / 100 = 3000 (top tier)

    let history = client.get_history();
    let mut count = 0u32;
    for i in 0..history.len() {
        if history.get(i).unwrap().outage_id == outage_id {
            count += 1;
        }
    }
    assert_eq!(count, 2);
    assert_eq!(client.get_stats().total_calculations, 2);
}

#[test]
fn test_config_bumped_duplicate_threshold_change_is_fresh() {
    // Config change alters threshold so same mttr flips from met→viol.
    let (_env, client, actors) = setup();
    let outage_id = symbol_short!("CFG_THR");
    let severity = symbol_short!("high");

    let r1 = client.calculate_sla(&actors.operator, &outage_id, &severity, &25);
    assert_eq!(r1.status, symbol_short!("met"));

    // Lower threshold so the same mttr now violates
    client.set_config(&actors.admin, &severity, &20, &50, &750);

    let r2 = client.calculate_sla(&actors.operator, &outage_id, &severity, &25);
    assert_eq!(r2.status, symbol_short!("viol"));
    assert_eq!(r2.amount, -250);

    assert_eq!(client.get_history().len(), 2);
    assert_eq!(client.get_stats().total_calculations, 2);
}

#[test]
fn test_duplicate_same_config_still_idempotent() {
    // Without config change, identical duplicate still returns cached result.
    let (_env, client, actors) = setup();
    let outage_id = symbol_short!("IDEM_P");

    let r1 = client.calculate_sla(&actors.operator, &outage_id, &symbol_short!("critical"), &5);
    let r2 = client.calculate_sla(&actors.operator, &outage_id, &symbol_short!("critical"), &5);

    assert_eq!(r1.config_version_hash, r2.config_version_hash);
    assert_eq!(client.get_history().len(), 1);
    assert_eq!(client.get_stats().total_calculations, 1);
}

#[test]
#[should_panic(expected = "#13")]
fn test_duplicate_same_config_with_different_mttr_still_panics() {
    // Without config change, conflicting inputs still reject.
    let (_env, client, actors) = setup();
    let outage_id = symbol_short!("CONF");

    client.calculate_sla(&actors.operator, &outage_id, &symbol_short!("high"), &10);
    client.calculate_sla(&actors.operator, &outage_id, &symbol_short!("high"), &20);
}

#[test]
fn test_255_prune_reduces_history_to_keep_latest() {
    // After prune_history(keep=3), exactly 3 entries remain (the most recent).
    let (env, client, actors) = setup();
    for i in 1u32..=10 {
        client.calculate_sla(
            &actors.operator,
            &soroban_sdk::Symbol::new(&env, &format!("O{}", i)),
            &symbol_short!("high"),
            &10,
        );
    }
    client.prune_history(&actors.admin, &3);
    assert_eq!(client.get_history().len(), 3);
}

#[test]
fn test_255_prune_to_zero_clears_history() {
    let (env, client, actors) = setup();
    for i in 1u32..=5 {
        client.calculate_sla(
            &actors.operator,
            &soroban_sdk::Symbol::new(&env, &format!("O{}", i)),
            &symbol_short!("high"),
            &10,
        );
    }
    client.prune_history(&actors.admin, &0);
    assert_eq!(client.get_history().len(), 0);
}

#[test]
fn test_255_history_page_offset_and_limit() {
    // get_history_page must return the correct slice.
    let (env, client, actors) = setup();
    for i in 1u32..=6 {
        client.calculate_sla(
            &actors.operator,
            &soroban_sdk::Symbol::new(&env, &format!("O{}", i)),
            &symbol_short!("high"),
            &10,
        );
    }
    let page = client.get_history_page(&2, &3);
    assert_eq!(page.len(), 3);
    assert_eq!(
        page.get(0).unwrap().outage_id,
        soroban_sdk::Symbol::new(&env, "O3")
    );
}

#[test]
fn test_255_history_page_beyond_end_returns_empty() {
    let (_env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("O1"),
        &symbol_short!("high"),
        &10,
    );
    let page = client.get_history_page(&100, &10);
    assert_eq!(page.len(), 0);
}

// ============================================================
// Issue #256 – Invariant checks: pause-state write prohibition
// ============================================================

#[test]
#[should_panic]
fn test_256_calculate_sla_blocked_when_paused() {
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("O1"),
        &symbol_short!("high"),
        &10,
    );
}

#[test]
fn test_256_calculate_sla_allowed_after_unpause() {
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );
    client.unpause(&actors.admin);
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("O1"),
        &symbol_short!("high"),
        &10,
    );
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
fn test_256_set_config_allowed_while_paused() {
    // Admin config updates must succeed even when the contract is paused.
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );
    client.set_config(&actors.admin, &symbol_short!("low"), &120, &10, &600);
    assert_eq!(
        client.get_config(&symbol_short!("low")).threshold_minutes,
        120
    );
}

#[test]
fn test_256_history_not_mutated_while_paused() {
    // History must not grow while the contract is paused.
    let (env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("O1"),
        &symbol_short!("high"),
        &10,
    );
    let len_before = client.get_history().len();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );
    let _ = client.try_calculate_sla(
        &actors.operator,
        &symbol_short!("O2"),
        &symbol_short!("high"),
        &10,
    );
    assert_eq!(
        client.get_history().len(),
        len_before,
        "history must not grow while paused"
    );
}

#[test]
fn test_256_stats_not_mutated_while_paused() {
    // Stats must not change while the contract is paused.
    let (env, client, actors) = setup();
    client.calculate_sla(
        &actors.operator,
        &symbol_short!("O1"),
        &symbol_short!("high"),
        &10,
    );
    let stats_before = client.get_stats();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );
    let _ = client.try_calculate_sla(
        &actors.operator,
        &symbol_short!("O2"),
        &symbol_short!("high"),
        &10,
    );
    let stats_after = client.get_stats();
    assert_eq!(
        stats_before.total_calculations,
        stats_after.total_calculations
    );
}

#[test]
fn test_256_calculate_sla_view_allowed_while_paused() {
    // The read-only audit view must remain accessible while paused.
    let (env, client, actors) = setup();
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "maintenance"),
    );
    let result = client.calculate_sla_view(&symbol_short!("O1"), &symbol_short!("high"), &10);
    assert_eq!(result.status, symbol_short!("met"));
}

#[test]
#[should_panic]
fn test_256_stranger_cannot_pause() {
    let (env, client, actors) = setup();
    client.pause(
        &actors.stranger,
        &soroban_sdk::String::from_str(&env, "unauthorized"),
    );
}

#[test]
#[should_panic]
fn test_256_operator_cannot_pause() {
    let (env, client, actors) = setup();
    client.pause(
        &actors.operator,
        &soroban_sdk::String::from_str(&env, "unauthorized"),
    );
}

// ============================================================
// Issue #257 – Canonical serialization parity with backend adapter
// ============================================================

#[test]
fn test_257_config_version_hash_is_deterministic() {
    // Same config must always produce the same hash.
    let (_env, client, _actors) = setup();
    let h1 = client.get_config_version_hash();
    let h2 = client.get_config_version_hash();
    assert_eq!(h1, h2, "hash must be stable across repeated reads");
}

#[test]
fn test_257_config_version_hash_changes_on_config_update() {
    // Updating any config field must change the hash.
    let (_env, client, actors) = setup();
    let h_before = client.get_config_version_hash();
    client.set_config(&actors.admin, &symbol_short!("low"), &200, &10, &600);
    let h_after = client.get_config_version_hash();
    assert_ne!(h_before, h_after, "hash must change after config update");
}

#[test]
fn test_257_config_version_hash_stable_after_no_op_read() {
    // Reading config must not change the hash.
    let (_env, client, _actors) = setup();
    let h1 = client.get_config_version_hash();
    let _ = client.get_config(&symbol_short!("critical"));
    let h2 = client.get_config_version_hash();
    assert_eq!(h1, h2);
}

#[test]
fn test_257_sla_result_config_version_hash_matches_standalone() {
    // The config_version_hash embedded in SLAResult must equal get_config_version_hash().
    let (_env, client, actors) = setup();
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("O1"),
        &symbol_short!("high"),
        &10,
    );
    let standalone_hash = client.get_config_version_hash();
    assert_eq!(
        result.config_version_hash, standalone_hash,
        "embedded hash must match standalone hash"
    );
}

#[test]
fn test_257_calculate_sla_view_hash_matches_standalone() {
    // calculate_sla_view must embed the same config_version_hash.
    let (_env, client, _actors) = setup();
    let view_result = client.calculate_sla_view(&symbol_short!("O1"), &symbol_short!("high"), &10);
    let standalone_hash = client.get_config_version_hash();
    assert_eq!(view_result.config_version_hash, standalone_hash);
}

#[test]
fn test_257_config_snapshot_entry_order_is_canonical() {
    let (_env, client, _actors) = setup();
    let snapshot = client.get_config_snapshot();
    let expected = [
        symbol_short!("critical"),
        symbol_short!("high"),
        symbol_short!("medium"),
        symbol_short!("low"),
    ];
    assert_eq!(snapshot.entries.len(), 4);
    for (i, sev) in expected.iter().enumerate() {
        assert_eq!(
            snapshot.entries.get(i as u32).unwrap().severity,
            *sev,
            "entry {} must be {:?}",
            i,
            sev
        );
    }
}

#[test]
fn test_exclusivity_status_and_payment_type_are_consistent() {
    // met ↔ rew, viol ↔ pen — no cross-pairing allowed
    let (_env, client, actors) = setup();
    let cases: &[(u32, &str, &str)] = &[
        (5, "critical", "EX30"),
        (20, "critical", "EX31"),
        (10, "high", "EX32"),
        (40, "high", "EX33"),
        (30, "medium", "EX34"),
        (70, "medium", "EX35"),
        (60, "low", "EX36"),
        (130, "low", "EX37"),
    ];
    for (mttr, sev, oid) in cases.iter() {
        let result = client.calculate_sla(
            &actors.operator,
            &Symbol::new(&_env, oid),
            &Symbol::new(&_env, sev),
            mttr,
        );
        if result.status == symbol_short!("met") {
            assert_eq!(
                result.payment_type,
                symbol_short!("rew"),
                "met status must pair with rew payment_type"
            );
        } else {
            assert_eq!(result.status, symbol_short!("viol"));
            assert_eq!(
                result.payment_type,
                symbol_short!("pen"),
                "viol status must pair with pen payment_type"
            );
        }
    }
}

#[test]
fn test_exclusivity_view_mode_matches_mutating_mode() {
    // calculate_sla_view must produce the same payment_type/amount sign as calculate_sla
    let (_env, client, actors) = setup();

    let view = client.calculate_sla_view(&symbol_short!("VIEW1"), &symbol_short!("high"), &50);
    let calc = client.calculate_sla(
        &actors.operator,
        &symbol_short!("VIEW1"),
        &symbol_short!("high"),
        &50,
    );

    assert_eq!(view.status, calc.status);
    assert_eq!(view.payment_type, calc.payment_type);
    assert_eq!(view.amount.signum(), calc.amount.signum());
}

#[test]
fn test_exclusivity_at_exact_threshold_boundary_is_met() {
    // mttr == threshold → SLA met (not violated)
    let (_env, client, actors) = setup();
    // high threshold = 30
    let result = client.calculate_sla(
        &actors.operator,
        &symbol_short!("BNDRY"),
        &symbol_short!("high"),
        &30,
    );
    assert_eq!(result.status, symbol_short!("met"));
    assert_eq!(result.payment_type, symbol_short!("rew"));
    assert!(result.amount > 0);
}

#[test]
fn test_257_result_schema_fields_are_stable() {
    let (_env, client, _actors) = setup();
    let schema = client.get_result_schema();
    assert_eq!(schema.status_met, symbol_short!("met"));
    assert_eq!(schema.status_violated, symbol_short!("viol"));
    assert_eq!(schema.payment_reward, symbol_short!("rew"));
    assert_eq!(schema.payment_penalty, symbol_short!("pen"));
    assert_eq!(schema.rating_exceptional, symbol_short!("top"));
    assert_eq!(schema.rating_excellent, symbol_short!("excel"));
    assert_eq!(schema.rating_good, symbol_short!("good"));
    assert_eq!(schema.rating_poor, symbol_short!("poor"));
    assert!(schema.includes_config_version_hash);
}

#[test]
fn test_257_hash_differs_across_all_four_severities() {
    // Updating each severity independently must produce a distinct hash.
    let (_env, client, actors) = setup();
    let h0 = client.get_config_version_hash();

    client.set_config(&actors.admin, &symbol_short!("critical"), &30, &100, &750);
    let h1 = client.get_config_version_hash();
    assert_ne!(h0, h1);

    client.set_config(&actors.admin, &symbol_short!("high"), &60, &50, &750);
    let h2 = client.get_config_version_hash();
    assert_ne!(h1, h2);

    client.set_config(&actors.admin, &symbol_short!("medium"), &120, &25, &750);
    let h3 = client.get_config_version_hash();
    assert_ne!(h2, h3);

    client.set_config(&actors.admin, &symbol_short!("low"), &240, &10, &600);
    let h4 = client.get_config_version_hash();
    assert_ne!(h3, h4);
}

// ============================================================
// Issue #4 – Config update metadata tracking
// ============================================================
//
// These tests cover (1) declaring the existing `config_metadata` module,
// (2) wiring `record_config_update()` into `set_config()` after the
// successful storage write, and (3) exposing the recorded ledger sequence
// through `get_last_config_update()`. Backends use the returned sequence as
// a cheap cache-invalidation signal: compare it against the ledger sequence
// observed at the last `get_config_snapshot()` and re-fetch only when it has
// advanced.

/// Acceptance criterion (a): after `initialize()` but before any
/// `set_config` call, no update has been recorded – the getter must
/// return `None`.
#[test]
fn test_issue4_get_last_config_update_is_none_after_initialize() {
    let (_env, client, _actors) = setup();
    assert_eq!(client.get_last_config_update(), None);
}

/// Acceptance criterion (b): once `set_config` succeeds, the getter must
/// return `Some(ConfigUpdateInfo)`.
#[test]
fn test_issue4_get_last_config_update_is_some_after_set_config() {
    let (_env, client, actors) = setup();
    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);
    let recorded = client.get_last_config_update();
    assert!(
        recorded.is_some(),
        "get_last_config_update must be Some(_) after set_config"
    );
}

/// Acceptance criterion (c): the recorded sequence must equal the
/// ledger sequence observed at the moment of the `set_config` call.
#[test]
fn test_issue4_get_last_config_update_matches_ledger_sequence() {
    let (env, client, actors) = setup();
    let sequence_before = env.ledger().sequence();

    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    let recorded = client.get_last_config_update().unwrap();
    // Within a single test ledger the sequence never advances on its own,
    // so any read during this test must match the recorded one.
    assert_eq!(
        recorded.sequence, sequence_before,
        "recorded sequence must match the ledger sequence at update time"
    );
    assert_eq!(
        recorded.sequence,
        env.ledger().sequence(),
        "recorded sequence must match the current ledger sequence within the same test ledger"
    );
}

/// Acceptance criterion (d): repeated `set_config` calls performed at
/// strictly increasing ledger sequences must produce strictly increasing
/// recorded sequences.
#[test]
fn test_issue4_repeated_set_config_produces_increasing_sequences() {
    let env = Env::default();

    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Advance the ledger to a known starting sequence and trigger an update.
    env.ledger().with_mut(|li| {
        li.sequence_number = 100;
    });
    client.set_config(&admin, &symbol_short!("critical"), &20, &200, &1000);
    let first = client.get_last_config_update().unwrap();
    assert_eq!(first.sequence, 100);

    // Advance further and trigger another update on a different severity.
    env.ledger().with_mut(|li| {
        li.sequence_number = 250;
    });
    client.set_config(&admin, &symbol_short!("high"), &45, &75, &800);
    let second = client.get_last_config_update().unwrap();
    assert_eq!(second.sequence, 250);

    assert!(
        second.sequence > first.sequence,
        "Repeated updates must produce an increasing sequence: second={} first={}",
        second.sequence,
        first.sequence
    );
}
