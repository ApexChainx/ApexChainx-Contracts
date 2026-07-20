//! SLA history snapshot analysis and normalization utilities.
//!
//! This module provides analytical utilities for inspecting the on-chain SLA
//! calculation history. The `NormalizedSnapshot` struct summarizes the history
//! in a form that backend consumers can use for dashboards and alerting.
//!
//! # Usage
//!
//! ```ignore
//! let history = contract.get_history();
//! let snapshot = normalize_history(&history);
//! println!("Total entries: {}", snapshot.count);
//! println!("Has violations: {}", snapshot.has_violations);
//! ```
//!
//! The normalization is deterministic: identical history inputs always produce
//! identical snapshot outputs.

use crate::SLAResult;
use soroban_sdk::{symbol_short, Vec};

/// Summarised view of SLA calculation history.
///
/// Provides a lightweight aggregate of the full history without exposing
/// individual record details. Suitable for dashboard telemetry.
pub struct NormalizedSnapshot {
    /// Total number of SLA calculation entries in the history.
    pub count: u32,
    /// Whether any entry has a "viol" (violated) status.
    pub has_violations: bool,
    /// Whether any entry has a "rew" (reward) payment type.
    pub has_rewards: bool,
}

/// Scans the full history and produces a `NormalizedSnapshot`.
///
/// Iterates through all history entries once, checking each for violation
/// status and reward payment type. Runtime is O(n) in the history size.
pub fn normalize_history(history: &Vec<SLAResult>) -> NormalizedSnapshot {
    let mut has_violations = false;
    let mut has_rewards = false;

    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.status == symbol_short!("viol") {
            has_violations = true;
        }
        if entry.payment_type == symbol_short!("rew") {
            has_rewards = true;
        }
    }

    NormalizedSnapshot {
        count: history.len(),
        has_violations,
        has_rewards,
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_history;
    use crate::{SLACalculatorContract, SLACalculatorContractClient};
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

    fn setup() -> (Env, SLACalculatorContractClient<'static>, Address, Address) {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        client.initialize(&admin, &operator);
        (env, client, admin, operator)
    }

    #[test]
    fn test_history_snapshot_is_deterministic() {
        let (_env, client, _admin, operator) = setup();
        client.calculate_sla(&operator, &symbol_short!("OUT1"), &symbol_short!("high"), &10);
        client.calculate_sla(&operator, &symbol_short!("OUT2"), &symbol_short!("high"), &10);
        let stats = client.get_stats();
        assert_eq!(stats.total_calculations, 2);
    }

    #[test]
    #[should_panic]
    fn test_stranger_cannot_calculate_sla() {
        let (env, client, _admin, _operator) = setup();
        let stranger = Address::generate(&env);
        // stranger does not hold the operator role
        client.calculate_sla(&stranger, &symbol_short!("U_OUT"), &symbol_short!("high"), &10);
    }

    /// Empty history: both flags false, count zero.
    #[test]
    fn test_normalize_history_empty() {
        let (_env, client, _admin, _operator) = setup();
        let history = client.get_history();
        let snap = normalize_history(&history);
        assert_eq!(snap.count, 0);
        assert!(!snap.has_violations);
        assert!(!snap.has_rewards);
    }

    /// Only SLA-met (reward) entries: has_rewards true, has_violations false.
    #[test]
    fn test_normalize_history_has_rewards_only() {
        let (_env, client, _admin, operator) = setup();
        // high threshold = 30 min; MTTR 10 < 30 → met + rew
        client.calculate_sla(&operator, &symbol_short!("R1"), &symbol_short!("high"), &10);
        client.calculate_sla(&operator, &symbol_short!("R2"), &symbol_short!("high"), &5);
        let history = client.get_history();
        let snap = normalize_history(&history);
        assert_eq!(snap.count, 2);
        assert!(snap.has_rewards);
        assert!(!snap.has_violations);
    }

    /// Only SLA-violated (penalty) entries: has_violations true, has_rewards false.
    #[test]
    fn test_normalize_history_has_violations_only() {
        let (_env, client, _admin, operator) = setup();
        // high threshold = 30 min; MTTR 60 > 30 → viol + pen
        client.calculate_sla(&operator, &symbol_short!("V1"), &symbol_short!("high"), &60);
        client.calculate_sla(&operator, &symbol_short!("V2"), &symbol_short!("high"), &90);
        let history = client.get_history();
        let snap = normalize_history(&history);
        assert_eq!(snap.count, 2);
        assert!(snap.has_violations);
        assert!(!snap.has_rewards);
    }

    /// Mixed history: both flags true.
    #[test]
    fn test_normalize_history_has_both_flags() {
        let (_env, client, _admin, operator) = setup();
        // met entry
        client.calculate_sla(&operator, &symbol_short!("M1"), &symbol_short!("high"), &10);
        // violated entry
        client.calculate_sla(&operator, &symbol_short!("V1"), &symbol_short!("high"), &60);
        let history = client.get_history();
        let snap = normalize_history(&history);
        assert_eq!(snap.count, 2);
        assert!(snap.has_rewards);
        assert!(snap.has_violations);
    }
}
