//! Two-step admin and operator transfer governance.//!
//! This module implements the two-step handoff pattern for admin and operator
//! role transfers, plus admin renounce and single-step operator assignment.
//! All functions require the appropriate role authorization and emit versioned
//! governance events for backend audit trails. Re-proposing a role while a
//! proposal is pending replaces the candidate and emits a supersession event
//! (`adm_sup`/`op_sup`) so the pending-slot history stays reconstructable.
//!
//! # Operator Transfer Paths
//!
//! There are two distinct ways to change the operator role:
//!
//! ## Two-step (canonical, preferred)
//!
//! 1. Admin calls `propose_operator` → emits `op_prop` event, stores pending operator.
//! 2. Proposed new operator calls `accept_operator` → emits `op_acc` event, operator is set.
//!
//! This is the **recommended** path because it requires the new operator's explicit
//! consent (`accept_operator` demands `require_auth()` from the proposed address).
//! The event trail (`op_prop` → `op_acc`) provides a clear audit sequence that
//! backend consumers can follow.
//!
//! ## Single-step (legacy break-glass)
//!
//! Admin calls `set_operator` → emits `op_set` event, operator is set immediately.
//!
//! This path does **not** require the new operator's consent or signature. It was
//! introduced in the #28-era before the two-step handoff (#64) existed and is
//! retained as a break-glass mechanism for emergency operator replacement.
//!
//! **Security note:** Because `set_operator` bypasses new-operator consent, a
//! compromised admin could install an operator silently. Consumers that need to
//! distinguish consented from non-consented operator changes must check the
//! event name: `op_set` = single-step (no consent), `op_prop`+`op_acc` =
//! two-step (consent obtained).
//!
//! Both paths are gated by `require_not_frozen()` and the admin role check.
//!
//! # Operator Transfer Paths
//!
//! There are two distinct ways to change the operator role:
//!
//! ## Two-step (canonical, preferred)
//!
//! 1. Admin calls `propose_operator` → emits `op_prop` event, stores pending operator.
//! 2. Proposed new operator calls `accept_operator` → emits `op_acc` event, operator is set.
//!
//! This is the **recommended** path because it requires the new operator's explicit
//! consent (`accept_operator` demands `require_auth()` from the proposed address).
//! The event trail (`op_prop` → `op_acc`) provides a clear audit sequence that
//! backend consumers can follow.
//!
//! ## Single-step (legacy break-glass)
//!
//! Admin calls `set_operator` → emits `op_set` event, operator is set immediately.
//!
//! This path does **not** require the new operator's consent or signature. It was
//! introduced in the #28-era before the two-step handoff (#64) existed and is
//! retained as a break-glass mechanism for emergency operator replacement.
//!
//! **Security note:** Because `set_operator` bypasses new-operator consent, a
//! compromised admin could install an operator silently. Consumers that need to
//! distinguish consented from non-consented operator changes must check the
//! event name: `op_set` = single-step (no consent), `op_prop`+`op_acc` =
//! two-step (consent obtained).
//!

use soroban_sdk::{Address, Env, Symbol};

use crate::{
    SLAError, ADMIN_KEY, EVENT_ADMIN_ACC, EVENT_ADMIN_CAN, EVENT_ADMIN_PROP, EVENT_ADMIN_REN,
    EVENT_ADMIN_SUP, EVENT_OP_ACC, EVENT_OP_CAN, EVENT_OP_PROP, EVENT_OP_SET, EVENT_OP_SUP, EVENT_VERSION,
    OPERATOR_KEY, PENDING_ADMIN_KEY, PENDING_ADMIN_TS_KEY, PENDING_OP_KEY, PENDING_OP_TS_KEY,
};

/// Window (in ledger seconds) after which a pending proposal expires.
const PROPOSAL_EXPIRY_WINDOW: u64 = 90 * 24 * 60 * 60;

/// Requires that the stored proposal is still within its expiry window.
fn require_proposal_valid(env: &Env, ts_key: Symbol) -> Result<(), SLAError> {
    let proposed: u64 = env
        .storage()
        .instance()
        .get(&ts_key)
        .ok_or(SLAError::NoPendingTransfer)?;
    let now = env.ledger().timestamp();
    if now.saturating_sub(proposed) > PROPOSAL_EXPIRY_WINDOW {
        return Err(SLAError::ProposalExpired);
    }
    Ok(())
}

