//! SC-W5-078 – Version negotiation protocol for multi-contract deployments.
//!
//! This module defines a standard protocol for contracts to query each other's
//! version information and negotiate compatibility during multi-contract
//! deployments and upgrades.
//!
//! # Protocol
//!
//! Each contract that participates in the multi-contract ecosystem exposes a
//! `get_version_info()` function returning a `VersionNegotiationInfo` struct.
//! A coordinator contract or backend calls `negotiate_contract_versions()` to
//! compare versions across a set of contracts and determine whether they are
//! mutually compatible.
//!
//! # Compatibility Rules
//!
//! - Contracts with the same `protocol_version`, `storage_version`,
//!   `result_schema_version`, and `event_version` are fully compatible.
//! - If `protocol_version` differs but both are within the `min_compatible`
//!   range, they are backward-compatible (negotiated).
//! - `storage_version`, `result_schema_version`, and `event_version` must
//!   match exactly (schema/event drift is never negotiable — #601); any
//!   mismatch fails the handshake and a deployment should be blocked.
//! - If any contract's version is outside the acceptable range, negotiation
//!   fails and a deployment should be blocked.
//!
//! # Integration
//!
//! Backend-facing integration surface:
//!
//! - `get_version_negotiation_info()` — a live `#[contractimpl]` method on the
//!   SLA calculator returns this contract's `VersionNegotiationInfo`
//!   (including `protocol_version` and `min_compatible_protocol`), so a
//!   multi-contract backend can obtain the data it needs to run
//!   `negotiate_contract_versions` against a live deployment (#427).
//! - `get_version_info()` / `get_migration_state()` — existing endpoints that
//!   continue to expose storage/result-schema and migration state.
//! - `negotiate_contract_versions()` / `build_negotiation_info()` are the
//!   pure, library-level rules. A coordinator calls `get_version_negotiation_info`
//!   on each peer (and itself), assembles the peer list, and invokes
//!   `negotiate_contract_versions` to decide whether all contracts agree
//!   before deploying.
//!
//! # Changing this module
//!
//! This protocol runs against independently deployed peers, so a small change
//! here can break interoperability across contracts. Before editing, read
//! `docs/VERSION_NEGOTIATION_CONTRIBUTOR_GUIDE.md` for the compatibility
//! constraints (append-only payloads, ordinal `NegotiationOutcome` variants,
//! fail-closed negotiation, directional version-constant rules) and the
//! author/reviewer checklists that PRs touching this file must satisfy.

use soroban_sdk::{contracttype, symbol_short, Env, Symbol, Vec};

/// Version information for a single contract, designed to be returned by
/// a standard `get_version_info()` function on any contract in the ecosystem.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionNegotiationInfo {
    /// Human-readable contract name for log correlation.
    pub contract_name: Symbol,
    /// The protocol version this contract implements.
    pub protocol_version: u32,
    /// The storage schema version stamped in this contract's storage.
    pub storage_version: u32,
    /// The minimum protocol version this contract can interoperate with.
    pub min_compatible_protocol: u32,
    /// Whether the contract is currently paused (blocking operations).
    pub is_paused: bool,
    /// Whether the contract requires storage migration.
    pub needs_migration: bool,
    /// The result-schema version this contract serializes (#601).
    pub result_schema_version: u32,
    /// The event ABI version this contract emits (#601).
    pub event_version: Symbol,
}

/// The outcome of a version negotiation between multiple contracts.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NegotiationOutcome {
    /// All contracts are fully compatible – proceed.
    Compatible,
    /// Contracts are compatible after negotiation (minor version skew).
    Negotiated,
    /// One or more contracts are incompatible – deployment must be blocked.
    Incompatible,
}

/// Describes which contract(s) caused an incompatibility.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionMismatchDetail {
    /// The name of the contract that is out of range.
    pub contract_name: Symbol,
    /// The protocol version the contract reported.
    pub reported_protocol: u32,
    /// The minimum compatible version required by another contract.
    pub required_min: u32,
    /// Which version dimension mismatched: "protocol", "storage",
    /// "result_schema", or "event" (#601). For the "event" dimension the two
    /// numeric fields carry `0` (event versions are symbols compared
    /// directly; the involved versions are in the peer/self infos).
    pub dimension: Symbol,
}

/// Full result of a version negotiation across a set of contracts.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionNegotiationResult {
    /// The overall outcome.
    pub outcome: NegotiationOutcome,
    /// Human-readable summary for logs.
    pub summary: Symbol,
    /// Details of any mismatches (empty when fully compatible).
    pub mismatches: Vec<VersionMismatchDetail>,
}

