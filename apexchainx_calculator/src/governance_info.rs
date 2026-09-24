//! Queryable governance parameters (Issue #665).
//!
//! `PROPOSAL_EXPIRY_WINDOW` was private to `governance.rs`; backends wanting
//! to alert before a handoff lapses had to duplicate the 90-day arithmetic.
//! This module re-exports the window as a public queryable constant and
//! provides a helper that returns it alongside other governance posture info.

use soroban_sdk::{contracttype, Env};

/// The proposal expiry window in seconds (90 days).
/// Matches `PROPOSAL_EXPIRY_WINDOW` in `governance.rs`.
/// Exposed here so backends can query it without hardcoding the value.
pub const PROPOSAL_EXPIRY_WINDOW_SECS: u64 = 90 * 24 * 60 * 60;

/// Governance posture snapshot returned by `get_governance_info`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GovernanceInfo {
    /// Proposal expiry window in seconds.
    pub proposal_expiry_window_secs: u64,
    /// Whether a pending admin proposal currently exists.
    pub has_pending_admin: bool,
    /// Whether a pending operator proposal currently exists.
    pub has_pending_operator: bool,
}

/// Build a `GovernanceInfo` snapshot from current on-chain state.
pub fn get_governance_info(env: &Env) -> GovernanceInfo {
    let has_pending_admin = env
        .storage()
        .instance()
        .has(&crate::PENDING_ADMIN_KEY);
    let has_pending_operator = env
        .storage()
        .instance()
        .has(&crate::PENDING_OP_KEY);
    GovernanceInfo {
        proposal_expiry_window_secs: PROPOSAL_EXPIRY_WINDOW_SECS,
        has_pending_admin,
        has_pending_operator,
    }
}