use crate::SLAError;

pub fn is_already_initialized(err: &SLAError) -> bool {
    matches!(err, SLAError::AlreadyInitialized)
}

pub fn is_not_initialized(err: &SLAError) -> bool {
    matches!(err, SLAError::NotInitialized)
}

pub fn is_unauthorized(err: &SLAError) -> bool {
    matches!(err, SLAError::Unauthorized)
}

pub fn is_config_not_found(err: &SLAError) -> bool {
    matches!(err, SLAError::ConfigNotFound)
}

pub fn is_version_mismatch(err: &SLAError) -> bool {
    matches!(err, SLAError::VersionMismatch)
}

pub fn is_contract_paused(err: &SLAError) -> bool {
    matches!(err, SLAError::ContractPaused)
}

pub fn is_no_pending_transfer(err: &SLAError) -> bool {
    matches!(err, SLAError::NoPendingTransfer)
}

pub fn is_invalid_threshold(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidThreshold)
}

pub fn is_invalid_penalty(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidPenalty)
}

pub fn is_invalid_reward(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidReward)
}

pub fn is_invalid_severity(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidSeverity)
}

pub fn is_retention_limit_out_of_range(err: &SLAError) -> bool {
    matches!(err, SLAError::RetentionLimitOutOfRange)
}

pub fn is_duplicate_outage_input(err: &SLAError) -> bool {
    matches!(err, SLAError::DuplicateOutageInput)
}

pub fn is_invalid_penalty_amount(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidPenaltyAmount)
}

pub fn is_invalid_reward_amount(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidRewardAmount)
}

pub fn is_config_frozen(err: &SLAError) -> bool {
    matches!(err, SLAError::ConfigFrozen)
}

pub fn is_invalid_input(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidInput)
}