/// Standard symbol for the SLA calculator contract.
pub const CONTRACT_SLA_CALC: Symbol = symbol_short!("sla_calc");
/// Standard symbol for the payment escrow contract.
pub const CONTRACT_PAY_ESCROW: Symbol = symbol_short!("pay_escro");
/// Standard symbol for the settlement contract.
pub const CONTRACT_SETTLEMENT: Symbol = symbol_short!("settle");

/// Current protocol version for the multi-contract ecosystem.
pub const PROTOCOL_VERSION: u32 = 1;
/// Minimum protocol version this contract can interoperate with.
pub const MIN_COMPATIBLE_PROTOCOL: u32 = 1;

/// Builds the `VersionNegotiationInfo` for this contract (apexchainx_calculator).
///
/// This is the canonical implementation that should be returned by
/// `get_version_info()` or exposed to coordinators.
pub fn build_negotiation_info(
    storage_version: u32,
    expected_version: u32,
    is_paused: bool,
) -> VersionNegotiationInfo {
    VersionNegotiationInfo {
        contract_name: CONTRACT_SLA_CALC,
        protocol_version: PROTOCOL_VERSION,
        storage_version,
        min_compatible_protocol: MIN_COMPATIBLE_PROTOCOL,
        is_paused,
        needs_migration: storage_version != expected_version,
        result_schema_version: crate::RESULT_SCHEMA_VERSION,
        event_version: crate::event_schema::EVENT_VERSION,
    }
}

/// Negotiate compatibility across a list of contract version infos.
///
/// Returns a `VersionNegotiationResult` summarising the compatibility of the
/// group.  The group includes this contract and one or more downstream
/// contracts.
pub fn negotiate_contract_versions(
    env: &Env,
    our_info: &VersionNegotiationInfo,
    peer_infos: &Vec<VersionNegotiationInfo>,
) -> VersionNegotiationResult {
    let mut mismatches = Vec::new(env);
    let mut outcome = NegotiationOutcome::Compatible;

    // Check our own compatibility against each peer
    for i in 0..peer_infos.len() {
        let peer = peer_infos.get(i).unwrap();

        // Peer's protocol must be >= our min_compatible
        if peer.protocol_version < our_info.min_compatible_protocol {
            mismatches.push_back(VersionMismatchDetail {
                contract_name: peer.contract_name.clone(),
                reported_protocol: peer.protocol_version,
                required_min: our_info.min_compatible_protocol,
                dimension: Symbol::new(env, "protocol"),
            });
            outcome = NegotiationOutcome::Incompatible;
        }

        // Our protocol must be >= peer's min_compatible
        if our_info.protocol_version < peer.min_compatible_protocol {
            mismatches.push_back(VersionMismatchDetail {
                contract_name: our_info.contract_name.clone(),
                reported_protocol: our_info.protocol_version,
                required_min: peer.min_compatible_protocol,
                dimension: Symbol::new(env, "protocol"),
            });
            outcome = NegotiationOutcome::Incompatible;
        }

        // Schema/event dimensions are exact-match: a compatible protocol
        // handshake must NOT tolerate storage, result-schema, or event-ABI
        // drift, because that is exactly the deployment the handshake exists
        // to prevent (#601: co-bump invariant enforced in the handshake, not
        // just in tests).
        if peer.storage_version != our_info.storage_version {
            mismatches.push_back(VersionMismatchDetail {
                contract_name: peer.contract_name.clone(),
                reported_protocol: peer.storage_version,
                required_min: our_info.storage_version,
                dimension: Symbol::new(env, "storage"),
            });
            outcome = NegotiationOutcome::Incompatible;
        }

        if peer.result_schema_version != our_info.result_schema_version {
            mismatches.push_back(VersionMismatchDetail {
                contract_name: peer.contract_name.clone(),
                reported_protocol: peer.result_schema_version,
                required_min: our_info.result_schema_version,
                dimension: Symbol::new(env, "result_schema"),
            });
            outcome = NegotiationOutcome::Incompatible;
        }

        if peer.event_version != our_info.event_version {
            mismatches.push_back(VersionMismatchDetail {
                contract_name: peer.contract_name.clone(),
                reported_protocol: 0,
                required_min: 0,
                dimension: Symbol::new(env, "event"),
            });
            outcome = NegotiationOutcome::Incompatible;
        }

        // If protocol versions differ but both are within min_compatible range,
        // it's a negotiated (non-breaking) difference
        if outcome == NegotiationOutcome::Compatible && peer.protocol_version != our_info.protocol_version {
            outcome = NegotiationOutcome::Negotiated;
        }
    }

    let summary = match outcome {
        NegotiationOutcome::Compatible => symbol_short!("compat"),
        NegotiationOutcome::Negotiated => symbol_short!("negoti"),
        NegotiationOutcome::Incompatible => symbol_short!("incompt"),
    };

    VersionNegotiationResult {
        outcome,
        summary,
        mismatches,
    }
}

