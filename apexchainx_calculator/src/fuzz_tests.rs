#![cfg(test)]

use crate::{SLACalculatorContract, SLAConfig, SLAError};
use proptest::prelude::*;
use soroban_sdk::{symbol_short, Env, Symbol};

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
