use soroban_sdk::{Address, Env};

use crate::{
    AdminProposalState, OperatorProposalState, SLAError, ADMIN_KEY, EVENT_ADMIN_ACC, EVENT_ADMIN_CAN,
    EVENT_ADMIN_PROP, EVENT_ADMIN_REN, EVENT_OP_ACC, EVENT_OP_CAN, EVENT_OP_PROP, EVENT_OP_SET, EVENT_VERSION,
    OPERATOR_KEY, PENDING_ADMIN_KEY, PENDING_ADMIN_STATE_KEY, PENDING_OP_KEY, PENDING_OP_STATE_KEY,
};

pub fn propose_admin(env: &Env, caller: &Address, new_admin: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    // #97 – keep PENDING_ADMIN_KEY populated for backward compatibility
    // with the existing `get_pending_admin` reader.
    env.storage().instance().set(&PENDING_ADMIN_KEY, new_admin);
    // #97 – additionally stamp the full proposal state so dashboards can
    // show who proposed, when (ledger sequence), and to whom, in one read.
    env.storage().instance().set(
        &PENDING_ADMIN_STATE_KEY,
        &AdminProposalState {
            address: new_admin.clone(),
            proposed_at_ledger: env.ledger().sequence(),
            proposed_by: caller.clone(),
        },
    );
    env.events().publish(
        (EVENT_ADMIN_PROP, EVENT_VERSION, caller.clone()),
        (new_admin.clone(),),
    );
    Ok(())
}

pub fn accept_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    caller.require_auth();
    let pending: Address = env
        .storage()
        .instance()
        .get(&PENDING_ADMIN_KEY)
        .ok_or(SLAError::NoPendingTransfer)?;
    if *caller != pending {
        return Err(SLAError::Unauthorized);
    }
    env.storage().instance().set(&ADMIN_KEY, caller);
    env.storage().instance().remove(&PENDING_ADMIN_KEY);
    // #97 – clear the in-progress proposal state so the new reader mirrors
    // `get_pending_admin` becoming None.
    env.storage().instance().remove(&PENDING_ADMIN_STATE_KEY);
    env.events()
        .publish((EVENT_ADMIN_ACC, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn cancel_admin_proposal(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    if !env.storage().instance().has(&PENDING_ADMIN_KEY) {
        return Err(SLAError::NoPendingTransfer);
    }
    env.storage().instance().remove(&PENDING_ADMIN_KEY);
    // #97 – mirror `remove(PENDING_ADMIN_KEY)` so public reads agree.
    env.storage().instance().remove(&PENDING_ADMIN_STATE_KEY);
    env.events()
        .publish((EVENT_ADMIN_CAN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn get_pending_admin(env: &Env) -> Result<Option<Address>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_ADMIN_KEY))
}

/// #97 – Returns the full pending admin proposal state authored by
/// `propose_admin`, or None when no proposal is in flight.
pub fn get_pending_admin_state(env: &Env) -> Result<Option<AdminProposalState>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_ADMIN_STATE_KEY))
}

pub fn propose_operator(env: &Env, caller: &Address, new_operator: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    // #97 – keep PENDING_OP_KEY populated for backward compatibility with
    // the existing `get_pending_operator` reader.
    env.storage().instance().set(&PENDING_OP_KEY, new_operator);
    // #97 – additionally stamp the full proposal state so dashboards can
    // show who proposed, when (ledger sequence), and to whom, in one read.
    env.storage().instance().set(
        &PENDING_OP_STATE_KEY,
        &OperatorProposalState {
            address: new_operator.clone(),
            proposed_at_ledger: env.ledger().sequence(),
            proposed_by: caller.clone(),
        },
    );
    env.events().publish(
        (EVENT_OP_PROP, EVENT_VERSION, caller.clone()),
        (new_operator.clone(),),
    );
    Ok(())
}

pub fn accept_operator(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    caller.require_auth();
    let pending: Address = env
        .storage()
        .instance()
        .get(&PENDING_OP_KEY)
        .ok_or(SLAError::NoPendingTransfer)?;
    if *caller != pending {
        return Err(SLAError::Unauthorized);
    }
    env.storage().instance().set(&OPERATOR_KEY, caller);
    env.storage().instance().remove(&PENDING_OP_KEY);
    // #97 – clear the in-progress proposal state so the new reader mirrors
    // `get_pending_operator` becoming None.
    env.storage().instance().remove(&PENDING_OP_STATE_KEY);
    env.events()
        .publish((EVENT_OP_ACC, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn cancel_operator_proposal(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    if !env.storage().instance().has(&PENDING_OP_KEY) {
        return Err(SLAError::NoPendingTransfer);
    }
    env.storage().instance().remove(&PENDING_OP_KEY);
    // #97 – mirror `remove(PENDING_OP_KEY)` so the new reader is consistent
    // with the legacy `get_pending_operator` reader.
    env.storage().instance().remove(&PENDING_OP_STATE_KEY);
    env.events()
        .publish((EVENT_OP_CAN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn get_pending_operator(env: &Env) -> Result<Option<Address>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_OP_KEY))
}

/// #97 – Returns the full pending operator proposal state authored by
/// `propose_operator`, or None when no proposal is in flight. Mirrors the
/// admin counterpart so dashboards can render operator proposals from a
/// single read.
pub fn get_pending_operator_state(
    env: &Env,
) -> Result<Option<OperatorProposalState>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_OP_STATE_KEY))
}

pub fn renounce_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    env.storage().instance().remove(&ADMIN_KEY);
    env.storage().instance().remove(&PENDING_ADMIN_KEY);
    // #97 – ensure the full proposal state is also cleared so
    // `get_pending_admin_state` returns None after renouncement.
    env.storage().instance().remove(&PENDING_ADMIN_STATE_KEY);
    env.events()
        .publish((EVENT_ADMIN_REN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn set_operator(env: &Env, caller: &Address, new_operator: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    env.storage().instance().set(&OPERATOR_KEY, new_operator);
    env.events().publish(
        (EVENT_OP_SET, EVENT_VERSION, caller.clone()),
        (new_operator.clone(),),
    );
    Ok(())
}
