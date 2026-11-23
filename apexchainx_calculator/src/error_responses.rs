//! Error-classification helpers for backend bridge consumers.
//!
//! Each function tests whether a given `SLAError` matches a specific variant.
//! Backend consumers use these for error-type dispatch without matching on
//! the raw enum discriminant.

use crate::SLAError;

/// Returns `true` if the error is `AlreadyInitialized`.
pub fn is_already_initialized(err: &SLAError) -> bool {
    matches!(err, SLAError::AlreadyInitialized)
}

/// Returns `true` if the error is `NotInitialized`.
pub fn is_not_initialized(err: &SLAError) -> bool {
    matches!(err, SLAError::NotInitialized)
}

/// Returns `true` if the error is `Unauthorized`.
pub fn is_unauthorized(err: &SLAError) -> bool {
    matches!(err, SLAError::Unauthorized)
}

/// Returns `true` if the error is `ConfigNotFound`.
pub fn is_config_not_found(err: &SLAError) -> bool {
    matches!(err, SLAError::ConfigNotFound)
}

/// Returns `true` if the error is `VersionMismatch`.
pub fn is_version_mismatch(err: &SLAError) -> bool {
    matches!(err, SLAError::VersionMismatch)
}

/// Returns `true` if the error is `ContractPaused`.
pub fn is_contract_paused(err: &SLAError) -> bool {
    matches!(err, SLAError::ContractPaused)
}

/// Returns `true` if the error is `NoPendingTransfer`.
pub fn is_no_pending_transfer(err: &SLAError) -> bool {
    matches!(err, SLAError::NoPendingTransfer)
}

/// Returns `true` if the error is `InvalidThreshold`.
pub fn is_invalid_threshold(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidThreshold)
}

/// Returns `true` if the error is `InvalidPenalty`.
pub fn is_invalid_penalty(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidPenalty)
}

/// Returns `true` if the error is `InvalidReward`.
pub fn is_invalid_reward(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidReward)
}

/// Returns `true` if the error is `InvalidSeverity`.
pub fn is_invalid_severity(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidSeverity)
}

/// Returns `true` if the error is `RetentionLimitOutOfRange`.
pub fn is_retention_limit_out_of_range(err: &SLAError) -> bool {
    matches!(err, SLAError::RetentionLimitOutOfRange)
}

/// Returns `true` if the error is `DuplicateOutageInput`.
pub fn is_duplicate_outage_input(err: &SLAError) -> bool {
    matches!(err, SLAError::DuplicateOutageInput)
}

/// Returns `true` if the error is `InvalidPenaltyAmount`.
pub fn is_invalid_penalty_amount(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidPenaltyAmount)
}

/// Returns `true` if the error is `InvalidRewardAmount`.
pub fn is_invalid_reward_amount(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidRewardAmount)
}

/// Returns `true` if the error is `ConfigFrozen`.
pub fn is_config_frozen(err: &SLAError) -> bool {
    matches!(err, SLAError::ConfigFrozen)
}

/// Returns `true` if the error is `InvalidInput`.
pub fn is_invalid_input(err: &SLAError) -> bool {
    matches!(err, SLAError::InvalidInput)
}

/// Returns `true` if the error is `OutageRecalcLimit`.
pub fn is_severity_not_in_set(err: &SLAError) -> bool {
    matches!(err, SLAError::SeverityNotInSet)
}

/// Returns `true` if the error is `OutageRecalcLimit`.
pub fn is_outage_recalc_limit(err: &SLAError) -> bool {
    matches!(err, SLAError::OutageRecalcLimit)
}

/// Returns `true` if the error is `ProposalExpired`.
pub fn is_proposal_expired(err: &SLAError) -> bool {
    matches!(err, SLAError::ProposalExpired)
}

/// Returns `true` if the error is `AdminRenounced`. (#406)
pub fn is_admin_renounced(err: &SLAError) -> bool {
    matches!(err, SLAError::AdminRenounced)
}

/// Returns `true` if the error is `ExposureOverflow`. (SC-W5-047)
pub fn is_exposure_overflow(err: &SLAError) -> bool {
    matches!(err, SLAError::ExposureOverflow)
}