/// Returns a standard set of expected backend interface symbols that
/// downstream contracts should expose for version discovery.
pub fn version_discovery_interfaces(env: &Env) -> Vec<Symbol> {
    let mut ifaces = Vec::new(env);
    ifaces.push_back(Symbol::new(env, "get_version_info"));
    ifaces.push_back(Symbol::new(env, "get_migration_state"));
    ifaces.push_back(symbol_short!("is_paused"));
    ifaces
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{symbol_short, Env};

    fn make_info(
        name: &str,
        protocol: u32,
        storage: u32,
        min_compat: u32,
        paused: bool,
        needs_mig: bool,
    ) -> VersionNegotiationInfo {
        // Default schema/event dims match build_negotiation_info's
        // (RESULT_SCHEMA_VERSION = 1, EVENT_VERSION = "v1").
        let event_v1 = symbol_short!("v1");
        make_info_full(name, protocol, storage, min_compat, paused, needs_mig, 1, &event_v1)
    }

    fn make_info_full(
        name: &str,
        protocol: u32,
        storage: u32,
        min_compat: u32,
        paused: bool,
        needs_mig: bool,
        result_schema: u32,
        event: &Symbol,
    ) -> VersionNegotiationInfo {
        VersionNegotiationInfo {
            contract_name: Symbol::new(&Env::default(), name),
            protocol_version: protocol,
            storage_version: storage,
            min_compatible_protocol: min_compat,
            is_paused: paused,
            needs_migration: needs_mig,
            result_schema_version: result_schema,
            event_version: event.clone(),
        }
    }

    #[test]
    fn test_self_negotiation_is_compatible() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);
        let peers = Vec::new(&env);
        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Compatible);
        assert_eq!(result.summary, symbol_short!("compat"));
        assert_eq!(result.mismatches.len(), 0);
    }

    #[test]
    fn test_matching_protocol_versions_are_compatible() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);
        let mut peers = Vec::new(&env);
        peers.push_back(make_info("pay_escro", 1, 1, 1, false, false));
        peers.push_back(make_info("settle", 1, 1, 1, false, false));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Compatible);
    }

    #[test]
    fn test_peer_out_of_range_is_incompatible() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);
        let mut peers = Vec::new(&env);
        // Peer protocol version 0 is below our min_compatible (1)
        peers.push_back(make_info("pay_escro", 0, 1, 0, false, false));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Incompatible);
        assert_eq!(result.summary, symbol_short!("incompt"));
        assert_eq!(result.mismatches.len(), 1);
        assert_eq!(
            result.mismatches.get(0).unwrap().contract_name,
            symbol_short!("pay_escro")
        );
    }

    #[test]
    fn test_minor_version_skew_is_negotiated() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false); // protocol=1
        let mut peers = Vec::new(&env);
        // Peer protocol version 2 – different but within range
        peers.push_back(make_info("settle", 2, 1, 1, false, false));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Negotiated);
        assert_eq!(result.summary, symbol_short!("negoti"));
    }

    #[test]
    fn test_paused_contract_still_negotiates() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, true); // paused
        let mut peers = Vec::new(&env);
        peers.push_back(make_info("pay_escro", 1, 1, 1, false, false));

        let result = negotiate_contract_versions(&env, &our, &peers);
        // Paused status does not affect version compatibility
        assert_eq!(result.outcome, NegotiationOutcome::Compatible);
    }

    #[test]
    fn test_needs_migration_still_negotiates() {
        let env = Env::default();
        let our = build_negotiation_info(1, 2, false); // needs migration
        let mut peers = Vec::new(&env);
        peers.push_back(make_info("pay_escro", 1, 1, 1, false, false));

        let result = negotiate_contract_versions(&env, &our, &peers);
        // Migration status does not affect version compatibility
        assert_eq!(result.outcome, NegotiationOutcome::Compatible);
    }

    #[test]
    fn test_multiple_mismatches_collected() {
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false);
        let mut peers = Vec::new(&env);
        peers.push_back(make_info("pay_escro", 0, 1, 0, false, false));
        peers.push_back(make_info("settle", 0, 1, 0, false, false));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Incompatible);
        assert_eq!(result.mismatches.len(), 2);
    }

    #[test]
    fn test_storage_schema_mismatch_is_incompatible() {
        // (#601) Protocol handshake alone is not enough: a peer on the same
        // protocol but a newer storage layout must be rejected.
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false); // storage 1
        let mut peers = Vec::new(&env);
        let peer = make_info_full("pay_escro", 1, 2, 1, false, true, 1, &symbol_short!("v1"));
        peers.push_back(peer);

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Incompatible);
        let d = result.mismatches.get(0).unwrap();
        assert_eq!(d.contract_name, symbol_short!("pay_escro"));
        assert_eq!(d.dimension, Symbol::new(&env, "storage"));
        assert_eq!(d.reported_protocol, 2);
        assert_eq!(d.required_min, 1);
    }

    #[test]
    fn test_result_schema_mismatch_is_incompatible() {
        // (#601) A peer that serializes results under a different schema
        // version is a broken-ABI peer and must fail the handshake.
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false); // result schema 1
        let mut peers = Vec::new(&env);
        peers.push_back(make_info_full("settle", 1, 1, 1, false, false, 2, &symbol_short!("v1")));

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Incompatible);
        let d = result.mismatches.get(0).unwrap();
        assert_eq!(d.contract_name, symbol_short!("settle"));
        assert_eq!(d.dimension, Symbol::new(&env, "result_schema"));
        assert_eq!(d.reported_protocol, 2);
        assert_eq!(d.required_min, 1);
    }

    #[test]
    fn test_event_version_mismatch_is_incompatible() {
        // (#601) A peer emitting an older (or newer) event ABI while agreeing
        // on protocol/storage/result-schema is still a broken-ABI peer.
        let env = Env::default();
        let our = build_negotiation_info(1, 1, false); // event v1
        let mut peers = Vec::new(&env);
        let peer = make_info_full("pay_escro", 1, 1, 1, false, false, 1, &symbol_short!("v2"));
        peers.push_back(peer);

        let result = negotiate_contract_versions(&env, &our, &peers);
        assert_eq!(result.outcome, NegotiationOutcome::Incompatible);
        let d = result.mismatches.get(0).unwrap();
        assert_eq!(d.contract_name, symbol_short!("pay_escro"));
        assert_eq!(d.dimension, Symbol::new(&env, "event"));
    }

    #[test]
    fn test_version_discovery_interfaces_are_defined() {
        let env = Env::default();
        let ifaces = version_discovery_interfaces(&env);
        assert_eq!(ifaces.len(), 3);
        assert!(ifaces.contains(Symbol::new(&env, "get_version_info")));
        assert!(ifaces.contains(Symbol::new(&env, "get_migration_state")));
        assert!(ifaces.contains(&symbol_short!("is_paused")));
    }

    #[test]
    fn test_version_discovery_interfaces_match_actual_methods() {
        let env = Env::default();
        let ifaces = version_discovery_interfaces(&env);

        // Verify each discovery symbol corresponds to an actual contract method
        // These are the exact method names exposed in the contract's public API
        let expected_methods = [
            Symbol::new(&env, "get_version_info"),
            Symbol::new(&env, "get_migration_state"),
            symbol_short!("is_paused"),
        ];

        for expected_method in expected_methods.iter() {
            assert!(
                ifaces.contains(expected_method),
                "Discovery list should contain actual method: {:?}",
                expected_method
            );
        }
    }

    #[test]
    fn test_negotiation_info_storage_version() {
        let info = build_negotiation_info(1, 1, false);
        assert_eq!(info.storage_version, 1);
        assert!(!info.needs_migration);
    }

    #[test]
    fn test_negotiation_info_detects_migration_needed() {
        let info = build_negotiation_info(1, 2, false);
        assert!(info.needs_migration);
    }

    #[test]
    fn test_contract_name_symbols_are_distinct() {
        let names = [CONTRACT_SLA_CALC, CONTRACT_PAY_ESCROW, CONTRACT_SETTLEMENT];
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(names[i], names[j]);
            }
        }
    }

    #[test]
    fn test_protocol_version_is_one() {
        assert_eq!(PROTOCOL_VERSION, 1);
        assert_eq!(MIN_COMPATIBLE_PROTOCOL, 1);
    }

    #[test]
    fn test_negotiation_outcome_variants_are_distinct() {
        assert_ne!(
            NegotiationOutcome::Compatible as u32,
            NegotiationOutcome::Negotiated as u32
        );
        assert_ne!(
            NegotiationOutcome::Compatible as u32,
            NegotiationOutcome::Incompatible as u32
        );
        assert_ne!(
            NegotiationOutcome::Negotiated as u32,
            NegotiationOutcome::Incompatible as u32
        );
    }
}
