//! Governance module tests: role transfers, overflow guards, expiry, and
//! invariant enforcement for the two-step handoff and break-glass paths.
//!
//! These tests are the automated-test complement to the governance static
//! analysis CI job (`governance-slither.yml`). They cover:
//!
//! - Happy-path admin and operator two-step transfers
//! - Expiry window boundary (PROPOSAL_EXPIRY_WINDOW ± 1 second)
//! - Integer overflow/underflow safety in timestamp arithmetic
//! - Single-step (break-glass) operator assignment
//! - Renounce semantics and pending-proposal invalidation
//! - Frozen-config gating on all governance functions
//! - Storage gap note: Soroban uses a key-value storage model; sequential
//!   slot gaps as used in Ethereum upgradeable contracts do not apply here.
//!   See docs/STORAGE_GAP_RATIONALE.md for the full analysis.

#[cfg(test)]
mod governance_tests {
    use crate::{governance::PROPOSAL_EXPIRY_WINDOW, SLACalculatorContract, SLACalculatorContractClient};
    use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};

    fn setup(env: &Env) -> (Address, Address, SLACalculatorContractClient<'_>) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        (admin, operator, client)
    }

    // ── Admin two-step transfer ─────────────────────────────────────────────

    #[test]
    fn test_propose_and_accept_admin_completes_transfer() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.propose_admin(&admin, &new_admin);
        let pending = client.get_pending_admin();
        assert_eq!(pending, Some(new_admin.clone()));

        client.accept_admin(&new_admin);
        assert_eq!(client.get_admin(), new_admin);
        assert_eq!(client.get_pending_admin(), None);
    }

    #[test]
    fn test_cancel_admin_proposal_clears_pending() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);

        client.propose_admin(&admin, &candidate);
        assert!(client.get_pending_admin().is_some());

        client.cancel_admin_proposal(&admin);
        assert_eq!(client.get_pending_admin(), None);
    }

    #[test]
    fn test_re_propose_admin_supersedes_previous_candidate() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let first = Address::generate(&env);
        let second = Address::generate(&env);

        client.propose_admin(&admin, &first);
        // Supersede the first proposal with a new candidate.
        client.propose_admin(&admin, &second);

        let pending = client.get_pending_admin();
        assert_eq!(pending, Some(second.clone()), "Pending slot must hold the latest candidate");
    }

    #[test]
    fn test_wrong_caller_cannot_accept_admin_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);
        let impostor = Address::generate(&env);

        client.propose_admin(&admin, &candidate);
        // An impostor who is not the proposed admin must be rejected.
        assert!(
            client.try_accept_admin(&impostor).is_err(),
            "Only the proposed admin may accept"
        );
    }

    #[test]
    fn test_stranger_cannot_propose_admin() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        let candidate = Address::generate(&env);
        assert!(client.try_propose_admin(&stranger, &candidate).is_err());
    }

    // ── Admin proposal expiry — boundary tests ─────────────────────────────
    //
    // The expiry window is PROPOSAL_EXPIRY_WINDOW seconds (90 days).
    // Exactly at the boundary the proposal is still valid; one second past
    // it is expired. These tests pin the inclusive/exclusive semantics.

    #[test]
    fn test_admin_proposal_valid_at_expiry_boundary() {
        let env = Env::default();
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);

        client.propose_admin(&admin, &candidate);

        // Advance to exactly the boundary — still valid (<=).
        env.ledger()
            .with_mut(|l| l.timestamp = 1_000_000 + PROPOSAL_EXPIRY_WINDOW);
        assert!(client.try_accept_admin(&candidate).is_ok());
    }

    #[test]
    fn test_admin_proposal_expired_one_second_past_window() {
        let env = Env::default();
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);

        client.propose_admin(&admin, &candidate);

        // Advance one second past the window — expired.
        env.ledger()
            .with_mut(|l| l.timestamp = 1_000_000 + PROPOSAL_EXPIRY_WINDOW + 1);
        assert!(
            client.try_accept_admin(&candidate).is_err(),
            "Proposal must be rejected one second past expiry"
        );
    }

    // ── Integer overflow / underflow guards ────────────────────────────────
    //
    // Governance uses `now.saturating_sub(proposed)` to compute elapsed time.
    // These tests verify that:
    //   1. A ledger clock set to u64::MAX does not panic or overflow.
    //   2. A proposed timestamp greater than `now` (clock anomaly) saturates
    //      to 0 rather than wrapping, keeping the proposal valid.
    //   3. PROPOSAL_EXPIRY_WINDOW fits safely in u64 without overflow at
    //      compile time (asserted as a constant below).

    /// PROPOSAL_EXPIRY_WINDOW must fit in u64 without overflow.
    /// 90 * 24 * 60 * 60 = 7_776_000, well within u64::MAX.
    const _OVERFLOW_GUARD: () = assert!(
        PROPOSAL_EXPIRY_WINDOW <= u64::MAX / 2,
        "PROPOSAL_EXPIRY_WINDOW must not be near u64::MAX to prevent addition overflow"
    );

    #[test]
    fn test_governance_saturating_sub_does_not_overflow_at_max_timestamp() {
        let env = Env::default();
        // Set ledger to u64::MAX — saturating_sub must not panic.
        env.ledger().with_mut(|l| l.timestamp = u64::MAX);
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);

        // propose_admin stores the current timestamp (u64::MAX).
        client.propose_admin(&admin, &candidate);

        // accept_admin runs require_proposal_valid: now.saturating_sub(proposed)
        // = u64::MAX - u64::MAX = 0, which is <= PROPOSAL_EXPIRY_WINDOW → valid.
        assert!(
            client.try_accept_admin(&candidate).is_ok(),
            "saturating_sub at u64::MAX must not overflow and must keep proposal valid"
        );
    }

    #[test]
    fn test_governance_clock_anomaly_future_timestamp_saturates_to_valid() {
        // If the ledger clock goes backwards between propose and accept
        // (a theoretical anomaly), saturating_sub returns 0, keeping the
        // proposal valid rather than wrapping to a huge elapsed value.
        let env = Env::default();
        env.ledger().with_mut(|l| l.timestamp = 5_000_000);
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);

        client.propose_admin(&admin, &candidate);

        // Clock moves backwards to before the proposal timestamp.
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        // elapsed = 1_000_000.saturating_sub(5_000_000) = 0 ≤ WINDOW → valid.
        assert!(
            client.try_accept_admin(&candidate).is_ok(),
            "Backwards clock must saturate to 0 elapsed, keeping proposal valid"
        );
    }

    // ── Operator two-step transfer ─────────────────────────────────────────

    #[test]
    fn test_propose_and_accept_operator_completes_handoff() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.propose_operator(&admin, &new_op);
        assert_eq!(client.get_pending_operator(), Some(new_op.clone()));

        client.accept_operator(&new_op);
        assert_eq!(client.get_operator(), new_op);
        assert_eq!(client.get_pending_operator(), None);
    }

    #[test]
    fn test_operator_proposal_expired_one_second_past_window() {
        let env = Env::default();
        env.ledger().with_mut(|l| l.timestamp = 1_000_000);
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.propose_operator(&admin, &new_op);

        env.ledger()
            .with_mut(|l| l.timestamp = 1_000_000 + PROPOSAL_EXPIRY_WINDOW + 1);
        assert!(
            client.try_accept_operator(&new_op).is_err(),
            "Operator proposal must be rejected one second past expiry"
        );
    }

    #[test]
    fn test_cancel_operator_proposal_clears_pending() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.propose_operator(&admin, &new_op);
        client.cancel_operator_proposal(&admin);
        assert_eq!(client.get_pending_operator(), None);
    }

    // ── Single-step (break-glass) set_operator ─────────────────────────────

    #[test]
    fn test_set_operator_installs_operator_without_consent() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.set_operator(&admin, &new_op);
        assert_eq!(client.get_operator(), new_op);
    }

    #[test]
    fn test_set_operator_cancels_pending_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let proposed = Address::generate(&env);
        let direct = Address::generate(&env);

        client.propose_operator(&admin, &proposed);
        assert!(client.get_pending_operator().is_some());

        // Single-step must cancel the pending proposal before installing.
        client.set_operator(&admin, &direct);
        assert_eq!(client.get_operator(), direct);
        assert_eq!(
            client.get_pending_operator(),
            None,
            "Pending proposal must be cleared after set_operator"
        );
    }

    // ── Renounce admin ─────────────────────────────────────────────────────

    #[test]
    fn test_renounce_admin_removes_admin_key() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.renounce_admin(&admin);
        // After renounce the admin key is removed; get_admin must fail.
        assert!(client.try_get_admin().is_err());
    }

    #[test]
    fn test_renounce_cancels_pending_operator_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.propose_operator(&admin, &new_op);
        assert!(client.get_pending_operator().is_some());

        client.renounce_admin(&admin);
        assert_eq!(
            client.get_pending_operator(),
            None,
            "Renounce must invalidate all pending proposals"
        );
    }

    // ── Frozen-config blocks all governance operations ────────────────────

    #[test]
    fn test_governance_blocked_when_config_frozen() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);

        client.freeze_config(&admin);

        assert!(
            client.try_propose_admin(&admin, &candidate).is_err(),
            "propose_admin must be blocked while config is frozen"
        );
        assert!(
            client.try_propose_operator(&admin, &candidate).is_err(),
            "propose_operator must be blocked while config is frozen"
        );
        assert!(
            client.try_set_operator(&admin, &candidate).is_err(),
            "set_operator must be blocked while config is frozen"
        );
        assert!(
            client.try_renounce_admin(&admin).is_err(),
            "renounce_admin must be blocked while config is frozen"
        );
    }

    // ── No-pending-transfer returns correct error ─────────────────────────

    #[test]
    fn test_cancel_admin_proposal_without_pending_returns_error() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        assert!(
            client.try_cancel_admin_proposal(&admin).is_err(),
            "cancel_admin_proposal with no pending proposal must error"
        );
    }

    #[test]
    fn test_accept_admin_without_pending_returns_error() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let caller = Address::generate(&env);
        assert!(
            client.try_accept_admin(&caller).is_err(),
            "accept_admin with no pending proposal must error"
        );
    }
}
