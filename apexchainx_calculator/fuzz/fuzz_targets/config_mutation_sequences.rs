#![no_main]

//! Fuzz target for config mutation sequences (issue #388).
//!
//! Generates sequences of `set_config`, `set_custom_severity`,
//! `remove_custom_severity`, `set_retention_limit`, and `calculate_sla`
//! operations and verifies that no sequence can leave the contract in an
//! inconsistent state:
//!
//! * the canonical config snapshot always exposes its four severities in
//!   canonical order with cross-severity penalty ordering preserved,
//! * the retention limit stays within `[1, MAX_HISTORY_SIZE]`,
//! * custom severities never shadow canonical names, and
//! * history stays bounded and every entry is well-formed.
//!
//! # What this target does NOT assert
//!
//! The *values* a calculation produces. Whether a given MTTR yields the
//! documented status, rating and amount is the job of the `compute_result`
//! target, which checks every outcome against `apexchainx_calculator::spec`;
//! this one only asserts that whatever was produced is internally consistent
//! and correctly stored. Authorization, pause and freeze gating, duplicate
//! detection and event payload shape are also out of scope here.
//!
//! # If this target fails
//!
//! See `docs/FUZZING_GUARANTEES.md` § "Which statement is authoritative"
//! before changing either the implementation or the invariants above.

use apexchainx_calculator::{SLACalculatorContract, SLACalculatorContractClient};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::EnvTestConfig;
use soroban_sdk::{symbol_short, Address, Env, Symbol};

/// Maximum number of operations decoded from a single fuzz input.
const MAX_OPS: usize = 32;

const OP_SET_CONFIG: u8 = 0;
const OP_SET_CUSTOM: u8 = 1;
const OP_REMOVE_CUSTOM: u8 = 2;
const OP_SET_RETENTION: u8 = 3;
const OP_CALCULATE_SLA: u8 = 4;
const OP_COUNT: u8 = 5;

fn canonical_severity(i: u32) -> Symbol {
    match i % 4 {
        0 => symbol_short!("critical"),
        1 => symbol_short!("high"),
        2 => symbol_short!("medium"),
        _ => symbol_short!("low"),
    }
}

fn custom_severity(i: u32) -> Symbol {
    match i % 8 {
        0 => symbol_short!("cust0"),
        1 => symbol_short!("cust1"),
        2 => symbol_short!("cust2"),
        3 => symbol_short!("cust3"),
        4 => symbol_short!("cust4"),
        5 => symbol_short!("cust5"),
        6 => symbol_short!("cust6"),
        _ => symbol_short!("cust7"),
    }
}

/// Any severity the contract can evaluate: four canonical + eight custom.
fn any_severity(i: u32) -> Symbol {
    let i = i % 12;
    if i < 4 {
        canonical_severity(i)
    } else {
        custom_severity(i)
    }
}

fn outage_id(i: u32) -> Symbol {
    match i % 8 {
        0 => symbol_short!("out0"),
        1 => symbol_short!("out1"),
        2 => symbol_short!("out2"),
        3 => symbol_short!("out3"),
        4 => symbol_short!("out4"),
        5 => symbol_short!("out5"),
        6 => symbol_short!("out6"),
        _ => symbol_short!("out7"),
    }
}

fn is_canonical(s: &Symbol) -> bool {
    *s == symbol_short!("critical")
        || *s == symbol_short!("high")
        || *s == symbol_short!("medium")
        || *s == symbol_short!("low")
}

fn read_u32(data: &[u8], pos: &mut usize) -> Option<u32> {
    let end = pos.checked_add(4)?;
    if end > data.len() {
        return None;
    }
    let value = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos = end;
    Some(value)
}

/// Maps raw bytes into a contract-valid threshold (`1..=1440`).
fn threshold(raw: u32) -> u32 {
    1 + (raw % 1440)
}

/// Maps raw bytes into a contract-valid penalty (`1..=10_000`).
fn penalty(raw: u32) -> i128 {
    1 + (raw % 10_000) as i128
}

/// Maps raw bytes into a reward that is biased to satisfy the cross-parameter
/// consistency rule (`penalty * 3 < reward * 2`) so a healthy share of the
/// generated mutations actually reach storage.
fn reward(raw: u32, penalty: i128) -> i128 {
    penalty
        .saturating_mul(2)
        .saturating_add(1 + (raw % 1000) as i128)
        .min(100_000)
}

/// Maps raw bytes into a retention limit (`1..=1000`).
fn retention(raw: u32) -> u32 {
    1 + (raw % 1000)
}

/// Maps raw bytes into an MTTR that straddles the threshold boundary.
fn mttr(raw: u32) -> u32 {
    raw % 2880
}