/// Proposes a new admin. The current admin initiates; the new admin must
/// call `accept_admin` to complete the transfer.
pub fn propose_admin(env: &Env, caller: &Address, new_admin: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    let superseded: Option<Address> = env.storage().instance().get(&PENDING_ADMIN_KEY);
    env.storage().instance().set(&PENDING_ADMIN_KEY, new_admin);
    env.storage()
        .instance()
        .set(&PENDING_ADMIN_TS_KEY, &env.ledger().timestamp());
    if let Some(previous) = superseded {
        // A re-proposal supersedes the pending candidate. Publish the
        // supersession first so the event stream records the replacement
        // before the new proposal, letting consumers reconstruct the slot.
        env.events().publish(
            (EVENT_ADMIN_SUP, EVENT_VERSION, caller.clone()),
            (previous, new_admin.clone()),
        );
    }
    env.events().publish(
        (EVENT_ADMIN_PROP, EVENT_VERSION, caller.clone()),
        (new_admin.clone(),),
    );
    Ok(())
}

/// Accepts a pending admin transfer. Must be called by the proposed new admin.
pub fn accept_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    caller.require_auth();
    let pending: Address = env
        .storage()
        .instance()
        .get(&PENDING_ADMIN_KEY)
        .ok_or(SLAError::NoPendingTransfer)?;
    require_proposal_valid(env, PENDING_ADMIN_TS_KEY)?;
    if *caller != pending {
        return Err(SLAError::Unauthorized);
    }
    env.storage().instance().set(&ADMIN_KEY, caller);
    env.storage().instance().remove(&PENDING_ADMIN_KEY);
    env.storage().instance().remove(&PENDING_ADMIN_TS_KEY);
    env.events()
        .publish((EVENT_ADMIN_ACC, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

/// Cancels a pending admin proposal. Only the current admin may cancel.
pub fn cancel_admin_proposal(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    if !env.storage().instance().has(&PENDING_ADMIN_KEY) {
        return Err(SLAError::NoPendingTransfer);
    }
    env.storage().instance().remove(&PENDING_ADMIN_KEY);
    env.storage().instance().remove(&PENDING_ADMIN_TS_KEY);
    env.events()
        .publish((EVENT_ADMIN_CAN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

/// Returns the pending admin address, if any.
pub fn get_pending_admin(env: &Env) -> Result<Option<Address>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_ADMIN_KEY))
}

/// Proposes a new operator. The current admin initiates; the new operator
/// must call `accept_operator` to complete the handoff.
///
/// This is step 1 of the canonical two-step operator transfer. The proposed
/// operator must call `accept_operator` (requiring their explicit consent via
/// `require_auth()`) before the transfer completes. This ensures the new
/// operator agrees to assume the role.
///
/// Emits an `op_prop` event carrying `(new_operator,)` in the payload.
/// If a previous proposal exists, it is silently overwritten.
pub fn propose_operator(env: &Env, caller: &Address, new_operator: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    let superseded: Option<Address> = env.storage().instance().get(&PENDING_OP_KEY);
    env.storage().instance().set(&PENDING_OP_KEY, new_operator);
    env.storage()
        .instance()
        .set(&PENDING_OP_TS_KEY, &env.ledger().timestamp());
    if let Some(previous) = superseded {
        env.events().publish(
            (EVENT_OP_SUP, EVENT_VERSION, caller.clone()),
            (previous, new_operator.clone()),
        );
    }
    env.events().publish(
        (EVENT_OP_PROP, EVENT_VERSION, caller.clone()),
        (new_operator.clone(),),
    );
    Ok(())
}

/// Accepts a pending operator handoff. Must be called by the proposed new operator.
///
/// This is step 2 of the canonical two-step operator transfer. It requires
/// the caller to be the pending operator (verified by `require_auth()` +
/// address match), ensuring the new operator explicitly consents to the role.
///
/// On success, the operator is set, the pending proposal is cleared, and an
/// `op_acc` event is emitted. Consumers see a `op_prop` → `op_acc` pair in
/// the event stream, confirming a consented handoff.
pub fn accept_operator(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    caller.require_auth();
    let pending: Address = env
        .storage()
        .instance()
        .get(&PENDING_OP_KEY)
        .ok_or(SLAError::NoPendingTransfer)?;
    require_proposal_valid(env, PENDING_OP_TS_KEY)?;
    if *caller != pending {
        return Err(SLAError::Unauthorized);
    }
    env.storage().instance().set(&OPERATOR_KEY, caller);
    env.storage().instance().remove(&PENDING_OP_KEY);
    env.storage().instance().remove(&PENDING_OP_TS_KEY);
    env.events()
        .publish((EVENT_OP_ACC, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

/// Cancels a pending operator proposal. Only the current admin may cancel.
pub fn cancel_operator_proposal(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    if !env.storage().instance().has(&PENDING_OP_KEY) {
        return Err(SLAError::NoPendingTransfer);
    }
    env.storage().instance().remove(&PENDING_OP_KEY);
    env.storage().instance().remove(&PENDING_OP_TS_KEY);
    env.events()
        .publish((EVENT_OP_CAN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

/// Returns the pending operator address, if any.
pub fn get_pending_operator(env: &Env) -> Result<Option<Address>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_OP_KEY))
}

/// Permanently renounces admin authority. Irreversible.
pub fn renounce_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    env.storage().instance().set(&crate::ADMIN_RENOUNCED_KEY, &true);
    env.storage().instance().remove(&ADMIN_KEY);
    env.storage().instance().remove(&PENDING_ADMIN_KEY);
    env.storage().instance().remove(&PENDING_ADMIN_TS_KEY);
    if env.storage().instance().has(&PENDING_OP_KEY) {
        // Renounce must invalidate every pending governance proposal: an
        // adminless contract must not allow a stale operator handoff to fire.
        env.storage().instance().remove(&PENDING_OP_KEY);
        env.storage().instance().remove(&PENDING_OP_TS_KEY);
        env.events()
            .publish((EVENT_OP_CAN, EVENT_VERSION, caller.clone()), ());
    }
    env.events()
        .publish((EVENT_ADMIN_REN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

/// Replaces the operator address directly (single-step, admin only).
///
/// # Single-step operator assignment (legacy break-glass path)
///
/// This function replaces the operator **immediately** without requiring the
/// new operator's consent. It is a legacy path retained from the #28 era,
/// before the two-step `propose_operator`/`accept_operator` handoff (#64)
/// was introduced.
///
/// ## Consent semantics
///
/// Unlike the two-step path (`op_prop` → `op_acc`), this function does **not**
/// call `require_auth()` on the new operator address. The new operator never
/// explicitly consents to the role change. The admin's authorization is the
/// only consent required.
///
/// ## Event trail
///
/// Emits a single `op_set` event carrying `(new_operator,)` in the payload.
/// This differs from the two-step path which emits `op_prop` on proposal and
/// `op_acc` on acceptance. Backend consumers can distinguish the two paths by
/// the event name: `op_set` = non-consented direct set, `op_prop`+`op_acc` =
/// consented handoff.
///
/// ## Intended use
///
/// Use `set_operator` as a **break-glass** mechanism when the two-step flow is
/// impractical (e.g., the current operator's key is compromised and the new
/// operator cannot be reached for `accept_operator`). For routine operator
/// rotations, prefer the two-step path.
///
/// ## Pending-slot interaction
///
/// This function does **not** clear or interact with any pending operator
/// proposal (`PENDING_OP_KEY`). If a two-step proposal is pending when
/// `set_operator` is called, the pending proposal remains in storage and
/// can still be accepted. Admins should explicitly cancel any pending
/// proposal before using this path to avoid ambiguity.
///
/// ## Gating
///
/// - Requires admin role (`require_admin`).
/// - Blocked when config is frozen (`require_not_frozen`).
/// - Requires storage version match (`check_version`).
pub fn set_operator(env: &Env, caller: &Address, new_operator: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::config_freeze::require_not_frozen(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    if env.storage().instance().has(&PENDING_OP_KEY) {
        // A direct assignment invalidates any pending operator proposal so a
        // stale handoff can never override the admin's single-step decision.
        env.storage().instance().remove(&PENDING_OP_KEY);
        env.storage().instance().remove(&PENDING_OP_TS_KEY);
        env.events()
            .publish((EVENT_OP_CAN, EVENT_VERSION, caller.clone()), ());
    }
    env.storage().instance().set(&OPERATOR_KEY, new_operator);
    env.events().publish(
        (EVENT_OP_SET, EVENT_VERSION, caller.clone()),
        (new_operator.clone(),),
    );
    Ok(())
}
