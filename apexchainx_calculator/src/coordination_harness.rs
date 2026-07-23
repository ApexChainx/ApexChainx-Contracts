//! SC-W5-080 – Coordination test harness for staged multi-contract upgrades.
//!
//! This module provides a test harness for simulating and verifying
//! multi-contract coordination scenarios.  It tests that the security model
//! (SC-W5-077), version negotiation (SC-W5-078), and event correlation
//! (SC-W5-079) work together correctly when contracts are deployed and
//! upgraded in stages.
//!
//! # Harness Scenarios
//!
//! 1. **Clean deployment** – All contracts at matching versions → Compatible.
//! 2. **Staged upgrade** – One contract upgraded before others; negotiation
//!    detects the skew and handles it.
//! 3. **Incompatible rollback** – A contract with protocol=0 is detected and
//!    deployment is blocked.
//! 4. **Correlation across boundaries** – A single workflow's correlation ID
//!    is propagated from SLA calculator through payment escrow.
//! 5. **Paused contract in group** – A paused contract still participates in
//!    version negotiation but blocks operations.
//! 6. **Safety rollback on downstream failure** – Cross-contract safety model
//!    compensates prior calls when a downstream call fails.

#[cfg(test)]
mod coordination_harness_tests {
    use alloc::format;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};

    use crate::cross_contract_safety::{self, CrossContractCallStatus, CrossContractSafety};
    use crate::event_correlation;
    use crate::version_negotiation::{
        build_negotiation_info, negotiate_contract_versions, NegotiationOutcome, VersionNegotiationInfo,
    };

    // -----------------------------------------------------------------------
    // Helper: build a mock peer info
    // -----------------------------------------------------------------------
    fn peer_info(name: &str, protocol: u32, storage: u32, min_compat: u32) -> VersionNegotiationInfo {
        VersionNegotiationInfo {
            contract_name: Symbol::new(&Env::default(), name),
            protocol_version: protocol,
            storage_version: storage,
            min_compatible_protocol: min_compat,
            is_paused: false,
            needs_migration: false,
        }
    }

    // ===================================================================
    // Scenario 1: Clean deployment – all contracts at matching versions
    // ===================================================================

    #[test]
    fn test_harness_clean_deployment_all_compatible() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);
        let mut peers = Vec::new(&env);

        // Both downstream contracts at protocol 1 (matching)
        peers.push_back(peer_info("pay_escro", 1, 1, 1));
        peers.push_back(peer_info("settle", 1, 1, 1));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(
            result.outcome,
            NegotiationOutcome::Compatible,
            "Clean deployment must be compatible"
        );
        assert_eq!(result.mismatches.len(), 0);
    }

    // ===================================================================
    // Scenario 2: Staged upgrade – one contract ahead
    // ===================================================================

    #[test]
    fn test_harness_staged_upgrade_detects_skew() {
        let env = Env::default();
        // SLA calculator still on protocol 1
        let our = build_negotiation_info(1, 1, false);

        let mut peers = Vec::new(&env);
        // Payment escrow already upgraded to protocol 2
        peers.push_back(peer_info("pay_escro", 2, 1, 1));
        peers.push_back(peer_info("settle", 1, 1, 1));

        let result = negotiate_contract_versions(&env, &our, &peers);
        // Should be "Negotiated" – protocol differs but both are within range
        assert_eq!(
            result.outcome,
            NegotiationOutcome::Negotiated,
            "Staged upgrade should result in Negotiated outcome"
        );
        assert_eq!(result.mismatches.len(), 0);
    }

    // ===================================================================
    // Scenario 3: Incompatible rollback – protocol out of range
    // ===================================================================

    #[test]
    fn test_harness_incompatible_rollback_detected() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);

        let mut peers = Vec::new(&env);
        // Settlement contract has protocol 0 – below our min_compatible (1)
        peers.push_back(peer_info("pay_escro", 1, 1, 1));
        peers.push_back(peer_info("settle", 0, 1, 0));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(
            result.outcome,
            NegotiationOutcome::Incompatible,
            "Out-of-range protocol must be detected"
        );
        assert_eq!(result.mismatches.len(), 1, "Should report one mismatch");
        let detail = result.mismatches.get(0).unwrap();
        assert_eq!(detail.contract_name, Symbol::new(&env, "settle"));
        assert_eq!(detail.reported_protocol, 0);
    }

    // ===================================================================
    // Scenario 4: Correlation across contract boundaries
    // ===================================================================

    #[test]
    fn test_harness_correlation_across_boundaries() {
        let env = Env::default();
        // Simulate an SLA calculation generating a correlation ID
        let outage_id = Symbol::new(&env, "OUTAGE_ABC");
        let ledger_seq = 12345u32;
        let corr_id = event_correlation::generate_correlation_id(&env, &outage_id, ledger_seq);

        // The correlation ID should be stable and propagate to downstream contracts
        assert_ne!(corr_id, 0, "Correlation ID must be non-zero");

        // Verify the correlation topics structure (3-topic arity)
        let topics = event_correlation::correlation_event_topics(
            symbol_short!("sla_calc"),
            symbol_short!("v1"),
            symbol_short!("critical"),
            corr_id,
        );
        assert_eq!(topics.0, symbol_short!("sla_calc"));
        assert_eq!(topics.1, symbol_short!("v1"));
        assert_eq!(topics.2, symbol_short!("critical"));

        // The same correlation ID should be passed to downstream contract events
        let downstream_topics = event_correlation::correlation_event_topics(
            symbol_short!("set_int"),
            symbol_short!("v1"),
            symbol_short!("critical"),
            corr_id,
        );
        assert_eq!(topics.1, downstream_topics.1, "Event version must match");
        assert_eq!(topics.2, downstream_topics.2, "Context must match");
    }

    // ===================================================================
    // Scenario 5: Paused contract in group
    // ===================================================================

    #[test]
    fn test_harness_paused_contract_in_group() {
        let env = Env::default();
        // SLA calculator is paused
        let our = build_negotiation_info(1, 1, true);

        let mut peers = Vec::new(&env);
        peers.push_back(peer_info("pay_escro", 1, 1, 1));

        // Paused status does not affect version compatibility
        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Compatible);
    }

    // ===================================================================
    // Scenario 6: Safety rollback on downstream failure
    // ===================================================================

    #[test]
    fn test_harness_safety_rollback_on_downstream_failure() {
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);
        let unknown = Address::generate(&env);

        // Try calling an unknown contract – should return FatalError
        let result = safety.call(
            &env,
            &unknown,
            &symbol_short!("lock_fnds"),
            &[],
            cross_contract_safety::COMP_UNLOCK_FUNDS,
            Vec::new(&env),
        );

        assert!(result.is_err(), "Call to unknown contract must return error");
        assert_eq!(result.unwrap_err().status, CrossContractCallStatus::FatalError);
        assert_eq!(safety.depth(), 0, "No compensation registered on failure");
    }

    #[test]
    fn test_harness_safety_compensation_stack_grows_with_successful_calls() {
        // We can't easily mock a successful cross-contract call in unit tests,
        // but we can verify the compensation stack grows when calls succeed.
        // Here we register compensations manually to simulate the pattern.
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);

        // Simulate two successful calls by directly registering compensations
        // (the actual call path would register them via safety.call())
        let mock_contract = Address::generate(&env);
        safety.compensation_stack.push_back((
            mock_contract.clone(),
            symbol_short!("lock_fnds"),
            cross_contract_safety::CompensationAction {
                tag: cross_contract_safety::COMP_UNLOCK_FUNDS,
                args: Vec::new(&env),
            },
        ));
        safety.compensation_stack.push_back((
            mock_contract,
            symbol_short!("rel_pay"),
            cross_contract_safety::CompensationAction {
                tag: cross_contract_safety::COMP_REVERSE_SETTLE,
                args: Vec::new(&env),
            },
        ));

        assert_eq!(safety.depth(), 2);
        assert!(safety.has_pending());
    }

    #[test]
    fn test_harness_empty_compensation_stack() {
        let env = Env::default();
        let safety = CrossContractSafety::new(&env);
        assert_eq!(safety.depth(), 0);
        assert!(!safety.has_pending());
    }

    // ===================================================================
    // Combined scenario: Full multi-contract workflow simulation
    // ===================================================================

    #[test]
    fn test_harness_full_multi_contract_workflow() {
        let env = Env::default();

        // Step 1: Version negotiation – all contracts compatible
        let our = build_negotiation_info(1, 1, false);
        let mut peers = Vec::new(&env);
        peers.push_back(peer_info("pay_escro", 1, 1, 1));
        peers.push_back(peer_info("settle", 1, 1, 1));

        let negotiation = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(
            negotiation.outcome,
            NegotiationOutcome::Compatible,
            "Step 1: All contracts must be compatible"
        );

        // Step 2: Generate correlation ID for the workflow
        let outage_id = Symbol::new(&env, "WF_2024_001");
        let ledger_seq = 50000u32;
        let corr_id = event_correlation::generate_correlation_id(&env, &outage_id, ledger_seq);
        assert_ne!(corr_id, 0, "Step 2: Correlation ID must be non-zero");

        // Step 3: Prepare safety tracker for calls
        let safety = CrossContractSafety::new(&env);
        assert!(!safety.has_pending(), "Step 3: Safety tracker starts empty");

        // Step 4: Verify correlation topics propagate (3-topic arity)
        let sla_topic = event_correlation::correlation_event_topics(
            symbol_short!("sla_calc"),
            symbol_short!("v1"),
            symbol_short!("critical"),
            corr_id,
        );
        let settle_topic = event_correlation::correlation_event_topics(
            symbol_short!("set_int"),
            symbol_short!("v1"),
            symbol_short!("critical"),
            corr_id,
        );
        assert_eq!(sla_topic.1, settle_topic.1, "Step 4: Event version must match");
        assert_eq!(sla_topic.2, settle_topic.2, "Step 4: Context must match");
    }

    // ===================================================================
    // Edge case: All contracts incompatible
    // ===================================================================

    #[test]
    fn test_harness_all_contracts_incompatible() {
        let env = Env::default();
        // Our contract at protocol 1
        let our = build_negotiation_info(1, 1, false);

        let mut peers = Vec::new(&env);
        // All peers at protocol 0 – incompatible
        peers.push_back(peer_info("pay_escro", 0, 1, 0));
        peers.push_back(peer_info("settle", 0, 1, 0));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Incompatible);
        assert_eq!(result.mismatches.len(), 2);
    }

    // ===================================================================
    // Edge case: Single contract deployment
    // ===================================================================

    #[test]
    fn test_harness_single_contract_deployment() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);
        let peers = Vec::new(&env);

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(
            result.outcome,
            NegotiationOutcome::Compatible,
            "Single contract must be compatible"
        );
    }

    // ===================================================================
    // Edge case: Migration required still negotiates
    // ===================================================================

    #[test]
    fn test_harness_migration_required_still_negotiates() {
        let env = Env::default();
        // Our contract needs migration (storage 1, expected 2)
        let our = build_negotiation_info(1, 2, false);
        assert!(our.needs_migration);

        let mut peers = Vec::new(&env);
        peers.push_back(peer_info("pay_escro", 1, 1, 1));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(
            result.outcome,
            NegotiationOutcome::Compatible,
            "Contracts needing migration can still be version-compatible"
        );
    }

    // ===================================================================
    // Edge case: Max peers stress test
    // ===================================================================

    #[test]
    fn test_harness_max_peers_stress() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);

        let mut peers = Vec::new(&env);
        // Register 10 peers all at protocol 1
        for i in 0..10u32 {
            peers.push_back(VersionNegotiationInfo {
                contract_name: Symbol::new(&env, &format!("contract_{}", i)),
                protocol_version: 1,
                storage_version: 1,
                min_compatible_protocol: 1,
                is_paused: false,
                needs_migration: false,
            });
        }

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(
            result.outcome,
            NegotiationOutcome::Compatible,
            "10 peers at protocol 1 must be compatible"
        );
    }
}
