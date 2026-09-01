//! Deployment compatibility verification.
//!
//! This module checks that the ledger environment supports the required
//! minimum protocol version before the contract is considered deployable.

use soroban_sdk::{symbol_short, Env, Symbol};

/// Deployment policy asserting protocol-version compatibility.
///
/// Used to verify that the target ledger meets minimum version requirements
/// before contract deployment proceeds.
pub struct DeploymentPolicy;

impl DeploymentPolicy {
    /// Minimum protocol version required for deployment.
    pub const REQUIRED_PROTOCOL_VERSION: u32 = 1;
    /// Deployment tag for release identification.
    pub const DEPLOYMENT_TAG: Symbol = symbol_short!("v1_rel");

    /// Verifies that the current ledger protocol version meets the minimum
    /// requirement for this contract.
    ///
    /// Returns `true` when deployment can proceed safely.
    pub fn verify_deployment_compatibility(env: &Env) -> bool {
        // Ensure ledger environment supports required minimum protocol version
        let current_protocol = env.ledger().protocol_version();
        current_protocol >= Self::REQUIRED_PROTOCOL_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_verify_deployment_compatibility_current_ledger_meets_minimum() {
        let env = Env::default();
        // In the test ledger the protocol version is non-negative and defaults
        // above 0, so deployment compatibility must hold.
        assert!(DeploymentPolicy::verify_deployment_compatibility(&env));
    }
}