fn check_invariants<'a>(client: &SLACalculatorContractClient<'a>) {
    // Retention limit must always stay within the enforced bounds.
    let limit = client.get_retention_limit();
    assert!(
        (1..=1000).contains(&limit),
        "retention limit {} out of bounds",
        limit
    );

    // Canonical snapshot: exactly four severities, in canonical order, with
    // valid configs and cross-severity penalty ordering preserved.
    let snapshot = client.get_config_snapshot();
    assert_eq!(
        snapshot.entries.len(),
        4,
        "canonical snapshot must always have 4 entries"
    );
    for i in 0..4u32 {
        let entry = snapshot.entries.get(i).expect("canonical entry");
        assert_eq!(entry.severity, canonical_severity(i), "canonical order violated");
        let cfg = &entry.config;
        assert!(
            (1..=1440).contains(&cfg.threshold_minutes),
            "threshold {} out of range",
            cfg.threshold_minutes
        );
        assert!(
            (1..=10_000).contains(&cfg.penalty_per_minute),
            "penalty {} out of range",
            cfg.penalty_per_minute
        );
        assert!(
            (1..=100_000).contains(&cfg.reward_base),
            "reward {} out of range",
            cfg.reward_base
        );
        assert!(
            cfg.penalty_per_minute * 3 < cfg.reward_base * 2,
            "reward/penalty consistency violated"
        );
    }
    let penalty_at = |i: u32| snapshot.entries.get(i).unwrap().config.penalty_per_minute;
    assert!(
        penalty_at(0) >= penalty_at(1) && penalty_at(1) >= penalty_at(2),
        "cross-severity penalty ordering violated"
    );
    let threshold_at = |i: u32| snapshot.entries.get(i).unwrap().config.threshold_minutes;
    assert!(
        threshold_at(0) <= threshold_at(1)
            && threshold_at(1) <= threshold_at(2)
            && threshold_at(2) <= threshold_at(3),
        "cross-severity threshold ordering violated: critical={} high={} medium={} low={}",
        threshold_at(0), threshold_at(1), threshold_at(2), threshold_at(3)
    );

    // Custom severities must never shadow a canonical name, and each custom
    // config must respect the same general bounds.
    let custom = client.get_custom_config_snapshot();
    for entry in custom.entries.iter() {
        assert!(
            !is_canonical(&entry.severity),
            "custom severity shadowed a canonical name"
        );
        let cfg = &entry.config;
        assert!((1..=1440).contains(&cfg.threshold_minutes));
        assert!((1..=10_000).contains(&cfg.penalty_per_minute));
        assert!((1..=100_000).contains(&cfg.reward_base));
        assert!(cfg.penalty_per_minute * 3 < cfg.reward_base * 2);
    }

    // History: bounded and every entry well-formed.
    let history = client.get_history();
    assert!(
        history.len() <= 1000,
        "history exceeded hard cap: {}",
        history.len()
    );
    for entry in history.iter() {
        let met = entry.status == symbol_short!("met");
        let viol = entry.status == symbol_short!("viol");
        assert!(met || viol, "invalid status symbol in history");
        if met {
            assert_eq!(entry.payment_type, symbol_short!("rew"));
            assert!(entry.amount > 0, "met entry has non-positive amount");
            assert!(
                entry.rating == symbol_short!("top")
                    || entry.rating == symbol_short!("excel")
                    || entry.rating == symbol_short!("good"),
                "invalid met rating in history"
            );
        } else {
            assert_eq!(entry.payment_type, symbol_short!("pen"));
            assert!(entry.amount < 0, "violation entry has non-negative amount");
            assert_eq!(entry.rating, symbol_short!("poor"));
        }
        assert!(
            (1..=1440).contains(&entry.threshold_minutes),
            "history threshold {} out of range",
            entry.threshold_minutes
        );
    }
}

fuzz_target!(|data: &[u8]| {
    let mut env = Env::default();
    // Each fuzz iteration creates a fresh environment; disable the per-drop
    // snapshot file so the campaign stays fast and does not litter the tree.
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env.mock_all_auths();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    client.initialize(&admin, &operator);

    let mut pos = 0usize;
    let mut ops = 0usize;
    while pos < data.len() && ops < MAX_OPS {
        let opcode = data[pos] % OP_COUNT;
        pos += 1;
        match opcode {
            OP_SET_CONFIG => {
                let (Some(sev), Some(thr), Some(pen), Some(rew)) = (
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                ) else {
                    break;
                };
                let severity = canonical_severity(sev);
                let p = penalty(pen);
                let _ = client.try_set_config(&admin, &severity, &threshold(thr), &p, &reward(rew, p));
            }
            OP_SET_CUSTOM => {
                let (Some(sev), Some(thr), Some(pen), Some(rew)) = (
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                ) else {
                    break;
                };
                let severity = custom_severity(sev);
                let p = penalty(pen);
                let _ =
                    client.try_set_custom_severity(&admin, &severity, &threshold(thr), &p, &reward(rew, p));
            }
            OP_REMOVE_CUSTOM => {
                let Some(sev) = read_u32(data, &mut pos) else {
                    break;
                };
                let _ = client.try_remove_custom_severity(&admin, &custom_severity(sev));
            }
            OP_SET_RETENTION => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                let _ = client.try_set_retention_limit(&admin, &retention(raw));
            }
            OP_CALCULATE_SLA => {
                let (Some(sev), Some(out), Some(m)) = (
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                ) else {
                    break;
                };
                let severity = any_severity(sev);
                let out = outage_id(out);
                let m = mttr(m);
                if client.try_calculate_sla(&operator, &out, &severity, &m).is_ok() {
                    // A successful calculation must respect the configured
                    // retention limit immediately.
                    let history = client.get_history();
                    let limit = client.get_retention_limit();
                    assert!(
                        history.len() <= limit,
                        "history {} exceeds retention limit {} after calculate_sla",
                        history.len(),
                        limit
                    );
                }
            }
            _ => unreachable!(),
        }
        ops += 1;
    }

    check_invariants(&client);
});
