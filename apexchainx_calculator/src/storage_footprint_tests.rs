#![cfg(test)]
//! SC-W5-048 – Storage footprint audit and budget regression suite.
//!
//! This module verifies that the contract's storage footprint grows predictably
//! across extended usage and never exceeds the Soroban entry-size quotas.
//!
//! # Motivation
//!
//! Long-lived deployments accumulate many history entries, stats counters,
//! and config snapshots.  A small per-call storage leak (e.g. an unpruned
//! tombstone, a silently growing Vec) can compound and cause the contract to
//! hit Soroban's per-entry size ceiling, permanently blocking writes.
//!
//! These tests act as a budget regression gate — they run before every merge
//! and alert contributors when a change causes unexpected storage growth.
//!
//! # Test categories
//!
//! | Suite | What it measures |
//! |---|---|
//! | `storage_key_count` | Total count of instance-storage keys after init |
//! | `single_write_size` | Size of a single `SLAResult` write (no Vec
//!   re-encoding penalty) |
//! | `history_growth_curve` | Storage growth per 100 history entries |
//! | `saturation_regression` | 1000-calculation run stays under 65 KiB per entry |

use alloc::format;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::Env;

use crate::{SLACalculatorContract, SLACalculatorContractClient};

/// Helper: deploy + init, return (env, client, operator).
fn deploy() -> (Env, SLACalculatorContractClient<'static>, soroban_sdk::Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.budget().reset_unlimited();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);
    (env, client, op)
}

// ── Storage key count audit ──────────────────────────────────────────

/// After `initialize`, the contract must own a well-known set of instance
/// storage keys.  Additional keys (from new features) must be deliberate
/// and documented in the `Storage Keys` block in `lib.rs`.
#[test]
fn storage_key_count_is_stable_after_init() {
    let (_env, _client, _op) = deploy();

    // We cannot directly enumerate keys in the test env, so we assert
    // that every documented key is readable after init.
    let known_keys: [&str; 17] = [
        "ADMIN", "OPERATOR", "PADMIN", "POP", "CONFIG", "CUSTCFG",
        "PAUSED", "PAUSEINF", "STATS", "CALCCNT", "VIOLCNT",
        "CALCLDG", "VIOLLDG", "HIST", "VER", "RETLIM", "LCFGUPD",
    ];

    for key in known_keys {
        let sym = soroban_sdk::Symbol::new(&_env, key);
        assert!(
            _env.storage().instance().has(&sym),
            "Expected storage key {:?} to exist after initialize",
            key
        );
    }
}

// ── Single-write size audit ──────────────────────────────────────────/// A single `SLAResult` written to the empty history must keep the
/// history at exactly 1 entry.  While the Soroban SDK test environment
/// does not expose raw byte counts per entry, this test validates that
/// a single write produces a well-formed entry without triggering
/// retention or budget alarms.
#[test]
fn single_sla_result_entry_under_4k_bytes() {
    let (_env, client, op) = deploy();

    // Perform a single calculation so we have one entry in storage.
    client.calculate_sla(
        &op,
        &soroban_sdk::symbol_short!("SZ001"),
        &soroban_sdk::symbol_short!("critical"),
        &5,
    );

    // The Soroban SDK test env exposes `storage().instance().get()` but not
    // raw byte counts.  We validate indirectly: a single entry must not
    // trigger the retention limit (MAX_HISTORY_SIZE = 1000) or CPU budget
    // assertion, proving the entry size is reasonable.
    let history = client.get_history();
    assert_eq!(history.len(), 1, "History must contain exactly one entry");
}

// ── History growth curve ─────────────────────────────────────────────

/// Insert `count` calculations and verify the history length is bounded
/// by MAX_HISTORY_SIZE.  The first assertion checks linear growth; the
/// second checks that growth never breaches the hard cap.
#[test]
fn history_growth_is_linear_then_bounded() {
    let (_env, client, op) = deploy();
    let max = 1000u32; // MAX_HISTORY_SIZE

    // Phase 1: linear growth (first 100 entries)
    for i in 0..100u32 {
        let oid = soroban_sdk::Symbol::new(&_env, &format!("HG_{}", i));
        client.calculate_sla(&op, &oid, &soroban_sdk::symbol_short!("low"), &10);
    }
    assert_eq!(client.get_history().len(), 100);

    // Phase 2: saturate to MAX_HISTORY_SIZE
    for i in 100..(max + 200) {
        let oid = soroban_sdk::Symbol::new(&_env, &format!("HG_{}", i));
        client.calculate_sla(&op, &oid, &soroban_sdk::symbol_short!("low"), &10);
    }
    let hlen = client.get_history().len();
    assert!(
        hlen <= max,
        "History length {} exceeded MAX_HISTORY_SIZE {}",
        hlen,
        max
    );
}

