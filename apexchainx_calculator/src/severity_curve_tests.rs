#[cfg(test)]
mod severity_curve_tests {
    // #610 – Boundary fixtures for the severity curves documented in
    // docs/SEVERITY_CURVES.md. Every edge of the admissible configuration
    // region is pinned as an accept/reject pair so the region cannot drift
    // away from the documentation.
    #![allow(clippy::module_inception)]
    use crate::{SLACalculator, SLACalculatorContract, SLACalculatorContractClient};
    use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, Symbol};

    fn setup(env: &Env) -> (Address, Address, SLACalculatorContractClient<'_>) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        (admin, operator, client)
    }

    /// Baseline config that satisfies every rule, used to bring storage into
    /// a state against which the cross-severity validators can be exercised.
    fn seed_baseline(client: &SLACalculatorContractClient<'_>, admin: &Address) {
        client.set_config(admin, &symbol_short!("critical"), &60, &50, &500);
        client.set_config(admin, &symbol_short!("high"), &120, &50, &500);
        client.set_config(admin, &symbol_short!("medium"), &240, &10, &500);
        client.set_config(admin, &symbol_short!("low"), &1440, &10, &500);
    }

    fn accepts(client: &SLACalculatorContractClient<'_>, admin: &Address, sev: Symbol, t: u32, p: i128, r: i128) {
        assert_eq!(
            client.try_set_config(admin, &sev, &t, &p, &r).map(|_| ()),
            Ok(()),
            "expected accept for boundary ({}_{}_{}_{})",
            sev.to_string(),
            t,
            p,
            r
        );
    }

    fn rejects(client: &SLACalculatorContractClient<'_>, admin: &Address, sev: Symbol, t: u32, p: i128, r: i128) {
        assert!(
            client.try_set_config(admin, &sev, &t, &p, &r).is_err(),
            "expected reject for boundary ({}_{}_{}_{})",
            sev.to_string(),
            t,
            p,
            r
        );
    }

    // ── General bounds (1..=1440, 1..=10000, 1..=100000) ─────────────
    //
    // The shared-range min/max are enforced by validate_general_bounds
    // (exercised directly in test_documented_edges_direct). Through set_config
    // only the window intersecting the per-severity region is reachable, which
    // is what these client-level cases pin.

    #[test]
    fn test_general_bounds_threshold_via_client() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        accepts(&client, &admin, symbol_short!("low"), 1, 10, 100);
        rejects(&client, &admin, symbol_short!("low"), 0, 10, 100);
    }

    #[test]
    fn test_general_bounds_penalty_via_client() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        accepts(&client, &admin, symbol_short!("low"), 60, 1, 100);
        rejects(&client, &admin, symbol_short!("low"), 60, 0, 100);
    }

    #[test]
    fn test_general_bounds_reward_via_client() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        accepts(&client, &admin, symbol_short!("medium"), 60, 25, 100);
        rejects(&client, &admin, symbol_short!("medium"), 60, 25, 0);
    }

    // ── Per-severity caps/floors ─────────────────────────────────────

    #[test]
    fn test_threshold_caps() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        accepts(&client, &admin, symbol_short!("critical"), 60, 50, 500);
        rejects(&client, &admin, symbol_short!("critical"), 61, 50, 500);
        accepts(&client, &admin, symbol_short!("high"), 120, 25, 500);
        rejects(&client, &admin, symbol_short!("high"), 121, 25, 500);
        accepts(&client, &admin, symbol_short!("medium"), 240, 10, 500);
        rejects(&client, &admin, symbol_short!("medium"), 241, 10, 500);
    }

    #[test]
    fn test_penalty_floors() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        accepts(&client, &admin, symbol_short!("critical"), 60, 50, 500);
        rejects(&client, &admin, symbol_short!("critical"), 60, 49, 500);
        accepts(&client, &admin, symbol_short!("high"), 120, 25, 500);
        rejects(&client, &admin, symbol_short!("high"), 120, 24, 500);
        accepts(&client, &admin, symbol_short!("medium"), 240, 10, 500);
        rejects(&client, &admin, symbol_short!("medium"), 240, 9, 500);
    }

    #[test]
    fn test_low_cap_and_exemption() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        // low penalty cap = 100; threshold capped only by the general 1440.
        accepts(&client, &admin, symbol_short!("low"), 1440, 100, 500);
        rejects(&client, &admin, symbol_short!("low"), 1440, 101, 500);
        accepts(&client, &admin, symbol_short!("low"), 1440, 1, 500);
        // low is exempt from the cross-severity upper check: any value within
        // its own cap is accepted even if it exceeds medium's penalty.
    }

    // ── Cross-parameter reward ratio (penalty * 3 < reward * 2) ─────

    #[test]
    fn test_reward_ratio_boundary() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        // 60 * 3 = 180 < 181 * 2 = 362 → accept; 180 !< 360 → reject.
        accepts(&client, &admin, symbol_short!("medium"), 240, 60, 181);
        rejects(&client, &admin, symbol_short!("medium"), 240, 60, 180);
    }

    // ── Cross-severity ordering (critical ≥ high ≥ medium; low exempt) ─

    #[test]
    fn test_cross_severity_threshold_ordering() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        seed_baseline(&client, &admin); // crit 60, high 120, medium 240, low 1440
        // In-order edges.
        accepts(&client, &admin, symbol_short!("high"), 120, 25, 500);
        accepts(&client, &admin, symbol_short!("medium"), 240, 10, 500);
        accepts(&client, &admin, symbol_short!("low"), 1440, 10, 500);
        // Inversions against adjacent stored severities are rejected.
        rejects(&client, &admin, symbol_short!("high"), 59, 25, 500); // < critical 60
        rejects(&client, &admin, symbol_short!("medium"), 119, 10, 500); // < high 120
        rejects(&client, &admin, symbol_short!("low"), 239, 10, 500); // < medium 240
    }

    #[test]
    fn test_cross_severity_penalty_ordering() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        seed_baseline(&client, &admin); // crit 50, high 50, medium 10, low 10
        // In-order edges.
        accepts(&client, &admin, symbol_short!("critical"), 60, 50, 500);
        accepts(&client, &admin, symbol_short!("medium"), 240, 50, 500);
        accepts(&client, &admin, symbol_short!("high"), 120, 50, 500);
        // Inversions are rejected.
        rejects(&client, &admin, symbol_short!("medium"), 240, 51, 500); // > high 50
        rejects(&client, &admin, symbol_short!("high"), 120, 51, 500); // > critical 50
        // Low exemption: 60 exceeds medium (50) yet is within the low cap 100.
        accepts(&client, &admin, symbol_short!("low"), 1440, 60, 500);
        // Critical must never drop below high.
        rejects(&client, &admin, symbol_short!("critical"), 60, 49, 500);
    }

    // ── Direct validator pinning (edges, incl. the shared-range cap that
    //    canonical severities cannot reach through set_config) ─────────

    #[test]
    fn test_documented_edges_direct() {
        assert!(SLACalculator::validate_config(&symbol_short!("critical"), 60, 50, 500).is_ok());
        assert!(SLACalculator::validate_config(&symbol_short!("critical"), 61, 50, 500).is_err());
        assert!(SLACalculator::validate_config(&symbol_short!("high"), 120, 25, 500).is_ok());
        assert!(SLACalculator::validate_config(&symbol_short!("high"), 121, 25, 500).is_err());
        assert!(SLACalculator::validate_config(&symbol_short!("medium"), 240, 10, 500).is_ok());
        assert!(SLACalculator::validate_config(&symbol_short!("medium"), 241, 10, 500).is_err());
        assert!(SLACalculator::validate_config(&symbol_short!("low"), 1440, 100, 500).is_ok());
        assert!(SLACalculator::validate_config(&symbol_short!("low"), 1440, 101, 500).is_err());

        assert!(SLACalculator::validate_general_bounds(0, 1, 100).is_err());
        assert!(SLACalculator::validate_general_bounds(1441, 1, 100).is_err());
        assert!(SLACalculator::validate_general_bounds(1, 0, 100).is_err());
        assert!(SLACalculator::validate_general_bounds(1, 10001, 100).is_err());
        assert!(SLACalculator::validate_general_bounds(1, 1, 0).is_err());
        assert!(SLACalculator::validate_general_bounds(1, 1, 100_001).is_err());
        assert!(SLACalculator::validate_general_bounds(1, 1, 100_000).is_ok());
    }
}