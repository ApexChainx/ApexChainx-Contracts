#[cfg(test)]
mod event_state_tests {
    use soroban_sdk::{
        symbol_short, testutils::Address as _, testutils::Events, testutils::Ledger, Address, Env,
        Symbol, TryIntoVal,
    };
    use crate::{
        EVENT_ADMIN_REN, EVENT_CONFIG_UPD, EVENT_PRUNED, EVENT_PRUNED_AGE, EVENT_SETTLE_INTENT,
        EVENT_SLA_CALC, EVENT_VERSION, SLACalculatorContract, SLACalculatorContractClient,
        SLAConfig,
    };

    fn setup(env: &Env) -> (Address, Address, SLACalculatorContractClient) {
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        (admin, operator, client)
    }

    fn symbol(env: &Env, value: &str) -> Symbol {
        Symbol::new(env, value)
    }

    // ── sla_calc event ↔ stored history parity ──────────────────────────

    #[test]
    fn test_sla_calc_event_matches_history_entry() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("EVT_MATCH"),
            &symbol_short!("critical"),
            &10, // met case
        );

        let events = env.events().all();
        let history = client.get_history();
        assert_eq!(history.len(), 1);

        let stored = history.get(0).unwrap();

        // Find the sla_calc event
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() >= 1 {
                let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
                if name == EVENT_SLA_CALC {
                    let (_, _, data) = events.get(i).unwrap();
                    let payload: (Symbol, Symbol, Symbol, Symbol, u32, u32, i128) =
                        data.try_into_val(&env).unwrap();
                    let (outage_id, status, payment_type, rating, mttr, threshold, amount) =
                        payload;

                    // Event payload must exactly match the stored SLAResult
                    assert_eq!(outage_id, stored.outage_id);
                    assert_eq!(status, stored.status);
                    assert_eq!(payment_type, stored.payment_type);
                    assert_eq!(rating, stored.rating);
                    assert_eq!(mttr, stored.mttr_minutes);
                    assert_eq!(threshold, stored.threshold_minutes);
                    assert_eq!(amount, stored.amount);
                    return;
                }
            }
        }
        panic!("sla_calc event not found");
    }

    #[test]
    fn test_settle_intent_event_matches_stored_result() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("INTENT01"),
            &symbol_short!("high"),
            &35, // violation case
        );

        let history = client.get_history();
        assert_eq!(history.len(), 1);
        let stored = history.get(0).unwrap();

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() >= 1 {
                let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
                if name == EVENT_SETTLE_INTENT {
                    let (_, _, data) = events.get(i).unwrap();
                    let payload: (Symbol, Symbol, Symbol, i128, u64, u64) =
                        data.try_into_val(&env).unwrap();
                    let (outage_id, status, payment_type, amount, config_hash, recorded_at) =
                        payload;

                    assert_eq!(outage_id, stored.outage_id);
                    assert_eq!(status, stored.status);
                    assert_eq!(payment_type, stored.payment_type);
                    assert_eq!(amount, stored.amount);
                    assert_eq!(config_hash, stored.config_version_hash);
                    assert_eq!(recorded_at, stored.recorded_at);
                    return;
                }
            }
        }
        panic!("settle_intent event not found");
    }

    #[test]
    fn test_multiple_events_each_match_their_corresponding_history_entry() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        // First calculation
        client.calculate_sla(
            &operator,
            &symbol_short!("EVT_A"),
            &symbol_short!("critical"),
            &5,
        );
        // Second calculation
        client.calculate_sla(
            &operator,
            &symbol_short!("EVT_B"),
            &symbol_short!("low"),
            &130,
        );

        let history = client.get_history();
        assert_eq!(history.len(), 2);
        let stored_a = history.get(0).unwrap();
        let stored_b = history.get(1).unwrap();

        let mut found_a = false;
        let mut found_b = false;

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, data) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name != EVENT_SLA_CALC {
                continue;
            }
            let payload: (Symbol, Symbol, Symbol, Symbol, u32, u32, i128) =
                data.try_into_val(&env).unwrap();
            let (outage_id, status, payment_type, rating, mttr, threshold, amount) = payload;

            if outage_id == symbol_short!("EVT_A") {
                assert_eq!(status, stored_a.status);
                assert_eq!(payment_type, stored_a.payment_type);
                assert_eq!(rating, stored_a.rating);
                assert_eq!(mttr, stored_a.mttr_minutes);
                assert_eq!(threshold, stored_a.threshold_minutes);
                assert_eq!(amount, stored_a.amount);
                found_a = true;
            } else if outage_id == symbol_short!("EVT_B") {
                assert_eq!(status, stored_b.status);
                assert_eq!(payment_type, stored_b.payment_type);
                assert_eq!(rating, stored_b.rating);
                assert_eq!(mttr, stored_b.mttr_minutes);
                assert_eq!(threshold, stored_b.threshold_minutes);
                assert_eq!(amount, stored_b.amount);
                found_b = true;
            }
        }

        assert!(found_a, "Event for EVT_A not found");
        assert!(found_b, "Event for EVT_B not found");
    }

    // ── cfg_upd event ↔ stored config parity ────────────────────────────

    #[test]
    fn test_cfg_upd_event_matches_stored_config() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        let new_config = SLAConfig {
            threshold_minutes: 25,
            penalty_per_minute: 120,
            reward_base: 900,
        };

        client.set_config(
            &admin,
            &symbol_short!("critical"),
            &new_config.threshold_minutes,
            &new_config.penalty_per_minute,
            &new_config.reward_base,
        );

        let stored = client.get_config(&symbol_short!("critical"));
        assert_eq!(stored.threshold_minutes, new_config.threshold_minutes);
        assert_eq!(stored.penalty_per_minute, new_config.penalty_per_minute);
        assert_eq!(stored.reward_base, new_config.reward_base);

        // Verify event matches
        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, data) = events.get(i).unwrap();
            if topics.len() < 1 {
                continue;
            }
            let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
            if name != EVENT_CONFIG_UPD {
                continue;
            }
            let severity: Symbol = topics.get(2).unwrap().try_into_val(&env).unwrap();
            assert_eq!(severity, symbol_short!("critical"));

            let payload: (u32, i128, i128) = data.try_into_val(&env).unwrap();
            let (thresh, penalty, reward) = payload;

            assert_eq!(thresh, stored.threshold_minutes);
            assert_eq!(penalty, stored.penalty_per_minute);
            assert_eq!(reward, stored.reward_base);
            return;
        }
        panic!("cfg_upd event not found");
    }

    #[test]
    fn test_multiple_config_updates_each_have_matching_event() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        // Update critical
        client.set_config(&admin, &symbol_short!("critical"), &20, &150, &850);
        let critical_stored = client.get_config(&symbol_short!("critical"));

        // Update high
        client.set_config(&admin, &symbol_short!("high"), &40, &75, &780);
        let high_stored = client.get_config(&symbol_short!("high"));

        // Verify last event matches high config
        let events = env.events().all();
        let (_, _, data) = events.last().unwrap();
        let payload: (u32, i128, i128) = data.try_into_val(&env).unwrap();
        let (thresh, penalty, reward) = payload;

        assert_eq!(thresh, high_stored.threshold_minutes);
        assert_eq!(penalty, high_stored.penalty_per_minute);
        assert_eq!(reward, high_stored.reward_base);

        // Verify config was actually stored
        assert_eq!(critical_stored.threshold_minutes, 20);
        assert_eq!(high_stored.threshold_minutes, 40);
    }

    // ── Pause/unpause event ↔ pause state parity ────────────────────────

    #[test]
    fn test_pause_event_reflects_paused_state() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.pause(&admin);

        assert!(client.is_paused());
        assert!(client.get_pause_info().is_some());

        let events = env.events().all();
        let (_, topics, data) = events.last().unwrap();
        let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        assert_eq!(name, symbol_short!("paused"));
        let payload: (bool,) = data.try_into_val(&env).unwrap();
        assert_eq!(payload, (true,));
    }

    #[test]
    fn test_unpause_event_reflects_unpaused_state() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.pause(&admin);
        client.unpause(&admin);

        assert!(!client.is_paused());
        assert!(client.get_pause_info().is_none());

        let events = env.events().all();
        // Last event should be unpause
        let (_, topics, data) = events.last().unwrap();
        let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        assert_eq!(name, symbol_short!("unpause"));
        let payload: (bool,) = data.try_into_val(&env).unwrap();
        assert_eq!(payload, (false,));
    }

    // ── Prune event ↔ pruned state parity ───────────────────────────────

    #[test]
    fn test_prune_event_reflects_pruned_history() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        // Add 5 entries
        for _ in 0..5u32 {
            client.calculate_sla(
                &operator,
                &symbol_short!("PR_EVT"),
                &symbol_short!("low"),
                &10,
            );
        }

        assert_eq!(client.get_history().len(), 5);

        // Prune to 2
        client.prune_history(&admin, &2);
        assert_eq!(client.get_history().len(), 2);

        // Verify event
        let events = env.events().all();
        let (_, topics, data) = events.last().unwrap();
        let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        assert_eq!(name, EVENT_PRUNED);

        let payload: (u32, u32) = data.try_into_val(&env).unwrap();
        assert_eq!(payload, (3u32, 2u32)); // removed=3, kept=2
    }

    #[test]
    fn test_prune_by_age_event_reflects_pruned_history() {
        let env = Env::default();
        env.ledger().set_timestamp(1000);
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        // Add 2 entries at t=1000
        client.calculate_sla(
            &operator,
            &symbol_short!("AGE_A"),
            &symbol_short!("critical"),
            &5,
        );
        client.calculate_sla(
            &operator,
            &symbol_short!("AGE_B"),
            &symbol_short!("high"),
            &10,
        );

        // Advance time to t=3000, add 1 entry
        env.ledger().set_timestamp(3000);
        client.calculate_sla(
            &operator,
            &symbol_short!("AGE_C"),
            &symbol_short!("low"),
            &10,
        );

        assert_eq!(client.get_history().len(), 3);

        // Prune entries older than 1500 seconds (cutoff = 3000 - 1500 = 1500)
        client.prune_history_by_age(&admin, &1500);

        assert_eq!(client.get_history().len(), 1);
        assert_eq!(
            client.get_history().get(0).unwrap().outage_id,
            symbol_short!("AGE_C")
        );

        // Verify event
        let events = env.events().all();
        let (_, topics, data) = events.last().unwrap();
        let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
        assert_eq!(name, EVENT_PRUNED_AGE);

        let payload: (u32, u32) = data.try_into_val(&env).unwrap();
        assert_eq!(payload, (2u32, 1u32)); // removed=2, kept=1
    }

    // ── Stats consistency with events ───────────────────────────────────

    #[test]
    fn test_stats_match_events_after_multiple_calculations() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        // Track expected stats from events
        let mut expected_calculations = 0u64;
        let mut expected_violations = 0u64;
        let mut expected_rewards = 0i128;
        let mut expected_penalties = 0i128;

        // Violation: mttr=25, critical → penalty
        let r1 = client.calculate_sla(
            &operator,
            &symbol_short!("STAT_E1"),
            &symbol_short!("critical"),
            &25,
        );
        expected_calculations += 1;
        expected_violations += 1;
        expected_penalties += -r1.amount;

        // Met: mttr=5, critical → reward
        let r2 = client.calculate_sla(
            &operator,
            &symbol_short!("STAT_E2"),
            &symbol_short!("critical"),
            &5,
        );
        expected_calculations += 1;
        expected_rewards += r2.amount;

        // Violation: mttr=40, high → penalty
        let r3 = client.calculate_sla(
            &operator,
            &symbol_short!("STAT_E3"),
            &symbol_short!("high"),
            &40,
        );
        expected_calculations += 1;
        expected_violations += 1;
        expected_penalties += -r3.amount;

        // Met: mttr=10, high → reward
        let r4 = client.calculate_sla(
            &operator,
            &symbol_short!("STAT_E4"),
            &symbol_short!("high"),
            &10,
        );
        expected_calculations += 1;
        expected_rewards += r4.amount;

        let stats = client.get_stats();
        assert_eq!(stats.total_calculations, expected_calculations);
        assert_eq!(stats.total_violations, expected_violations);
        assert_eq!(stats.total_rewards, expected_rewards);
        assert_eq!(stats.total_penalties, expected_penalties);

        // Verify events were emitted for each calculation
        let events = env.events().all();
        let mut calc_events = 0;
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() >= 1 {
                let name: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
                if name == EVENT_SLA_CALC {
                    calc_events += 1;
                }
            }
        }
        assert_eq!(calc_events, 4, "Expected 4 sla_calc events");
    }

    // ── No phantom events for failed operations ──────────────────────────

    #[test]
    fn test_no_events_on_unauthorized_calculate() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        let before = env.events().all().len();

        // Admin cannot calculate — should fail
        let _ = client.try_calculate_sla(
            &admin,
            &symbol_short!("UNAUTH"),
            &symbol_short!("critical"),
            &5,
        );

        // No events should be emitted for failed operations
        assert_eq!(
            env.events().all().len(),
            before,
            "No events should be emitted on failed calculate_sla"
        );
    }

    #[test]
    fn test_no_events_on_invalid_config_update() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        let before = env.events().all().len();

        // Invalid config (threshold=0) should fail
        let _ = client.try_set_config(
            &admin,
            &symbol_short!("critical"),
            &0,
            &100,
            &750,
        );

        // No events should be emitted for failed config updates
        assert_eq!(
            env.events().all().len(),
            before,
            "No events should be emitted on failed set_config"
        );
    }

    // ── Event parity across view vs mutating path ────────────────────────

    #[test]
    fn test_view_path_emits_no_events_despite_same_result() {
        let env = Env::default();
        let (_, _, client) = setup(&env);

        let before = env.events().all().len();

        let _ = client.calculate_sla_view(
            &symbol_short!("VIEW_EVT"),
            &symbol_short!("critical"),
            &10,
        );

        assert_eq!(
            env.events().all().len(),
            before,
            "View path must not emit any events"
        );
    }

    // ── Event topic stability ──────────────────────────────────────────

    #[test]
    fn test_event_topic_version_is_always_v1() {
        let env = Env::default();
        let (admin, operator, client) = setup(&env);

        // Trigger various events
        client.calculate_sla(
            &operator,
            &symbol_short!("TOPIC_V"),
            &symbol_short!("critical"),
            &5,
        );
        client.set_config(&admin, &symbol_short!("critical"), &20, &200, &1000);

        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() >= 2 {
                let version: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
                assert_eq!(
                    version, EVENT_VERSION,
                    "All events must use version v1"
                );
            }
        }
    }

    // ── Auth-gated negative tests ───────────────────────────────────────

    #[test]
    #[should_panic]
    fn test_stranger_cannot_set_config() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.set_config(&stranger, &symbol_short!("critical"), &20, &200, &1000);
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_calculate_sla() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.calculate_sla(
            &stranger,
            &symbol_short!("E_U"),
            &symbol_short!("critical"),
            &5,
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
    fn test_stranger_cannot_prune_history() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        client.prune_history(&stranger, &1);
    }

    // ── adm_ren lifecycle_terminal payload shape ─────────────────────────

    /// Verify that `renounce_admin` emits an `adm_ren` event whose payload
    /// decodes as `(caller: Address, ledger_seq: u32, last_admin_set_at: u64)`.
    ///
    /// The test also asserts the three topic slot values so that a future
    /// struct-layout change surfaces as a compile or assertion failure here
    /// rather than silently at the backend.
    #[test]
    fn test_renounce_admin_event_payload_shape() {
        let env = Env::default();
        // Pin the ledger state so the assertions are deterministic.
        env.ledger().set_sequence_number(42);
        env.ledger().set_timestamp(9_000_000u64);

        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        // Advance ledger before renounce so ledger_seq differs from init seq.
        env.ledger().set_sequence_number(99);
        env.ledger().set_timestamp(9_001_000u64);

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

            // ── topic layout ──────────────────────────────────────────────
            assert_eq!(topics.len(), 3, "adm_ren must have exactly 3 topics");

            let version: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap();
            assert_eq!(version, EVENT_VERSION, "topic[1] must be event version v1");

            let topic_caller: Address = topics.get(2).unwrap().try_into_val(&env).unwrap();
            assert_eq!(topic_caller, admin, "topic[2] must be the renouncing admin");

            // ── payload layout ────────────────────────────────────────────
            // Decode as the canonical 3-tuple.
            let payload: (Address, u32, u64) = data.try_into_val(&env).unwrap();
            let (payload_caller, ledger_seq, last_admin_set_at) = payload;

            assert_eq!(payload_caller, admin, "payload[0] caller must match admin");
            // Renounce was called at sequence 99.
            assert_eq!(ledger_seq, 99u32, "payload[1] ledger_seq must be 99");
            // last_admin_set_at was recorded during initialize at timestamp 9_000_000.
            assert_eq!(
                last_admin_set_at, 9_000_000u64,
                "payload[2] last_admin_set_at must be the initialize timestamp"
            );
            return;
        }
        panic!("adm_ren event not found");
    }

    /// Verify that when admin is transferred via accept_admin, renounce reports
    /// the timestamp of the *transfer* (accept_admin), not the original init.
    #[test]
    fn test_renounce_after_transfer_reports_transfer_timestamp() {
        let env = Env::default();
        env.ledger().set_sequence_number(10);
        env.ledger().set_timestamp(1_000u64);

        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let new_admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        // Propose + accept at a later timestamp.
        env.ledger().set_sequence_number(20);
        env.ledger().set_timestamp(5_000u64);
        client.propose_admin(&admin, &new_admin);
        client.accept_admin(&new_admin);

        // Renounce at an even later sequence.
        env.ledger().set_sequence_number(30);
        env.ledger().set_timestamp(9_000u64);
        client.renounce_admin(&new_admin);

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
            let payload: (Address, u32, u64) = data.try_into_val(&env).unwrap();
            let (payload_caller, ledger_seq, last_admin_set_at) = payload;

            assert_eq!(payload_caller, new_admin, "caller must be the new admin");
            assert_eq!(ledger_seq, 30u32, "ledger_seq at renounce time");
            // accept_admin was called at timestamp 5_000 — that is when ADMIN_SET_AT_KEY
            // was last written.
            assert_eq!(
                last_admin_set_at, 5_000u64,
                "last_admin_set_at must be the accept_admin timestamp"
            );
            return;
        }
        panic!("adm_ren event not found");
    }
}
