#![cfg(test)]

use crate::{SLACalculatorContract, SLACalculatorContractClient, SLAConfig, SLAError};
use proptest::prelude::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::EnvTestConfig;
use soroban_sdk::{symbol_short, Address, Env, Symbol};

// Helper to check if a config is valid for a given severity.
fn is_config_valid(
    severity: &Symbol,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) -> bool {
    SLACalculatorContract::validate_config(severity, threshold_minutes, penalty_per_minute, reward_base)
        .is_ok()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn test_fuzz_compute_result_invariants(
        mttr in 0..u32::MAX,
        severity_idx in 0..4u8,
        threshold_minutes in 0..2000u32,
        penalty_per_minute in -100..20000i128,
        reward_base in -100..200000i128,
    ) {
        let _env = Env::default();
        let severity = match severity_idx {
            0 => symbol_short!("critical"),
            1 => symbol_short!("high"),
            2 => symbol_short!("medium"),
            _ => symbol_short!("low"),
        };

        let valid = is_config_valid(&severity, threshold_minutes, penalty_per_minute, reward_base);

        if valid {
            let cfg = SLAConfig {
                threshold_minutes,
                penalty_per_minute,
                reward_base,
            };

            let res_result = SLACalculatorContract::compute_result(
                symbol_short!("outage"),
                mttr,
                &cfg,
                0,
                0,
            );

            // If config is valid under validate_config, compute_result should always succeed
            // and satisfy the invariants.
            let res = res_result.expect("Valid configuration must succeed computing SLA result");

            assert_eq!(res.outage_id, symbol_short!("outage"));
            assert_eq!(res.threshold_minutes, threshold_minutes);

            if mttr <= threshold_minutes {
                // Case 2: SLA met -> reward
                assert_eq!(res.status, symbol_short!("met"));
                assert_eq!(res.payment_type, symbol_short!("rew"));
                assert!(res.amount > 0, "Reward amount must be positive, got {}", res.amount);

                // Reward scaling check
                // base * multiplier / 100
                // multiplier is 200, 150, or 100
                let performance_ratio = (mttr * 100).checked_div(threshold_minutes).unwrap_or(0);
                let expected_multiplier = if performance_ratio < 50 {
                    200u32
                } else if performance_ratio < 75 {
                    150u32
                } else {
                    100u32
                };
                let expected_reward = reward_base
                    .saturating_mul(expected_multiplier as i128)
                    .div_euclid(100);
                assert_eq!(res.amount, expected_reward);

                // Rating check
                let expected_rating = if performance_ratio < 50 {
                    symbol_short!("top")
                } else if performance_ratio < 75 {
                    symbol_short!("excel")
                } else {
                    symbol_short!("good")
                };
                assert_eq!(res.rating, expected_rating);
            } else {
                // Case 1: SLA violated -> penalty
                assert_eq!(res.status, symbol_short!("viol"));
                assert_eq!(res.payment_type, symbol_short!("pen"));
                assert!(res.amount < 0, "Penalty amount must be negative, got {}", res.amount);
                assert_eq!(res.rating, symbol_short!("poor"));

                let overtime = (mttr - threshold_minutes) as i128;
                let expected_penalty = overtime.saturating_mul(penalty_per_minute);
                assert_eq!(res.amount, -expected_penalty);
            }
        }
    }

    #[test]
    fn test_fuzz_compute_result_monotonicity(
        mttr1 in 0..u32::MAX,
        delta in 1..200000u32, // delta > 0
        severity_idx in 0..4u8,
        threshold_minutes in 0..2000u32,
        penalty_per_minute in -100..20000i128,
        reward_base in -100..200000i128,
    ) {
        let mttr2 = mttr1.saturating_add(delta);
        if mttr1 == mttr2 {
            return Ok(()); // avoid saturated values where mttr1 == mttr2
        }

        let _env = Env::default();
        let severity = match severity_idx {
            0 => symbol_short!("critical"),
            1 => symbol_short!("high"),
            2 => symbol_short!("medium"),
            _ => symbol_short!("low"),
        };

        let valid = is_config_valid(&severity, threshold_minutes, penalty_per_minute, reward_base);

        if valid {
            let cfg = SLAConfig {
                threshold_minutes,
                penalty_per_minute,
                reward_base,
            };

            let res1 = SLACalculatorContract::compute_result(
                symbol_short!("outage"),
                mttr1,
                &cfg,
                0,
                0,
            );
            let res2 = SLACalculatorContract::compute_result(
                symbol_short!("outage"),
                mttr2,
                &cfg,
                0,
                0,
            );

            if let (Ok(r1), Ok(r2)) = (res1, res2) {
                assert!(
                    r1.amount >= r2.amount,
                    "Monotonicity violated: amount for mttr1={} is {}, but for mttr2={} is {} (cfg threshold={}, penalty={}, reward={})",
                    mttr1, r1.amount, mttr2, r2.amount, threshold_minutes, penalty_per_minute, reward_base
                );
            }
        }
    }

    /// SC-W5-047: an overflowing penalty (mttr near u32::MAX combined with a
    /// huge penalty_per_minute) must NEVER silently collapse to amount == 0.
    /// It must instead surface via a deterministic error code
    /// (InvalidPenaltyAmount / InvalidRewardAmount).
    #[test]
    fn test_fuzz_compute_result_never_silent_zero(
        mttr in (u32::MAX - 1_000_000)..=u32::MAX,
        threshold_minutes in 0..1000u32,
        penalty_per_minute in (i128::MAX / 2)..=i128::MAX,
        reward_base in (i128::MAX / 2)..=i128::MAX,
    ) {
        let _env = Env::default();
        let cfg = SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        };

        match SLACalculatorContract::compute_result(symbol_short!("outage"), mttr, &cfg, 0, 0) {
            Ok(res) => {
                // No silent saturation: a successful result must carry a non-zero amount.
                prop_assert!(
                    res.amount != 0,
                    "compute_result silently produced amount == 0 (mttr={}, threshold={}, penalty={}, reward={})",
                    mttr, threshold_minutes, penalty_per_minute, reward_base
                );
            }
            Err(e) => {
                // Overflow must be exposed via a deterministic error code.
                let code = e as u32;
                prop_assert!(
                    code == SLAError::InvalidPenaltyAmount as u32
                        || code == SLAError::InvalidRewardAmount as u32,
                    "unexpected error code {} for overflowing inputs (mttr={}, threshold={}, penalty={}, reward={})",
                    code, mttr, threshold_minutes, penalty_per_minute, reward_base
                );
            }
        }
    }

    #[test]
    fn test_fuzz_compute_result_no_panic(
        mttr in 0..u32::MAX,
        threshold_minutes in 0..u32::MAX,
        penalty_per_minute in i128::MIN..=i128::MAX,
        reward_base in i128::MIN..=i128::MAX,
    ) {
        let _env = Env::default();
        let cfg = SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        };

        // This call must not panic under any circumstances.
        let _ = SLACalculatorContract::compute_result(
            symbol_short!("outage"),
            mttr,
            &cfg,
            0,
            0,
        );
    }
}

