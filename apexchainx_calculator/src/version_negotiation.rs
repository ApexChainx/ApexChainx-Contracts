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
//! - Contracts with the same `protocol_version` and `storage_version` are
//!   fully compatible.
//! - If `protocol_version` differs but both are within the `min_compatible`
//!   range, they are backward-compatible (negotiated).
//! - If any contract's version is outside the acceptable range, negotiation
//!   fails and a deployment should be blocked.
//!
//! # Integration
//!
//! Existing backend-facing endpoints (`get_version_info`, `get_migration_state`)
//! on the SLA calculator are extended with this protocol so that multi-contract
//! backends can verify all contracts agree before deploying.

use soroban_sdk::{contracttype, symbol_short, Env, Symbol, Vec};

/// Version information for a single contract, designed to be returned by
/// a standard `get_version_info()` function on any contract in the ecosystem.
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
}

/// The outcome of a version negotiation between multiple contracts.
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
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionMismatchDetail {
    /// The name of the contract that is out of range.
    pub contract_name: Symbol,
    /// The protocol version the contract reported.
    pub reported_protocol: u32,
    /// The minimum compatible version required by another contract.
    pub required_min: u32,
}

/// Full result of a version negotiation across a set of contracts.
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
/// Minimum protocol version we can interoperate with.
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
            });
            outcome = NegotiationOutcome::Incompatible;
        }

        // Our protocol must be >= peer's min_compatible
        if our_info.protocol_version < peer.min_compatible_protocol {
            mismatches.push_back(VersionMismatchDetail {
                contract_name: our_info.contract_name.clone(),
                reported_protocol: our_info.protocol_version,
                required_min: peer.min_compatible_protocol,
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
    ifaces.push_back(symbol_short!("ver_info"));
    ifaces.push_back(symbol_short!("mig_state"));
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
        VersionNegotiationInfo {
            contract_name: Symbol::new(&Env::default(), name),
            protocol_version: protocol,
            storage_version: storage,
            min_compatible_protocol: min_compat,
            is_paused: paused,
            needs_migration: needs_mig,
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
    fn test_version_discovery_interfaces_are_defined() {
        let env = Env::default();
        let ifaces = version_discovery_interfaces(&env);
        assert_eq!(ifaces.len(), 3);
        assert!(ifaces.contains(&symbol_short!("ver_info")));
        assert!(ifaces.contains(&symbol_short!("mig_state")));
        assert!(ifaces.contains(&symbol_short!("is_paused")));
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
