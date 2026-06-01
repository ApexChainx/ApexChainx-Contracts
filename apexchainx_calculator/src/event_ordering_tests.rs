//! SC-W5-042 – Event ordering guarantees across config and calculation operations.
//!
//! This module tests that events are emitted in a predictable, deterministic order
//! that backend consumers can rely on for correct event processing.
//!
//! Ordering guarantees:
//! 1. `cfg_upd` events precede any `sla_calc` or `set_int` events that use the
//!    updated configuration.
//! 2. `sla_calc` and `set_int` events are emitted in the same order as their
//!    corresponding `calculate_sla` calls.
//! 3. `paused` events are emitted before any operation is blocked, and `unpause`
//!    events are emitted before blocked operations resume.
//! 4. Admin transfer events (`adm_prop`, `adm_acc`, `adm_can`, `adm_ren`) are
//!    emitted in the order of the lifecycle phase they represent.
//! 5. Operator handoff events (`op_prop`, `op_acc`, `op_can`) follow the same
//!    lifecycle ordering as admin events.

#[cfg(test)]
mod event_ordering_tests {
    use soroban_sdk::{
        symbol_short, testutils::Address as _, testutils::Events, Address, Env, Symbol,
        TryIntoVal,
    };
    use crate::{
        EVENT_CONFIG_UPD, EVENT_OP_ACC, EVENT_OP_CAN, EVENT_OP_PROP, EVENT_PAUSED, EVENT_PRUNED,
        EVENT_PRUNED_AGE, EVENT_SETTLE_INTENT, EVENT_SLA_CALC, EVENT_UNPAUSED,
        SLACalculatorContract, SLACalculatorContractClient,
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

    /// Extract event names from all events in order.
    fn event_names(env: &Env) -> Vec<Symbol> {
        let mut names = Vec::new();
        let events = env.events().all();
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if topics.len() >= 1 {
                let name: Symbol = topics.get(0).unwrap().try_into_val(env).unwrap();
                names.push(name);
            }
        }
        names
    }

    /// Count occurrences of a specific event name.
    fn count_event(names: &[Symbol], target: &Symbol) -> usize {
        names.iter().filter(|n| *n == target).count()
    }

    // ── 1. cfg_upd precedes subsequent sla_calc ─────────────────────────

    #[test]
    fn test_cfg_upd_event_precedes_sla_calc() {
        let env = Env::default();
        let (admin, operator, client) = setup(&env);

        client.set_config(&admin, &symbol_short!("critical"), &20, &200, &1000);
        client.calculate_sla(
            &operator,
            &symbol_short!("ORD001"),
            &symbol_short!("critical"),
            &10,
        );

        let names = event_names(&env);
        let cfg_upd_positions: Vec<usize> = names
            .iter()
            .enumerate()
            .filter(|(_, n)| **n == EVENT_CONFIG_UPD)
            .map(|(i, _)| i)
            .collect();
        let sla_calc_positions: Vec<usize> = names
            .iter()
            .enumerate()
            .filter(|(_, n)| **n == EVENT_SLA_CALC)
            .map(|(i, _)| i)
            .collect();

        assert!(!cfg_upd_positions.is_empty(), "Expected cfg_upd event");
        assert!(!sla_calc_positions.is_empty(), "Expected sla_calc event");
        assert!(
            cfg_upd_positions[0] < sla_calc_positions[0],
            "cfg_upd event must precede sla_calc event"
        );
    }

    // ── 2. sla_calc and set_int order matches call order ─────────────────

    #[test]
    fn test_sla_calc_and_set_int_emit_in_call_order() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        client.calculate_sla(
            &operator,
            &symbol_short!("ORD_A"),
            &symbol_short!("critical"),
            &5,
        );
        client.calculate_sla(
            &operator,
            &symbol_short!("ORD_B"),
            &symbol_short!("high"),
            &35,
        );
        client.calculate_sla(
            &operator,
            &symbol_short!("ORD_C"),
            &symbol_short!("low"),
            &60,
        );

        let names = event_names(&env);

        // Collect sla_calc and set_int positions in order of emission
        let mut sla_positions: Vec<usize> = Vec::new();
        let mut set_int_positions: Vec<usize> = Vec::new();
        for (i, name) in names.iter().enumerate() {
            if *name == EVENT_SLA_CALC {
                sla_positions.push(i);
            }
            if *name == EVENT_SETTLE_INTENT {
                set_int_positions.push(i);
            }
        }