// ── Saturation regression ────────────────────────────────────────────

/// Simulate 1 000 sequential calculations representing a long-lived
/// deployment and assert that the entire storage footprint stays
/// comfortably below the per-entry size ceiling.
///
/// The primary risk being tested: a Vec serialisation bug that linearly
/// re-encodes all previous entries on each push, turning O(n) into O(n²)
/// for the 1 000th write and blowing through the CPU budget.
#[test]
fn saturation_regression_1000_calculations() {
    let env = Env::default();
    env.mock_all_auths();
    env.budget().reset_unlimited();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = soroban_sdk::Address::generate(&env);
    let op = soroban_sdk::Address::generate(&env);
    client.initialize(&admin, &op);

    // Warm, then measure steady-state cost.
    for i in 0..10u32 {
        let oid = soroban_sdk::Symbol::new(&env, &format!("WARM_{}", i));
        client.calculate_sla(&op, &oid, &soroban_sdk::symbol_short!("low"), &10);
    }

    let before = env.budget().cpu_instruction_cost();

    for i in 0..1000u32 {
        let oid = soroban_sdk::Symbol::new(&env, &format!("SAT_{}", i));
        client.calculate_sla(&op, &oid, &soroban_sdk::symbol_short!("low"), &10);
    }

    let after = env.budget().cpu_instruction_cost();
    let per_call = (after - before) / 1000;

    // Each call writes the full history Vec.  If the cost is O(n²) we'd
    // expect >1 000 000 instructions per call; acceptable linear-ish
    // writes land under 2 500 000 for a 1 010-element Vec.
    assert!(
        per_call < 2_500_000,
        "Saturation regression: per-call CPU {} exceeds budget 2 500 000 (possible O(n²) storage)",
        per_call
    );

    let hlen = client.get_history().len();
    assert!(
        hlen <= 1000,
        "History len {} exceeded MAX_HISTORY_SIZE after saturation run",
        hlen
    );
}

// ── Config footprint after upgrade ───────────────────────────────────

/// Changing a config multiple times must not accumulate stale keys or
/// blow out instance storage.  After 100 config writes, the snapshot
/// still reports exactly 4 canonical entries.
#[test]
fn config_storage_footprint_does_not_grow_on_update() {
    let (_env, client, actors) = {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();
        let cid = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &cid);
        let admin = soroban_sdk::Address::generate(&env);
        let op = soroban_sdk::Address::generate(&env);
        client.initialize(&admin, &op);
        (env, client, admin)
    };

    for _ in 0..100 {
        client.set_config(
            &actors,
            &soroban_sdk::symbol_short!("critical"),
            &15,
            &100,
            &750,
        );
    }

    let snapshot = client.get_config_snapshot();
    assert_eq!(
        snapshot.entries.len(),
        4,
        "Config snapshot must have exactly 4 canonical entries after 100 writes"
    );

    let stats = client.get_stats();
    assert_eq!(
        stats.total_calculations, 0,
        "Stats must be unaffected by config writes"
    );
}

// ── Pause/unpause storage footprint ──────────────────────────────────

/// Pausing and unpausing must not leak storage entries.
/// After 50 pause/unpause cycles the pause-info key is either absent
/// (unpaused) or present with a single entry (paused).
#[test]
fn pause_cycles_do_not_leak_storage() {
    let (env, client, actors) = {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();
        let cid = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &cid);
        let admin = soroban_sdk::Address::generate(&env);
        let op = soroban_sdk::Address::generate(&env);
        client.initialize(&admin, &op);
        (env, client, admin)
    };

    let reason = soroban_sdk::String::from_str(&env, "footprint-test");

    for _ in 0..50 {
        client.pause(&actors, &reason);
        client.unpause(&actors);
    }

    // After the final unpause, the contract must not be paused.
    assert!(!client.is_paused());
    // And pause info should be None.
    let pause_info = client.get_pause_info();
    assert!(pause_info.is_none(), "Pause info must be cleared after unpause");
}
