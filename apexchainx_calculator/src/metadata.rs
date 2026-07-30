//! Contract pause/unpause lifecycle management.
//!
//! This module implements the pause mechanism that blocks state-changing
//! operations while preserving read-only access for monitoring.

use soroban_sdk::{Address, Env, String};

use crate::{
    PauseInfo, SLAError, EVENT_PAUSED, EVENT_UNPAUSED, EVENT_VERSION, MAX_REASON_LEN, PAUSED_KEY,
    PAUSE_INFO_KEY,
};

/// Pauses the contract, blocking state-changing operations. Admin only.
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

/// Unpauses the contract, restoring normal operation. Admin only.
pub fn unpause(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    env.storage().instance().set(&PAUSED_KEY, &false);
    env.storage().instance().remove(&PAUSE_INFO_KEY);
    env.events()
        .publish((EVENT_UNPAUSED, EVENT_VERSION, caller.clone()), (false,));
    Ok(())
}

/// Returns `true` when the contract is currently paused.
pub fn is_paused(env: &Env) -> Result<bool, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PAUSED_KEY).unwrap_or(false))
}

/// Returns pause metadata (reason, timestamp, caller) if currently paused.
pub fn get_pause_info(env: &Env) -> Result<Option<PauseInfo>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PAUSE_INFO_KEY))
}

/// Guards state-changing functions — returns `ContractPaused` if paused.
pub fn require_not_paused(env: &Env) -> Result<(), SLAError> {
    let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
    if paused {
        return Err(SLAError::ContractPaused);
    }
    Ok(())
}
