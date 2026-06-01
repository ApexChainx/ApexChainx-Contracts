//! SC-W5-044 – Event payload versioning and backward compatibility rules.
//!
//! This module tests that event payloads follow versioning rules:
//! - Breaking changes (field removal, type changes, reordering) require a
//!   version symbol bump from "v1" to "v2".
//! - Additive changes (new fields appended at the end) are backward-compatible
//!   and do NOT require a version bump.
//! - Old consumers can safely ignore unrecognised trailing fields.
//!
//! The current canonical event version is "v1" (EVENT_VERSION).

#[cfg(test)]
mod payload_versioning_tests {
    use soroban_sdk::{
        symbol_short, testutils::Address as _, testutils::Events, Address, Env, Symbol,
        TryIntoVal,
    };
    use crate::{
        EVENT_CONFIG_UPD, EVENT_PAUSED, EVENT_PRUNED, EVENT_PRUNED_AGE, EVENT_SETTLE_INTENT,
        EVENT_SLA_CALC, EVENT_UNPAUSED, EVENT_VERSION, SLACalculatorContract,
        SLACalculatorContractClient,
    };

    fn setup(env: &Env) -> (Address, Address, SLACalculatorContractClient) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        (admin, operator, client)
    }

    // ── sla_calc payload versioning ─────────────────────────────────────

    #[test]
    fn test_sla_calc_payload_has_seven_fields() {
        // sla_calc payload: (outage_id, status, payment_type, rating, mttr, threshold, amount)
        // This is the canonical 7-field format. Adding fields would be backward-compatible.
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("VERSION1"),
            &symbol_short!("critical"),
            &5,
        );

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, data) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_SLA_CALC {
                // Canonical 7-field payload must decode without error
                let payload: Result<(Symbol, Symbol, Symbol, Symbol, u32, u32, i128), _> =
                    data.try_into_val(&env);
                assert!(
                    payload.is_ok(),
                    "sla_calc payload must decode as 7-field tuple"
                );
                let (outage_id, status, ptype, rating, mttr, threshold, amount) = payload.unwrap();
                assert_eq!(outage_id, symbol_short!("VERSION1"));
                assert_eq!(status, symbol_short!("met"));
                assert_eq!(ptype, symbol_short!("rew"));
                assert_eq!(rating, symbol_short!("top"));
                assert_eq!(mttr, 5u32);
                assert_eq!(threshold, 15u32);
                assert_eq!(amount, 1500i128);
                return;
            }
        }
        panic!("sla_calc event not found");
    }

    // ── set_int payload versioning ──────────────────────────────────────

    #[test]
    fn test_settle_intent_payload_has_six_fields() {
        // set_int payload: (outage_id, status, payment_type, amount, config_hash, recorded_at)
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("VERSION2"),
            &symbol_short!("critical"),
            &5,
        );

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, data) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_SETTLE_INTENT {
                // 6-field payload must decode without error
                let payload: Result<(Symbol, Symbol, Symbol, i128, u64, u64), _> =
                    data.try_into_val(&env);
                assert!(
                    payload.is_ok(),
                    "set_int payload must decode as 6-field tuple"
                );
                return;
            }
        }
        panic!("set_int event not found");
    }

    // ── cfg_upd payload versioning ──────────────────────────────────────

    #[test]
    fn test_cfg_upd_payload_has_three_fields() {
        // cfg_upd payload: (threshold_minutes, penalty_per_minute, reward_base)
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.set_config(&admin, &symbol_short!("critical"), &20, &200, &1000);

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, _, data) = events.get(i).unwrap();
            let payload: Result<(u32, i128, i128), _> = data.try_into_val(&env);
            if payload.is_ok() {
                let (thresh, penalty, reward) = payload.unwrap();
                assert_eq!(thresh, 20u32, "cfg_upd payload field 0 mismatch");
                assert_eq!(penalty, 200i128, "cfg_upd payload field 1 mismatch");
                assert_eq!(reward, 1000i128, "cfg_upd payload field 2 mismatch");
                return;
            }
        }
        panic!("cfg_upd event not found");
    }

    // ── Pause/unpause payload versioning ────────────────────────────────

    #[test]
    fn test_pause_payload_is_single_bool() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.pause(&admin);

        let events = env.events().all();
        let (_, _, data) = events.last().unwrap();
        let payload: (bool,) = data.try_into_val(&env).unwrap();
        assert_eq!(payload, (true,), "pause payload must be (true,)");
    }

    #[test]
    fn test_unpause_payload_is_single_bool() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.pause(&admin);
        client.unpause(&admin);

        let events = env.events().all();
        let (_, _, data) = events.last().unwrap();
        let payload: (bool,) = data.try_into_val(&env).unwrap();
        assert_eq!(payload, (false,), "unpause payload must be (false,)");
    }

    // ── Prune payload versioning ────────────────────────────────────────

    #[test]
    fn test_prune_payload_is_two_u32s() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        for _ in 0..5u32 {
            client.calculate_sla(
                &operator,
                &symbol_short!("PR_VER"),
                &symbol_short!("low"),
                &10,
            );
        }

        client.prune_history(&admin, &2);

        let events = env.events().all();
        let (_, _, data) = events.last().unwrap();
        let payload: (u32, u32) = data.try_into_val(&env).unwrap();
        assert_eq!(payload, (3u32, 2u32), "prune payload must be (removed, kept)");
    }

    // ── Event version is always "v1" ────────────────────────────────────

    #[test]
    fn test_all_events_use_current_version() {
        let env = Env::default();
        let (admin, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("VER_ALL"),
            &symbol_short!("critical"),
            &5,
        );
        client.set_config(&admin, &symbol_short!("critical"), &20, &200, &1000);
        client.pause(&admin);
        client.unpause(&admin);

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() >= 2 {
                let version: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
                assert_eq!(
                    version, EVENT_VERSION,
                    "All events must use the current event version"
                );
            }
        }
    }

    // ── Payload field count consistency ─────────────────────────────────

    #[test]
    fn test_violation_payload_has_negative_amount() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        let result = client.calculate_sla(
            &operator,
            &symbol_short!("VIOL_PAY"),
            &symbol_short!("critical"),
            &25, // violation: 10 min over threshold
        );

        assert_eq!(result.status, symbol_short!("viol"));
        assert!(result.amount < 0, "Violation amount must be negative");

        // Verify the event payload matches
        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, data) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_SLA_CALC {
                let payload: (Symbol, Symbol, Symbol, Symbol, u32, u32, i128) =
                    data.try_into_val(&env).unwrap();
                assert_eq!(payload.6, result.amount, "Event amount must match result");
                return;
            }
        }
        panic!("sla_calc event not found");
    }
}
