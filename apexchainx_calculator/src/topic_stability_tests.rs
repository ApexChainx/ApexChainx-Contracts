//! SC-W5-043 – Event topic stability contract for backend indexers.
//!
//! This module tests that event topic structures remain stable across repeated
//! calls and operations. Backend indexers depend on a fixed topic structure:
//!
//! topic[0]: event name Symbol (constant per event type)
//! topic[1]: event version Symbol ("v1")
//! topic[2]: event-specific context (severity, caller, etc.)
//!
//! These tests verify that the topic structure never changes between operations.

#[cfg(test)]
mod topic_stability_tests {
    use soroban_sdk::{
        symbol_short, testutils::Address as _, testutils::Events, Address, Env, Symbol,
        TryIntoVal,
    };
    use crate::{
        EVENT_ADMIN_ACC, EVENT_ADMIN_CAN, EVENT_ADMIN_PROP, EVENT_ADMIN_REN, EVENT_CONFIG_UPD,
        EVENT_OP_ACC, EVENT_OP_CAN, EVENT_OP_PROP, EVENT_OP_SET, EVENT_PAUSED, EVENT_PRUNED,
        EVENT_PRUNED_AGE, EVENT_SETTLE_INTENT, EVENT_SLA_CALC, EVENT_UNPAUSED, EVENT_VERSION,
        SLACalculatorContract, SLACalculatorContractClient,
    };

    fn setup(env: &Env) -> (Address, Address, SLACalculatorContractClient) {
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        (admin, operator, client)
    }

    // ── Auth-gated negative tests ───────────────────────────────────────

