//! Threshold / config parameter fixtures.
//!
//! [`default_configs`] mirrors the contract's initial per-severity
//! configuration; [`strict_configs`] and [`lenient_configs`] halve and double
//! every threshold for scenario tests. All fixtures are plain data.

use alloc::{string::String, vec::Vec};

/// Per-severity SLA configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdConfig {
    pub severity: String,
    pub threshold_minutes: u32,
    pub penalty_per_minute: i128,
    pub reward_base: i128,
}

fn config(severity: &str, threshold: u32, penalty: i128, reward: i128) -> ThresholdConfig {
    ThresholdConfig {
        severity: String::from(severity),
        threshold_minutes: threshold,
        penalty_per_minute: penalty,
        reward_base: reward,
    }
}

/// The canonical configuration, most severe first.
pub fn default_configs() -> Vec<ThresholdConfig> {
    alloc::vec![
        config("critical", 15, 100, 750),
        config("high", 30, 50, 750),
        config("medium", 60, 25, 750),
        config("low", 120, 10, 600),
    ]
}

/// [`default_configs`] with every threshold halved (rounded down, minimum 1).
pub fn strict_configs() -> Vec<ThresholdConfig> {
    default_configs()
        .into_iter()
        .map(|mut c| {
            c.threshold_minutes = (c.threshold_minutes / 2).max(1);
            c
        })
        .collect()
}

/// [`default_configs`] with every threshold doubled (saturating).
pub fn lenient_configs() -> Vec<ThresholdConfig> {
    default_configs()
        .into_iter()
        .map(|mut c| {
            c.threshold_minutes = c.threshold_minutes.saturating_mul(2);
            c
        })
        .collect()
}

/// Look up one severity in `configs`.
pub fn config_for<'a>(configs: &'a [ThresholdConfig], severity: &str) -> Option<&'a ThresholdConfig> {
    configs.iter().find(|c| c.severity == severity)
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn variants_scale_the_defaults() {
        let d = default_configs();
        assert_eq!(config_for(&d, "critical").unwrap().threshold_minutes, 15);
        assert_eq!(config_for(&strict_configs(), "critical").unwrap().threshold_minutes, 7);
        assert_eq!(config_for(&lenient_configs(), "low").unwrap().threshold_minutes, 240);
        assert!(config_for(&d, "unknown").is_none());
    }
}
