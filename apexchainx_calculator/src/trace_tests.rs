//! Issue #98 — End-to-end tests demonstrating the `debug-trace` harness.
//!
//! These tests are only compiled when the `debug-trace` cargo feature is
//! enabled. Run them with:
//!
//! ```text
//! cargo test --features debug-trace trace_tests
//! ```
//!
//! Each scenario calls [`trace::reset`] first so assertions only see
//! events emitted by the current scenario (the trace is a `thread_local`
//! RefCell, so concurrent test threads stay isolated).

#![cfg(feature = "debug-trace")]

use super::*;
use soroban_sdk::symbol_short;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

struct TraceActors {
    admin: soroban_sdk::Address,
    operator: soroban_sdk::Address,
}

fn trace_setup() -> (Env, SLACalculatorContractClient<'static>, TraceActors) {
    let env = Env::default();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let operator = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &operator);
    (env, client, TraceActors { admin, operator })
}

#[test]
fn test_initialize_records_trace_event() {
    trace::reset();
    let (_env, _client, _actors) = trace_setup();

    assert!(
        trace::find_event("initialize"),
        "initialize transition must be recorded; captured: {}",
        trace::dump_jsonl()
    );
}

#[test]
fn test_set_config_records_trace_event_with_fields() {
    trace::reset();
    let (_env, client, actors) = trace_setup();

    client.set_config(&actors.admin, &symbol_short!("critical"), &20, &200, &1000);

    assert!(trace::find_event("set_config"));
    let events = trace::find_events("set_config");
    assert_eq!(events.len(), 1);
    assert!(events[0].json.contains(r#""severity":"critical""#));
    assert!(events[0].json.contains(r#""threshold_minutes":20"#));
    assert!(events[0].json.contains(r#""penalty_per_minute":200"#));
    assert!(events[0].json.contains(r#""reward_base":1000"#));
}

#[test]
fn test_set_operator_records_trace_event() {
    trace::reset();
    let (env, client, actors) = trace_setup();
    let new_op = soroban_sdk::Address::generate(&env);

    client.set_operator(&actors.admin, &new_op);

    assert!(trace::find_event("set_operator"));
}

#[test]
fn test_calculate_sla_records_trace_event_for_met_path() {
    trace::reset();
    let (_env, client, actors) = trace_setup();

    let _ = client.calculate_sla(
        &actors.operator,
        &symbol_short!("TRC_M"),
        &symbol_short!("critical"),
        &5, // well under threshold
    );

    assert!(trace::find_event("calculate_sla"));
    let events = trace::find_events("calculate_sla");
    assert_eq!(events.len(), 1);
    assert!(events[0].json.contains(r#""status":"met""#));
    assert!(events[0].json.contains(r#""payment_type":"rew""#));
    assert!(events[0].json.contains(r#""rating":"top""#));
}

#[test]
fn test_calculate_sla_records_trace_event_for_violated_path() {
    trace::reset();
    let (_env, client, actors) = trace_setup();

    let _ = client.calculate_sla(
        &actors.operator,
        &symbol_short!("TRC_V"),
        &symbol_short!("critical"),
        &25, // 10 min overtime → violation
    );

    let events = trace::find_events("calculate_sla");
    assert_eq!(events.len(), 1);
    assert!(events[0].json.contains(r#""status":"viol""#));
    assert!(events[0].json.contains(r#""payment_type":"pen""#));
    assert!(events[0].json.contains(r#""rating":"poor""#));
    assert!(events[0].json.contains("\"amount\":-1000"));
}

#[test]
fn test_pause_records_trace_event_with_escaped_reason() {
    trace::reset();
    let (env, client, actors) = trace_setup();

    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "reason-with-\"quote\""),
    );

    assert!(trace::find_event("pause"));
    let events = trace::find_events("pause");
    assert_eq!(events.len(), 1);
    // Quotes in the reason must be escaped so the JSON line is valid.
    assert!(
        events[0].json.contains(r#"\"quote\""#),
        "expected escaped quote in pause reason; got: {}",
        events[0].json
    );
}

#[test]
fn test_unpause_is_recorded() {
    trace::reset();
    let (env, client, actors) = trace_setup();
    client.pause(&actors.admin, &soroban_sdk::String::from_str(&env, "x"));
    client.unpause(&actors.admin);

    assert!(trace::find_event("pause"));
    assert!(trace::find_event("unpause"));
}

#[test]
fn test_freeze_and_unfreeze_are_recorded() {
    trace::reset();
    let (_env, client, actors) = trace_setup();
    client.freeze_config(&actors.admin);
    client.unfreeze_config(&actors.admin);

    assert!(trace::find_event("freeze_config"));
    assert!(trace::find_event("unfreeze_config"));
}

#[test]
fn test_reset_clears_only_current_thread() {
    // Independent scenario on the same thread: previous events must not leak.
    trace::reset();
    let (_env, client, actors) = trace_setup();
    // setup emits "initialize"; reset -> no events.
    trace::reset();
    client.set_config(&actors.admin, &symbol_short!("critical"), &15, &100, &750);
    assert_eq!(trace::event_count(), 1);
    assert!(trace::find_event("set_config"));
}

#[test]
fn test_complex_scenario_emits_expected_event_sequence() {
    trace::reset();
    let (env, client, actors) = trace_setup();

    // Pause, mutate config while paused is allowed at the API level,
    // unpause, then run a calc. We only assert that each expected event
    // is present at least once — ordering is not enforced by the API
    // because other tests on this thread may have emitted other events.
    client.pause(
        &actors.admin,
        &soroban_sdk::String::from_str(&env, "hold"),
    );
    client.unpause(&actors.admin);
    client.set_config(&actors.admin, &symbol_short!("high"), &40, &75, &900);
    let _ = client.calculate_sla(
        &actors.operator,
        &symbol_short!("SEQ01"),
        &symbol_short!("high"),
        &20,
    );

    assert!(trace::find_event("pause"));
    assert!(trace::find_event("unpause"));
    assert!(trace::find_event("set_config"));
    assert!(trace::find_event("calculate_sla"));

    // Dump the JSONL line-by-line; ensure each line is parseable (begins
    // with `{` and ends with `}`). This validates the wire format.
    for line in trace::dump_jsonl().lines() {
        assert!(line.starts_with('{'), "non-JSON line: {line}");
        assert!(line.ends_with('}'), "non-JSON line: {line}");
    }
}