// ---------------------------------------------------------------------------
// Config mutation sequence fuzz (issue #388)
//
// Mirrors the cargo-fuzz target at fuzz/fuzz_targets/config_mutation_sequences.rs
// so the same invariants are exercised on stable CI (`cargo test --lib fuzz_tests::`)
// without requiring a nightly toolchain. Each case decodes a byte string into a
// sequence of config mutations and verifies the contract never ends up in an
// inconsistent state.
// ---------------------------------------------------------------------------

const MAX_CONFIG_OPS: usize = 24;

const OP_SET_CONFIG: u8 = 0;
const OP_SET_CUSTOM: u8 = 1;
const OP_REMOVE_CUSTOM: u8 = 2;
const OP_SET_RETENTION: u8 = 3;
const OP_CALCULATE_SLA: u8 = 4;
const OP_COUNT: u8 = 5;

fn config_canonical_severity(i: u32) -> Symbol {
    match i % 4 {
        0 => symbol_short!("critical"),
        1 => symbol_short!("high"),
        2 => symbol_short!("medium"),
        _ => symbol_short!("low"),
    }
}

fn config_custom_severity(i: u32) -> Symbol {
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

fn config_any_severity(i: u32) -> Symbol {
    let i = i % 12;
    if i < 4 {
        config_canonical_severity(i)
    } else {
        config_custom_severity(i)
    }
}

fn config_outage_id(i: u32) -> Symbol {
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

fn config_is_canonical(s: &Symbol) -> bool {
    *s == symbol_short!("critical")
        || *s == symbol_short!("high")
        || *s == symbol_short!("medium")
        || *s == symbol_short!("low")
}

fn config_read_u32(data: &[u8], pos: &mut usize) -> Option<u32> {
    let end = pos.checked_add(4)?;
    if end > data.len() {
        return None;
    }
    let value = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos = end;
    Some(value)
}

fn config_threshold(raw: u32) -> u32 {
    1 + (raw % 1440)
}

fn config_penalty(raw: u32) -> i128 {
    1 + (raw % 10_000) as i128
}

fn config_reward(raw: u32, penalty: i128) -> i128 {
    penalty
        .saturating_mul(2)
        .saturating_add(1 + (raw % 1000) as i128)
        .min(100_000)
}

fn config_retention(raw: u32) -> u32 {
    1 + (raw % 1000)
}

fn config_mttr(raw: u32) -> u32 {
    raw % 2880
}

fn config_apply_ops<'a>(
    client: &SLACalculatorContractClient<'a>,
    admin: &Address,
    operator: &Address,
    data: &[u8],
) {
    let mut pos = 0usize;
    let mut ops = 0usize;
    while pos < data.len() && ops < MAX_CONFIG_OPS {
        let opcode = data[pos] % OP_COUNT;
        pos += 1;
        match opcode {
            OP_SET_CONFIG => {
                let (Some(sev), Some(thr), Some(pen), Some(rew)) = (
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                ) else {
                    break;
                };
                let severity = config_canonical_severity(sev);
                let p = config_penalty(pen);
                let _ = client.try_set_config(
                    admin,
                    &severity,
                    &config_threshold(thr),
                    &p,
                    &config_reward(rew, p),
                );
            }
            OP_SET_CUSTOM => {
                let (Some(sev), Some(thr), Some(pen), Some(rew)) = (
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                ) else {
                    break;
                };
                let severity = config_custom_severity(sev);
                let p = config_penalty(pen);
                let _ = client.try_set_custom_severity(
                    admin,
                    &severity,
                    &config_threshold(thr),
                    &p,
                    &config_reward(rew, p),
                );
            }
            OP_REMOVE_CUSTOM => {
                let Some(sev) = config_read_u32(data, &mut pos) else {
                    break;
                };
                let _ = client.try_remove_custom_severity(admin, &config_custom_severity(sev));
            }
            OP_SET_RETENTION => {
                let Some(raw) = config_read_u32(data, &mut pos) else {
                    break;
                };
                let _ = client.try_set_retention_limit(admin, &config_retention(raw));
            }
            OP_CALCULATE_SLA => {
                let (Some(sev), Some(out), Some(m)) = (
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                    config_read_u32(data, &mut pos),
                ) else {
                    break;
                };
                let severity = config_any_severity(sev);
                let out = config_outage_id(out);
                let m = config_mttr(m);
                if client.try_calculate_sla(operator, &out, &severity, &m).is_ok() {
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
}

fn config_check_invariants<'a>(client: &SLACalculatorContractClient<'a>) {
    let limit = client.get_retention_limit();
    assert!(
        (1..=1000).contains(&limit),
        "retention limit {} out of bounds",
        limit
    );

    let snapshot = client.get_config_snapshot();
    assert_eq!(
        snapshot.entries.len(),
        4,
        "canonical snapshot must always have 4 entries"
    );
    for i in 0..4u32 {
        let entry = snapshot.entries.get(i).expect("canonical entry");
        assert_eq!(
            entry.severity,
            config_canonical_severity(i),
            "canonical order violated"
        );
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
        threshold_at(0),
        threshold_at(1),
        threshold_at(2),
        threshold_at(3)
    );

    let custom = client.get_custom_config_snapshot();
    for entry in custom.entries.iter() {
        assert!(
            !config_is_canonical(&entry.severity),
            "custom severity shadowed a canonical name"
        );
        let cfg = &entry.config;
        assert!((1..=1440).contains(&cfg.threshold_minutes));
        assert!((1..=10_000).contains(&cfg.penalty_per_minute));
        assert!((1..=100_000).contains(&cfg.reward_base));
        assert!(cfg.penalty_per_minute * 3 < cfg.reward_base * 2);
    }

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

proptest! {
    // Each case stands up a fresh contract; snapshot capture is disabled so this
    // runs quickly. The cargo-fuzz target runs the full 10,000-iteration campaign.
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn test_fuzz_config_mutation_sequences_invariants(
        bytes in prop::collection::vec(any::<u8>(), 0..=200)
    ) {
        let mut env = Env::default();
        // This test stands up thousands of environments; disable the per-drop
        // snapshot file so the run stays fast and does not litter the tree.
        env.set_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        });
        env.mock_all_auths();
        let cid = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &cid);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);

        config_apply_ops(&client, &admin, &operator, &bytes);
        config_check_invariants(&client);
    }
}
