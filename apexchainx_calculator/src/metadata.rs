use soroban_sdk::{Address, Env, String};

use crate::{
    SLAError, PauseInfo,
    PAUSED_KEY, PAUSE_INFO_KEY, MAX_REASON_LEN, EVENT_VERSION, EVENT_PAUSED, EVENT_UNPAUSED,
};

pub fn pause(env: &Env, caller: &Address, reason: String) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    if reason.len() > MAX_REASON_LEN as u32 {
        return Err(SLAError::InvalidInput);
    }

    let paused_at = env.ledger().timestamp();
    env.storage().instance().set(&PAUSED_KEY, &true);
    env.storage().instance().set(
        &PAUSE_INFO_KEY,
        &PauseInfo {
            reason,
            paused_at,
            paused_by: caller.clone(),
        },
    );
    env.events()
        .publish((EVENT_PAUSED, EVENT_VERSION, caller.clone()), (true,));
    Ok(())
}

pub fn unpause(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    env.storage().instance().set(&PAUSED_KEY, &false);
    env.storage().instance().remove(&PAUSE_INFO_KEY);
    env.events()
        .publish((EVENT_UNPAUSED, EVENT_VERSION, caller.clone()), (false,));
    Ok(())
}

pub fn is_paused(env: &Env) -> Result<bool, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PAUSED_KEY).unwrap_or(false))
}

pub fn get_pause_info(env: &Env) -> Result<Option<PauseInfo>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PAUSE_INFO_KEY))
}

pub fn require_not_paused(env: &Env) -> Result<(), SLAError> {
    let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
    if paused {
        return Err(SLAError::ContractPaused);
    }
    Ok(())
}
