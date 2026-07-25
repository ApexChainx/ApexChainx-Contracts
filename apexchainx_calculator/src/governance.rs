use soroban_sdk::{Address, Env};

use crate::{
    SLAError, ADMIN_KEY, PENDING_ADMIN_KEY, PENDING_OP_KEY, OPERATOR_KEY, EVENT_VERSION,
    EVENT_ADMIN_PROP, EVENT_ADMIN_ACC, EVENT_ADMIN_CAN, EVENT_ADMIN_REN, EVENT_OP_PROP,
    EVENT_OP_ACC, EVENT_OP_CAN, EVENT_OP_SET,
};

pub fn propose_admin(
    env: &Env,
    caller: &Address,
    new_admin: &Address,
) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    env.storage().instance().set(&PENDING_ADMIN_KEY, new_admin);
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
    env.events()
        .publish((EVENT_ADMIN_CAN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn get_pending_admin(env: &Env) -> Result<Option<Address>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_ADMIN_KEY))
}

pub fn propose_operator(
    env: &Env,
    caller: &Address,
    new_operator: &Address,
) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    env.storage().instance().set(&PENDING_OP_KEY, new_operator);
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
    env.events()
        .publish((EVENT_OP_CAN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn get_pending_operator(env: &Env) -> Result<Option<Address>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PENDING_OP_KEY))
}

pub fn renounce_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    env.storage().instance().remove(&ADMIN_KEY);
    env.storage().instance().remove(&PENDING_ADMIN_KEY);
    env.events()
        .publish((EVENT_ADMIN_REN, EVENT_VERSION, caller.clone()), ());
    Ok(())
}

pub fn set_operator(
    env: &Env,
    caller: &Address,
    new_operator: &Address,
) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    env.storage().instance().set(&OPERATOR_KEY, new_operator);
    env.events().publish(
        (EVENT_OP_SET, EVENT_VERSION, caller.clone()),
        (new_operator.clone(),),
    );
    Ok(())
}
