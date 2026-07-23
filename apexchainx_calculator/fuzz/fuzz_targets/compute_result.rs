#![no_main]

use libfuzzer_sys::fuzz_target;
use apexchainx_calculator::{SLACalculatorContract, SLAConfig, SLAError};
use soroban_sdk::{symbol_short, Env};

fuzz_target!(|data: (u32, u32, u32, i128, i128)| {
    let (mttr, severity_idx, threshold_minutes, penalty_per_minute, reward_base) = data;

    let severity = match severity_idx % 4 {
        0 => symbol_short!("critical"),
        1 => symbol_short!("high"),
        2 => symbol_short!("medium"),
        _ => symbol_short!("low"),
    };

    let valid = SLACalculatorContract::validate_config(
        &severity,
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    )
    .is_ok();

    if valid {
        let cfg = SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        };

        let _env = Env::default();
        match SLACalculatorContract::compute_result(
            symbol_short!("outage"),
            mttr,
            &cfg,
            0,
            0,
        ) {
            Ok(res) => {
                assert_eq!(res.outage_id, symbol_short!("outage"));
                assert_eq!(res.threshold_minutes, threshold_minutes);

                if mttr <= threshold_minutes {
                    assert_eq!(res.status, symbol_short!("met"));
                    assert_eq!(res.payment_type, symbol_short!("rew"));
                    assert!(
                        res.amount > 0,
                        "Reward must be positive, got {}",
                        res.amount
                    );
                } else {
                    assert_eq!(res.status, symbol_short!("viol"));
                    assert_eq!(res.payment_type, symbol_short!("pen"));
                    assert!(
                        res.amount < 0,
                        "Penalty must be negative, got {}",
                        res.amount
                    );
                    assert_eq!(res.rating, symbol_short!("poor"));
                }
            }
            Err(e) => {
                let code = e as u32;
                assert!(
                    code == SLAError::InvalidPenaltyAmount as u32
                        || code == SLAError::InvalidRewardAmount as u32,
                    "unexpected error code {}",
                    code
                );
            }
        }
    }
});