        assert_eq!(sla_positions.len(), 3, "Expected 3 sla_calc events");
        assert_eq!(set_int_positions.len(), 3, "Expected 3 set_int events");

        // Verify sla_calc and set_int appear in the same relative order
        assert!(
            sla_positions[0] < sla_positions[1],
            "sla_calc events must maintain call order"
        );
        assert!(
            sla_positions[1] < sla_positions[2],
            "sla_calc events must maintain call order"
        );

        // Each sla_calc should be immediately or closely followed by a set_int
        for i in 0..3 {
            assert!(
                sla_positions[i] < set_int_positions[i],
                "set_int must follow sla_calc for call {}",
                i
            );
        }
    }

    // ── 3. Pause/unpause ordering ───────────────────────────────────────

    #[test]
    fn test_pause_and_unpause_events_in_correct_order() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.pause(&admin);
        client.unpause(&admin);

        let names = event_names(&env);

        let pause_pos: Vec<usize> = names
            .iter()
            .enumerate()
            .filter(|(_, n)| **n == EVENT_PAUSED)
            .map(|(i, _)| i)
            .collect();
        let unpause_pos: Vec<usize> = names
            .iter()
            .enumerate()
            .filter(|(_, n)| **n == EVENT_UNPAUSED)
            .map(|(i, _)| i)
            .collect();

        assert_eq!(pause_pos.len(), 1, "Expected 1 paused event");
        assert_eq!(unpause_pos.len(), 1, "Expected 1 unpause event");
        assert!(
            pause_pos[0] < unpause_pos[0],
            "paused event must precede unpause event"
        );
    }

    // ── 4. Calculation count matches event count ────────────────────────

    #[test]
    fn test_each_calculation_produces_exactly_two_events() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);

        let count = 5;
        for i in 0..count {
            client.calculate_sla(
                &operator,
                &symbol_short!("CNT"),
                &symbol_short!("critical"),
                &(5u32 + i),
            );
        }

        let names = event_names(&env);
        let sla_calc_count = count_event(&names, &EVENT_SLA_CALC);
        let set_int_count = count_event(&names, &EVENT_SETTLE_INTENT);

        assert_eq!(
            sla_calc_count, count as usize,
            "Expected {} sla_calc events",
            count
        );
        assert_eq!(
            set_int_count, count as usize,
            "Expected {} set_int events",
            count
        );
    }

    // ── 5. Event count consistency after multi-op sequences ─────────────

    #[test]
    fn test_event_order_in_mixed_operation_sequence() {
        let env = Env::default();
        let (admin, operator, client) = setup(&env);

        // Mix of operations
        client.set_config(&admin, &symbol_short!("low"), &240, &15, &900);
        client.calculate_sla(
            &operator,
            &symbol_short!("MIX_A"),
            &symbol_short!("low"),
            &100,
        );
        client.set_config(&admin, &symbol_short!("critical"), &25, &150, &850);
        client.calculate_sla(
            &operator,
            &symbol_short!("MIX_B"),
            &symbol_short!("critical"),
            &20,
        );

        let names = event_names(&env);

        // Expected: cfg_upd, sla_calc, set_int, cfg_upd, sla_calc, set_int
        // (plus init events which we ignore by scanning only named events)
        let named: Vec<&Symbol> = names
            .iter()
            .filter(|n| {
                **n == EVENT_CONFIG_UPD
                    || **n == EVENT_SLA_CALC
                    || **n == EVENT_SETTLE_INTENT
            })
            .collect();

        assert_eq!(named.len(), 6, "Expected 6 named events in mixed sequence");
        assert_eq!(*named[0], EVENT_CONFIG_UPD, "First event must be cfg_upd");
        assert_eq!(*named[1], EVENT_SLA_CALC, "Second event must be sla_calc");
        assert_eq!(*named[2], EVENT_SETTLE_INTENT, "Third event must be set_int");
        assert_eq!(
            *named[3], EVENT_CONFIG_UPD,
            "Fourth event must be cfg_upd"
        );
        assert_eq!(*named[4], EVENT_SLA_CALC, "Fifth event must be sla_calc");
        assert_eq!(
            *named[5], EVENT_SETTLE_INTENT,
            "Sixth event must be set_int"
        );
    }
}
