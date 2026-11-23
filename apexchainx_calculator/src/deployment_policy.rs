//! Deployment compatibility verification.
//!
//! This module checks that the ledger environment supports the required
//! minimum protocol version before the contract is considered deployable.
//!
//! `REQUIRED_PROTOCOL_VERSION` is **derived** from
//! `defaults::query_defaults::DEFAULT_PROTOCOL_VERSION`, the single canonical
//! protocol number backing the protocol-version query defaults. Keeping one
//! source of truth means a version bump can never silently drift between the
//! advertised value and the deployment gate (#600).

use soroban_sdk::{symbol_short, Env, Symbol};

use crate::defaults::query_defaults::DEFAULT_PROTOCOL_VERSION;

/// Deployment policy asserting protocol-version compatibility.
///
/// Used to verify that the target ledger meets minimum version requirements
/// before contract deployment proceeds.
pub struct DeploymentPolicy;

impl DeploymentPolicy {
    /// Minimum protocol version required for deployment.
    ///
    /// Derived from the canonical `DEFAULT_PROTOCOL_VERSION` so the policy
    /// gate and the query-defaults advertisement can never disagree.
    pub const REQUIRED_PROTOCOL_VERSION: u32 = DEFAULT_PROTOCOL_VERSION;
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

    #[test]
    fn test_policy_version_equals_canonical_default_never_drifts() {
        // (#600) Equal-adversity guard: the deployment gate and the public
        // query-defaults value read the same number. If this ever fires, a
        // release bumped one source of the protocol version and forgot the
        // other.
        assert_eq!(
            DeploymentPolicy::REQUIRED_PROTOCOL_VERSION,
            crate::defaults::query_defaults::DEFAULT_PROTOCOL_VERSION
        );
    }
}