    #[test]
    #[should_panic]
    fn test_stranger_cannot_calculate_sla() {
        let env = Env::default();
        let (_admin, _operator, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.calculate_sla(
            &stranger,
            &symbol_short!("U_TS"),
            &symbol_short!("critical"),
            &5,
        );
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_set_config() {
        let env = Env::default();
        let (_admin, _operator, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.set_config(&stranger, &symbol_short!("critical"), &20, &200, &1000);
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_pause() {
        let env = Env::default();
        let (_admin, _operator, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.pause(&stranger);
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_renounce() {
        let env = Env::default();
        let (_admin, _operator, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.renounce_admin(&stranger);
    }

    /// Assert that an event has exactly 3 topics with the expected structure.
    fn assert_topic_structure(env: &Env, topics: &soroban_sdk::Vec<Symbol>, expected_name: Symbol) {
        assert_eq!(topics.len(), 3, "Event must have exactly 3 topics");

        let name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
        let version: Symbol = topics.get(1).unwrap().try_into_val(env).unwrap();
        let _context: Symbol = topics.get(2).unwrap().try_into_val(env).unwrap();

        assert_eq!(name, expected_name, "topic[0] must be the event name");
        assert_eq!(
            version, EVENT_VERSION,
            "topic[1] must be the event version v1"
        );
    }

    // ── sla_calc topic stability ────────────────────────────────────────

    #[test]
    fn test_sla_calc_topic_structure_is_stable() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("STABLE1"),
            &symbol_short!("critical"),
            &5,
        );

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_SLA_CALC {
                assert_topic_structure(&env, &topics, EVENT_SLA_CALC);

                let context: Symbol = topics.get(2).unwrap().try_into_val(&env).unwrap();
                assert_eq!(context, symbol_short!("critical"));
                return;
            }
        }
        panic!("sla_calc event not found");
    }

    #[test]
    fn test_sla_calc_topic_is_consistent_across_severities() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);
        let severities = [
            symbol_short!("critical"),
            symbol_short!("high"),
            symbol_short!("medium"),
            symbol_short!("low"),
        ];

        for (i, sev) in severities.iter().enumerate() {
            client.calculate_sla(
                &operator,
                &symbol_short!("TOPIC"),
                sev,
                &(10u32 + i as u32),
            );
        }

        let events = env.events().all();
        let mut calc_count = 0;
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_SLA_CALC {
                assert_topic_structure(&env, &topics, EVENT_SLA_CALC);
                calc_count += 1;
            }
        }
        assert_eq!(calc_count, 4, "Expected 4 sla_calc events");
    }

    // ── cfg_upd topic stability ─────────────────────────────────────────

    #[test]
    fn test_cfg_upd_topic_structure_is_stable() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.set_config(&admin, &symbol_short!("critical"), &20, &200, &1000);

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_CONFIG_UPD {
                assert_topic_structure(&env, &topics, EVENT_CONFIG_UPD);
                return;
            }
        }
        panic!("cfg_upd event not found");
    }

    // ── Pause/unpause topic stability ───────────────────────────────────

    #[test]
    fn test_pause_topic_structure_is_stable() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.pause(&admin);

        let events = env.events().all();
        let (_, topics, _) = events.last().unwrap();
        assert_topic_structure(&env, &topics, EVENT_PAUSED);
    }

    #[test]
    fn test_unpause_topic_structure_is_stable() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.pause(&admin);
        client.unpause(&admin);

        let events = env.events().all();
        let (_, topics, _) = events.last().unwrap();
        assert_topic_structure(&env, &topics, EVENT_UNPAUSED);
    }

    // ── All events use consistent topic[1] version ──────────────────────

    #[test]
    fn test_all_events_have_consistent_version_topic() {
        let env = Env::default();
        let (admin, operator, client) = setup(&env);
        let new_admin = Address::generate(&env);
        let new_op = Address::generate(&env);

        // Trigger all event types
        client.calculate_sla(
            &operator,
            &symbol_short!("ALL_EVT"),
            &symbol_short!("critical"),
            &5,
        );
        client.set_config(&admin, &symbol_short!("critical"), &20, &200, &1000);
        client.pause(&admin);
        client.unpause(&admin);
        client.propose_admin(&admin, &new_admin);
        client.cancel_admin_proposal(&admin);
        client.propose_operator(&admin, &new_op);
        client.cancel_operator_proposal(&admin);
        client.set_operator(&admin, &new_op);
        client.renounce_admin(&admin);

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() >= 2 {
                let version: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
                assert_eq!(
                    version, EVENT_VERSION,
                    "topic[1] must always be event version v1"
                );
            }
        }
    }

    // ── topic[2] (context) stability ────────────────────────────────────

    #[test]
    fn test_sla_calc_topic_context_is_severity() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("CTX1"),
            &symbol_short!("high"),
            &20,
        );

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_SLA_CALC {
                let context: Symbol = topics.get(2).unwrap().try_into_val(&env).unwrap();
                assert_eq!(
                    context, symbol_short!("high"),
                    "topic[2] must be the severity for sla_calc"
                );
                return;
            }
        }
        panic!("sla_calc event not found");
    }

    #[test]
    fn test_settle_intent_topic_context_is_severity() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("CTX2"),
            &symbol_short!("medium"),
            &30,
        );

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name == EVENT_SETTLE_INTENT {
                let context: Symbol = topics.get(2).unwrap().try_into_val(&env).unwrap();
                assert_eq!(
                    context, symbol_short!("medium"),
                    "topic[2] must be the severity for set_int"
                );
                return;
            }
        }
        panic!("set_int event not found");
    }

    // ── adm_ren topic + payload stability ──────────────────────────────

    /// Verify that the `adm_ren` event has the 3-topic layout with an Address
    /// in topic[2], and that its payload decodes as the canonical tuple
    /// `(caller: Address, ledger_seq: u32, last_admin_set_at: u64)`.
    ///
    /// This test pins ledger state so the decoded values are deterministic and
    /// any future field-order or type change surfaces as a compile / assertion
    /// failure rather than a silent backend regression.
    #[test]
    fn test_adm_ren_topic_and_payload_shape() {
        let env = Env::default();
        env.ledger().set_sequence_number(5);
        env.ledger().set_timestamp(100_000u64);

        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        // Advance ledger before renounce.
        env.ledger().set_sequence_number(77);
        env.ledger().set_timestamp(200_000u64);

        client.renounce_admin(&admin);

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, data) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name != EVENT_ADMIN_REN {
                continue;
            }

            // ── topic structure ───────────────────────────────────────────
            assert_eq!(topics.len(), 3, "adm_ren must have exactly 3 topics");

            let version: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
            assert_eq!(version, EVENT_VERSION, "topic[1] must be v1");

            // topic[2] is the caller Address (not a Symbol — verified by type decode).
            let topic_caller: Address = topics.get(2).unwrap().try_into_val(&env).unwrap();
            assert_eq!(topic_caller, admin, "topic[2] must be the renouncing admin");

            // ── payload structure ─────────────────────────────────────────
            let payload: (Address, u32, u64) = data.try_into_val(&env).unwrap();
            let (payload_caller, ledger_seq, last_admin_set_at) = payload;

            assert_eq!(payload_caller, admin, "payload[0] caller must match admin");
            // Renounce was called at sequence 77.
            assert_eq!(ledger_seq, 77u32, "payload[1] ledger_seq must be renounce seq");
            // last_admin_set_at recorded during initialize at timestamp 100_000.
            assert_eq!(
                last_admin_set_at, 100_000u64,
                "payload[2] last_admin_set_at must be initialize timestamp"
            );
            return;
        }
        panic!("adm_ren event not found");
    }
}
