//! Deployment compatibility verification.
//!
//! This module checks that the ledger environment supports the required
//! minimum protocol version before the contract is considered deployable.

use soroban_sdk::{Env, Symbol, symbol_short};

/// Deployment policy asserting protocol-version compatibility.
///
/// Used to verify that the target ledger meets minimum version requirements
/// before contract deployment proceeds.
pub struct DeploymentPolicy;

impl DeploymentPolicy {
    /// Minimum protocol version required for deployment.
    pub const REQUIRED_PROTOCOL_VERSION: u32 = 1;
    /// Deployment tag for release identification.
    pub const DEPLOYMENT_TAG: Symbol = symbol_short!("v1_release");

    /// Verifies that the current ledger protocol version meets the minimum
    /// requirement for this contract.
    ///
    /// Returns `true` when deployment can proceed safely.
    pub fn verify_deployment_compatibility(env: &Env) -> bool {
        // Ensure ledger environment supports required minimum protocol version
        let current_protocol = env.ledger().protocol_version();
        current_protocol >= 1
    }
}
