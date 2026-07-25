#![no_main]

use libfuzzer_sys::fuzz_target;
use apexchainx_calculator::SLACalculatorContract;
use soroban_sdk::symbol_short;

fuzz_target!(|data: (u32, u32, u32, i128, i128)| {
    let (severity_idx, threshold_minutes, penalty_per_minute, reward_base, _) = data;

    let severity = match severity_idx % 4 {
        0 => symbol_short!("critical"),
        1 => symbol_short!("high"),
        2 => symbol_short!("medium"),
        _ => symbol_short!("low"),
    };

    let result = SLACalculatorContract::validate_config(
        &severity,
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    );

    match result {
        Ok(()) => {
            // If validation passes, the config must produce a valid SLA result
            let cfg = apexchainx_calculator::SLAConfig {
                threshold_minutes,
                penalty_per_minute,
                reward_base,
            };
            let _env = soroban_sdk::Env::default();
            let res = SLACalculatorContract::compute_result(
                symbol_short!("test"),
                0,
                &cfg,
                0,
                0,
            );
            assert!(
                res.is_ok(),
                "validate_config passed but compute_result failed for valid inputs"
            );
        }
        Err(_) => {
            // Error is expected for invalid inputs
        }
    }
});
