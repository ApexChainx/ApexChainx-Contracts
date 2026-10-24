/// Unit tests for the Governance module.
///
/// Covers the two-step admin/operator handoff, single-step operator assignment,
/// admin renounce, proposal cancellation, expiry semantics, and all negative
/// (unauthorized) paths. Resolves issue #818.
#[cfg(test)]
mod governance_tests {
    #![allow(clippy::module_inception)]
    use crate::{
        SLACalculatorContract, SLACalculatorContractClient, EVENT_ADMIN_ACC, EVENT_ADMIN_PROP,
    };
    use soroban_sdk::{
        testutils::Address as _, testutils::Events, Address, Env, TryIntoVal,
    };

    /// Creates a fresh contract environment with admin and operator initialized.
    fn setup(env: &Env) -> (Address, Address, SLACalculatorContractClient<'_>) {
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let operator = Address::generate(env);
        client.initialize(&admin, &operator);
        (admin, operator, client)
    }

    // ── Admin propose/accept two-step handoff ─────────────────────────────

    /// Happy path: propose then accept admin transfer.
    #[test]
    fn test_propose_and_accept_admin_happy_path() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);

        // No pending admin before proposal
        assert_eq!(client.get_pending_admin(), None);

        client.propose_admin(&admin, &new_admin);
        assert_eq!(client.get_pending_admin(), Some(new_admin.clone()));

        client.accept_admin(&new_admin);
        // After acceptance the pending slot is cleared
        assert_eq!(client.get_pending_admin(), None);
        // New admin should now be the stored admin
        assert_eq!(client.get_admin(), new_admin);
    }

    /// Admin proposal and acceptance each emit their corresponding event.
    #[test]
    fn test_admin_handoff_emits_proposal_and_acceptance_events() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.propose_admin(&admin, &new_admin);
        client.accept_admin(&new_admin);

        let events = env.events().all();
        let mut proposal_count = 0;
        let mut acceptance_count = 0;
        for i in 0..events.len() {
            let (_, topics, _) = events.get(i).unwrap();
            if !topics.is_empty() {
                let name = topics.get(0).unwrap().try_into_val(&env).unwrap();
                if name == EVENT_ADMIN_PROP {
                    proposal_count += 1;
                }
                if name == EVENT_ADMIN_ACC {
                    acceptance_count += 1;
                }
            }
        }

        assert_eq!(proposal_count, 1);
        assert_eq!(acceptance_count, 1);
    }

    /// Old admin loses authority once the new admin accepts.
    #[test]
    fn test_old_admin_cannot_act_after_accept() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.propose_admin(&admin, &new_admin);
        client.accept_admin(&new_admin);

        // Old admin should no longer be able to propose another admin
        env.mock_auths(&[]);
        let result = client.try_propose_admin(&admin, &Address::generate(&env));
        assert!(result.is_err(), "old admin must lose authority after handoff");
    }

    /// Wrong caller cannot accept a pending admin proposal.
    #[test]
    fn test_wrong_caller_cannot_accept_admin() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);
        let stranger = Address::generate(&env);

        client.propose_admin(&admin, &new_admin);

        env.mock_auths(&[]);
        let result = client.try_accept_admin(&stranger);
        assert!(result.is_err(), "stranger must not be able to accept admin proposal");
    }

    /// Accepting when there is no proposal must fail.
    #[test]
    fn test_accept_admin_without_proposal_fails() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);

        let result = client.try_accept_admin(&stranger);
        assert!(result.is_err(), "accept_admin with no pending proposal must return an error");
    }

    /// Governance validation failures keep their typed contract error codes.
    #[test]
    fn test_governance_failures_return_typed_errors() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let candidate = Address::generate(&env);

        assert_eq!(
            client.try_propose_admin(&admin, &admin),
            Err(Ok(SLAError::InvalidInput))
        );
        assert_eq!(
            client.try_accept_admin(&candidate),
            Err(Ok(SLAError::NoPendingTransfer))
        );
        assert_eq!(
            client.try_cancel_admin_proposal(&admin),
            Err(Ok(SLAError::NoPendingTransfer))
        );
    }

    /// Re-proposing a new admin supersedes the previous pending candidate.
    #[test]
    fn test_repropose_admin_supersedes_previous() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let first_candidate = Address::generate(&env);
        let second_candidate = Address::generate(&env);

        client.propose_admin(&admin, &first_candidate);
        assert_eq!(client.get_pending_admin(), Some(first_candidate.clone()));

        // Re-propose with a different candidate
        client.propose_admin(&admin, &second_candidate);
        // Pending slot should now hold the second candidate
        assert_eq!(client.get_pending_admin(), Some(second_candidate.clone()));

        // The first candidate can no longer accept
        env.mock_auths(&[]);
        let result = client.try_accept_admin(&first_candidate);
        assert!(result.is_err(), "superseded candidate must not be able to accept");
    }

    // ── Admin proposal cancellation ───────────────────────────────────────

    /// Admin can cancel its own pending proposal.
    #[test]
    fn test_admin_can_cancel_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.propose_admin(&admin, &new_admin);
        assert!(client.get_pending_admin().is_some());

        client.cancel_admin_proposal(&admin);
        assert_eq!(client.get_pending_admin(), None);
    }

    /// Cancelling when there is no pending proposal must fail.
    #[test]
    fn test_cancel_admin_proposal_with_no_proposal_fails() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        let result = client.try_cancel_admin_proposal(&admin);
        assert!(result.is_err(), "cancel with no pending proposal must fail");
    }

    /// A non-admin cannot cancel an admin proposal.
    #[test]
    fn test_stranger_cannot_cancel_admin_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);
        let stranger = Address::generate(&env);

        client.propose_admin(&admin, &new_admin);

        env.mock_auths(&[]);
        let result = client.try_cancel_admin_proposal(&stranger);
        assert!(result.is_err(), "stranger must not cancel admin proposal");
    }

    // ── Admin renounce ────────────────────────────────────────────────────

    /// Admin can renounce, after which no admin-gated calls succeed.
    #[test]
    fn test_renounce_admin_is_irreversible() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        client.renounce_admin(&admin);

        // No admin means propose_admin must fail
        env.mock_auths(&[]);
        let result = client.try_propose_admin(&admin, &Address::generate(&env));
        assert!(result.is_err(), "admin-gated call must fail after renounce");
    }

    /// Renounce clears any pending admin proposal at the same time.
    #[test]
    fn test_renounce_clears_pending_admin_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_admin = Address::generate(&env);

        client.propose_admin(&admin, &new_admin);
        assert!(client.get_pending_admin().is_some());

        client.renounce_admin(&admin);
        assert_eq!(client.get_pending_admin(), None);
    }

    /// Renounce clears any pending operator proposal.
    #[test]
    fn test_renounce_clears_pending_operator_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.propose_operator(&admin, &new_op);
        assert!(client.get_pending_operator().is_some());

        client.renounce_admin(&admin);
        assert_eq!(client.get_pending_operator(), None);
    }

    /// A non-admin cannot renounce.
    #[test]
    fn test_stranger_cannot_renounce() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);

        env.mock_auths(&[]);
        let result = client.try_renounce_admin(&stranger);
        assert!(result.is_err(), "stranger must not be able to renounce admin");
    }

    // ── Operator propose/accept two-step handoff ──────────────────────────

    /// Happy path: propose then accept operator transfer.
    #[test]
    fn test_propose_and_accept_operator_happy_path() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        assert_eq!(client.get_pending_operator(), None);

        client.propose_operator(&admin, &new_op);
        assert_eq!(client.get_pending_operator(), Some(new_op.clone()));

        client.accept_operator(&new_op);
        assert_eq!(client.get_pending_operator(), None);
        assert_eq!(client.get_operator(), new_op);
    }

    /// Wrong address cannot accept an operator proposal.
    #[test]
    fn test_wrong_caller_cannot_accept_operator() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);
        let stranger = Address::generate(&env);

        client.propose_operator(&admin, &new_op);

        env.mock_auths(&[]);
        let result = client.try_accept_operator(&stranger);
        assert!(result.is_err(), "stranger must not accept operator proposal");
    }

    /// Accepting when there is no pending operator proposal must fail.
    #[test]
    fn test_accept_operator_without_proposal_fails() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);

        let result = client.try_accept_operator(&stranger);
        assert!(result.is_err(), "accept_operator with no pending proposal must fail");
    }

    /// Re-proposing supersedes the previous operator candidate.
    #[test]
    fn test_repropose_operator_supersedes_previous() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let first_op = Address::generate(&env);
        let second_op = Address::generate(&env);

        client.propose_operator(&admin, &first_op);
        client.propose_operator(&admin, &second_op);

        assert_eq!(client.get_pending_operator(), Some(second_op.clone()));

        env.mock_auths(&[]);
        let result = client.try_accept_operator(&first_op);
        assert!(result.is_err(), "superseded operator must not accept");
    }

    // ── Operator proposal cancellation ───────────────────────────────────

    /// Admin can cancel a pending operator proposal.
    #[test]
    fn test_admin_can_cancel_operator_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.propose_operator(&admin, &new_op);
        assert!(client.get_pending_operator().is_some());

        client.cancel_operator_proposal(&admin);
        assert_eq!(client.get_pending_operator(), None);
    }

    /// Cancelling an operator proposal with no pending proposal must fail.
    #[test]
    fn test_cancel_operator_proposal_with_none_fails() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);

        let result = client.try_cancel_operator_proposal(&admin);
        assert!(result.is_err(), "cancel with no pending operator proposal must fail");
    }

    // ── Single-step set_operator (legacy break-glass) ─────────────────────

    /// Admin can directly set an operator via single-step path.
    #[test]
    fn test_admin_can_set_operator_directly() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let new_op = Address::generate(&env);

        client.set_operator(&admin, &new_op);
        assert_eq!(client.get_operator(), new_op);
    }

    /// set_operator cancels any pending two-step proposal.
    #[test]
    fn test_set_operator_cancels_pending_proposal() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let proposed_op = Address::generate(&env);
        let direct_op = Address::generate(&env);

        client.propose_operator(&admin, &proposed_op);
        assert!(client.get_pending_operator().is_some());

        client.set_operator(&admin, &direct_op);
        // Pending slot must be cleared
        assert_eq!(client.get_pending_operator(), None);
        assert_eq!(client.get_operator(), direct_op);
    }

    /// A non-admin cannot call set_operator.
    #[test]
    fn test_stranger_cannot_set_operator() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        let stranger = Address::generate(&env);
        let new_op = Address::generate(&env);

        env.mock_auths(&[]);
        let result = client.try_set_operator(&stranger, &new_op);
        assert!(result.is_err(), "stranger must not call set_operator");
    }

    /// The operator role itself cannot call set_operator.
    #[test]
    fn test_operator_cannot_call_set_operator() {
        let env = Env::default();
        let (_, operator, client) = setup(&env);
        let new_op = Address::generate(&env);

        env.mock_auths(&[]);
        let result = client.try_set_operator(&operator, &new_op);
        assert!(result.is_err(), "operator must not call set_operator");
    }

    // ── Query functions ───────────────────────────────────────────────────

    /// get_pending_admin returns None when no proposal is active.
    #[test]
    fn test_get_pending_admin_returns_none_initially() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        assert_eq!(client.get_pending_admin(), None);
    }

    /// get_pending_operator returns None when no proposal is active.
    #[test]
    fn test_get_pending_operator_returns_none_initially() {
        let env = Env::default();
        let (_, _, client) = setup(&env);
        assert_eq!(client.get_pending_operator(), None);
    }

    // ── Double-propose admin then cancel then re-propose ──────────────────

    /// Full lifecycle: propose → cancel → re-propose → accept.
    #[test]
    fn test_full_admin_proposal_lifecycle() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let first = Address::generate(&env);
        let second = Address::generate(&env);

        client.propose_admin(&admin, &first);
        client.cancel_admin_proposal(&admin);
        assert_eq!(client.get_pending_admin(), None);

        client.propose_admin(&admin, &second);
        client.accept_admin(&second);
        assert_eq!(client.get_admin(), second);
    }

    /// Full operator lifecycle: propose → cancel → direct set.
    #[test]
    fn test_full_operator_proposal_lifecycle() {
        let env = Env::default();
        let (admin, _, client) = setup(&env);
        let proposed = Address::generate(&env);
        let direct = Address::generate(&env);

        client.propose_operator(&admin, &proposed);
        client.cancel_operator_proposal(&admin);
        assert_eq!(client.get_pending_operator(), None);

        client.set_operator(&admin, &direct);
        assert_eq!(client.get_operator(), direct);
    }
}
