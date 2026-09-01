//! Executable specification for the contract's pure, documented semantics.
//!
//! # Why this module exists
//!
//! The invariants that govern [`SLACalculatorContract::compute_result`] and
//! [`SLACalculatorContract::validate_config`] were, until now, written down in
//! three places that could drift independently: the doc comments on the
//! functions themselves, the prose in `docs/` and `README.md`, and the
//! hand-maintained golden vectors in `parity_tests`. The cargo-fuzz targets in
//! `fuzz/` had no way to reference any of them, so they could only assert
//! "did not panic".
//!
//! This module is the answer to *where the spec lives*: a single, `no_std`,
//! dependency-free restatement of the documented rules that
//!
//! * every consumer imports rather than re-deriving,
//! * is compiled and unit-tested by `cargo test --lib` (so it cannot rot
//!   silently even though the fuzz crate builds separately), and
//! * is reachable from outside the crate — the fuzz targets in `fuzz/` link
//!   against it as `apexchainx_calculator::spec`.
//!
//! # Independence requirement
//!
//! **The functions here deliberately do not call the contract implementation.**
//! They are an independent restatement of the documented rules. That is the
//! whole point: `impl == spec` is only a meaningful assertion when the two
//! sides are written separately. A "simplification" that makes any function
//! below delegate to [`crate::SLACalculatorContract`] or
//! [`crate::calculation`] destroys every test and fuzz target built on this
//! module. Do not do it.
//!
//! # Authority policy — which statement wins
//!
//! When [`crate::fuzz_spec`] or a unit test reports `impl != spec`, exactly one
//! of three things is true, and they are resolved in this order:
//!
//! 1. **The implementation regressed.** Fix the implementation. This is the
//!    outcome the fuzz targets exist to produce.
//! 2. **The spec restatement is wrong** (a transcription slip in this file).
//!    Fix this file. Nothing else changes.
//! 3. **The behaviour changed deliberately** and the documented rule is now
//!    stale. Then, and only then, update this module — *and* the prose docs it
//!    cites, *and* the parity baseline — in the same commit, and record the
//!    change in `CHANGELOG.md`.
//!
//! Case 3 is a reviewed, deliberate break. It is never correct to edit this
//! file solely to make a red build go green; see `docs/FUZZING_GUARANTEES.md`.
//!
//! # What is specified here
//!
//! | Area | Functions | Prose source |
//! |---|---|---|
//! | Config validation | [`expected_validate_config`] | `docs/config-validation.md` |
//! | SLA outcome | [`expected_compute_result`] | `SLACalculatorContract::compute_result` |
//! | History pagination | [`expected_page_end`], [`expected_has_more`] | `docs/HISTORY_PAGINATION_POLICY.md` |

use soroban_sdk::{symbol_short, Symbol};

use crate::{SLAConfig, SLAError};

// ─── General parameter bounds (validate_config, step 2-4) ───────────────────

/// Smallest accepted `threshold_minutes`. `0` is rejected.
pub const THRESHOLD_MIN: u32 = 1;
/// Largest accepted `threshold_minutes` (24 hours).
pub const THRESHOLD_MAX: u32 = 1440;
/// Smallest accepted `penalty_per_minute`. Zero and negatives are rejected.
pub const PENALTY_MIN: i128 = 1;
/// Largest accepted `penalty_per_minute`.
pub const PENALTY_MAX: i128 = 10_000;
/// Smallest accepted `reward_base`. Zero and negatives are rejected.
pub const REWARD_MIN: i128 = 1;
/// Largest accepted `reward_base`.
pub const REWARD_MAX: i128 = 100_000;

// ─── Severity-specific bounds (validate_config, step 5) ─────────────────────

/// `critical` may not exceed a one-hour threshold.
pub const CRITICAL_THRESHOLD_MAX: u32 = 60;
/// `critical` must penalise at least this much per minute.
pub const CRITICAL_PENALTY_MIN: i128 = 50;
/// `high` may not exceed a two-hour threshold.
pub const HIGH_THRESHOLD_MAX: u32 = 120;
/// `high` must penalise at least this much per minute.
pub const HIGH_PENALTY_MIN: i128 = 25;
/// `medium` may not exceed a four-hour threshold.
pub const MEDIUM_THRESHOLD_MAX: u32 = 240;
/// `medium` must penalise at least this much per minute.
pub const MEDIUM_PENALTY_MIN: i128 = 10;
/// `low` is capped from above instead of from below, and has no
/// severity-specific threshold ceiling beyond [`THRESHOLD_MAX`].
pub const LOW_PENALTY_MAX: i128 = 100;

// ─── Reward tier boundaries (compute_result, met branch) ────────────────────

/// A performance ratio strictly below this earns the `top` tier.
pub const TIER_TOP_RATIO_EXCLUSIVE: u64 = 50;
/// A performance ratio strictly below this (and not `top`) earns `excel`.
pub const TIER_EXCEL_RATIO_EXCLUSIVE: u64 = 75;
/// Reward multiplier, in percent, for the `top` tier.
pub const MULTIPLIER_TOP: i128 = 200;
/// Reward multiplier, in percent, for the `excel` tier.
pub const MULTIPLIER_EXCEL: i128 = 150;
/// Reward multiplier, in percent, for the `good` tier.
pub const MULTIPLIER_GOOD: i128 = 100;

// ─── Result symbols ─────────────────────────────────────────────────────────

/// `status` when `mttr_minutes <= threshold_minutes`.
pub const STATUS_MET: Symbol = symbol_short!("met");
/// `status` when `mttr_minutes > threshold_minutes`.
pub const STATUS_VIOLATED: Symbol = symbol_short!("viol");
/// `payment_type` paired with [`STATUS_MET`].
pub const PAYMENT_REWARD: Symbol = symbol_short!("rew");
/// `payment_type` paired with [`STATUS_VIOLATED`].
pub const PAYMENT_PENALTY: Symbol = symbol_short!("pen");
/// `rating` for the `top` reward tier.
pub const RATING_TOP: Symbol = symbol_short!("top");
/// `rating` for the `excel` reward tier.
pub const RATING_EXCELLENT: Symbol = symbol_short!("excel");
/// `rating` for the `good` reward tier.
pub const RATING_GOOD: Symbol = symbol_short!("good");
/// `rating` always attached to a violation.
pub const RATING_POOR: Symbol = symbol_short!("poor");

/// The four canonical severity symbols, in canonical (index) order.
pub const CANONICAL_SEVERITIES: [Symbol; 4] = [
    symbol_short!("critical"),
    symbol_short!("high"),
    symbol_short!("medium"),
    symbol_short!("low"),
];

/// The documented outcome of a successful [`expected_compute_result`].
///
/// Mirrors the outcome-bearing fields of [`crate::SLAResult`]. The pass-through
/// fields (`outage_id`, `mttr_minutes`, `threshold_minutes`,
/// `config_version_hash`, `recorded_at`) are not modelled here because the
/// spec for them is "returned unchanged", which callers assert directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedOutcome {
    /// [`STATUS_MET`] or [`STATUS_VIOLATED`].
    pub status: Symbol,
    /// [`PAYMENT_REWARD`] or [`PAYMENT_PENALTY`].
    pub payment_type: Symbol,
    /// One of the four rating symbols.
    pub rating: Symbol,
    /// Strictly positive for a reward, strictly negative for a penalty.
    pub amount: i128,
}

/// Returns `true` when `severity` is one of the four canonical severities.
pub fn is_canonical_severity(severity: &Symbol) -> bool {
    CANONICAL_SEVERITIES.iter().any(|s| s == severity)
}

/// The documented result of `validate_config`, restated independently.
///
/// # Error precedence
///
/// Checks are evaluated in a fixed order and the **first** failure wins. A
/// config that violates several rules always reports the earliest one, so the
/// error code is a deterministic function of the inputs — not an artefact of
/// evaluation order:
///
/// 1. `severity` must be canonical → [`SLAError::InvalidSeverity`]
/// 2. `threshold_minutes` in `[1, 1440]` → [`SLAError::InvalidThreshold`]
/// 3. `penalty_per_minute` in `[1, 10_000]` → [`SLAError::InvalidPenalty`]
/// 4. `reward_base` in `[1, 100_000]` → [`SLAError::InvalidReward`]
/// 5. severity-specific bounds → [`SLAError::InvalidThreshold`] /
///    [`SLAError::InvalidPenalty`]
/// 6. cross-parameter consistency `penalty * 3 < reward * 2`
///    → [`SLAError::InvalidReward`]
///
/// Step 6 encodes "rewards must materially exceed penalties" (`penalty * 1.5 <
/// reward`) in integer arithmetic so meeting the SLA is always financially
/// better than absorbing penalties for a minor overrun.
///
/// **Note:** Cross-severity threshold ordering (`critical <= high <= medium <=
/// low`) is enforced by `validate_cross_severity_threshold_ordering` in the
/// `set_config` path, not here, because it requires access to stored state.
pub fn expected_validate_config(
    severity: &Symbol,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) -> Result<(), SLAError> {
    // 1 — severity membership.
    if !is_canonical_severity(severity) {
        return Err(SLAError::InvalidSeverity);
    }

    // 2-4 — general bounds, in declaration order.
    if !(THRESHOLD_MIN..=THRESHOLD_MAX).contains(&threshold_minutes) {
        return Err(SLAError::InvalidThreshold);
    }
    if !(PENALTY_MIN..=PENALTY_MAX).contains(&penalty_per_minute) {
        return Err(SLAError::InvalidPenalty);
    }
    if !(REWARD_MIN..=REWARD_MAX).contains(&reward_base) {
        return Err(SLAError::InvalidReward);
    }

    // 5 — severity-specific bounds. Threshold is checked before penalty within
    // each branch, matching the documented precedence.
    if *severity == CANONICAL_SEVERITIES[0] {
        if threshold_minutes > CRITICAL_THRESHOLD_MAX {
            return Err(SLAError::InvalidThreshold);
        }
        if penalty_per_minute < CRITICAL_PENALTY_MIN {
            return Err(SLAError::InvalidPenalty);
        }
    } else if *severity == CANONICAL_SEVERITIES[1] {
        if threshold_minutes > HIGH_THRESHOLD_MAX {
            return Err(SLAError::InvalidThreshold);
        }
        if penalty_per_minute < HIGH_PENALTY_MIN {
            return Err(SLAError::InvalidPenalty);
        }
    } else if *severity == CANONICAL_SEVERITIES[2] {
        if threshold_minutes > MEDIUM_THRESHOLD_MAX {
            return Err(SLAError::InvalidThreshold);
        }
        if penalty_per_minute < MEDIUM_PENALTY_MIN {
            return Err(SLAError::InvalidPenalty);
        }
    } else if penalty_per_minute > LOW_PENALTY_MAX {
        // `low` — capped from above, no severity-specific threshold ceiling.
        return Err(SLAError::InvalidPenalty);
    }

    // 6 — cross-parameter consistency. Steps 3 and 4 have already bounded both
    // operands, so neither product can overflow here; the implementation's
    // `checked_mul` guards are belt-and-braces and map to `InvalidReward` too.
    let scaled_penalty = match penalty_per_minute.checked_mul(3) {
        Some(v) => v,
        None => return Err(SLAError::InvalidReward),
    };
    let scaled_reward = match reward_base.checked_mul(2) {
        Some(v) => v,
        None => return Err(SLAError::InvalidReward),
    };
    if scaled_penalty >= scaled_reward {
        return Err(SLAError::InvalidReward);
    }

    Ok(())
}

/// The documented outcome of `compute_result`, restated independently.
///
/// # Boundary
///
/// `mttr_minutes == threshold_minutes` is **met**, not violated: the threshold
/// is the largest still-compliant repair time. Violation requires strictly
/// `mttr_minutes > threshold_minutes`.
///
/// # Violation branch
///
/// `overtime = mttr_minutes - threshold_minutes` (at least 1), and
/// `amount = -(overtime * penalty_per_minute)`. Both the multiplication and the
/// negation are checked; either overflowing yields
/// [`SLAError::InvalidPenaltyAmount`]. A non-negative result — which a
/// zero or negative `penalty_per_minute` would produce — is also
/// [`SLAError::InvalidPenaltyAmount`], so a stored penalty is **always
/// strictly negative**. `rating` is always [`RATING_POOR`].
///
/// # Met branch
///
/// `performance_ratio = (mttr_minutes * 100) / threshold_minutes`, in `u64`, so
/// it cannot overflow for any `u32` input. A zero threshold divides to a
/// sentinel ratio of `0` rather than panicking. The ratio selects a tier:
///
/// | Ratio | Multiplier | Rating |
/// |---|---|---|
/// | `< 50` | 200% | [`RATING_TOP`] |
/// | `50..75` | 150% | [`RATING_EXCELLENT`] |
/// | `>= 75` | 100% | [`RATING_GOOD`] |
///
/// `amount = (reward_base * multiplier) / 100`, with the multiplication
/// checked ([`SLAError::InvalidRewardAmount`] on overflow) and the division
/// Euclidean. A result that is not strictly positive is also
/// [`SLAError::InvalidRewardAmount`], so a stored reward is **always strictly
/// positive**.
pub fn expected_compute_result(mttr_minutes: u32, cfg: &SLAConfig) -> Result<ExpectedOutcome, SLAError> {
    let threshold = cfg.threshold_minutes;

    if mttr_minutes > threshold {
        let overtime = (mttr_minutes - threshold) as i128;
        let penalty = match overtime.checked_mul(cfg.penalty_per_minute) {
            Some(v) => v,
            None => return Err(SLAError::InvalidPenaltyAmount),
        };
        let amount = match penalty.checked_neg() {
            Some(v) => v,
            None => return Err(SLAError::InvalidPenaltyAmount),
        };
        if amount >= 0 {
            return Err(SLAError::InvalidPenaltyAmount);
        }
        return Ok(ExpectedOutcome {
            status: STATUS_VIOLATED,
            payment_type: PAYMENT_PENALTY,
            rating: RATING_POOR,
            amount,
        });
    }

    // A zero threshold divides to a sentinel ratio of 0 rather than panicking,
    // matching the implementation's `checked_div(..).unwrap_or(0)`.
    let performance_ratio = (mttr_minutes as u64 * 100)
        .checked_div(threshold as u64)
        .unwrap_or(0);
    let (multiplier, rating) = if performance_ratio < TIER_TOP_RATIO_EXCLUSIVE {
        (MULTIPLIER_TOP, RATING_TOP)
    } else if performance_ratio < TIER_EXCEL_RATIO_EXCLUSIVE {
        (MULTIPLIER_EXCEL, RATING_EXCELLENT)
    } else {
        (MULTIPLIER_GOOD, RATING_GOOD)
    };

    let amount = match cfg.reward_base.checked_mul(multiplier) {
        Some(v) => v.div_euclid(100),
        None => return Err(SLAError::InvalidRewardAmount),
    };
    if amount <= 0 {
        return Err(SLAError::InvalidRewardAmount);
    }

    Ok(ExpectedOutcome {
        status: STATUS_MET,
        payment_type: PAYMENT_REWARD,
        rating,
        amount,
    })
}

/// The exclusive end index of the page returned for `(offset, limit)` over a
/// history of `len` entries.
///
/// Restates `docs/HISTORY_PAGINATION_POLICY.md`: `limit` is first clamped to
/// [`crate::history::MAX_PAGE_SIZE`], then
/// `end = min(saturating_add(offset, clamped_limit), len)`. Saturating
/// addition means no `u32` pair can wrap into a wrong slice. The page is
/// `history[offset..end]`, which is empty whenever `offset >= len` or the
/// clamped `limit` is `0`.
pub fn expected_page_end(offset: u32, limit: u32, len: u32) -> u32 {
    let limit = if limit > crate::history::MAX_PAGE_SIZE {
        crate::history::MAX_PAGE_SIZE
    } else {
        limit
    };
    let end = offset.saturating_add(limit);
    if end > len {
        len
    } else {
        end
    }
}

/// The number of entries the page for `(offset, limit)` contains.
pub fn expected_page_len(offset: u32, limit: u32, len: u32) -> u32 {
    expected_page_end(offset, limit, len).saturating_sub(offset)
}

/// `has_more` as reported by `get_history_page_with_meta`.
///
/// Mirrors the contract implementation (lib.rs): True exactly when the
/// requested range stops before the end of history **and** `limit > 0`. A
/// `limit == 0` page is empty and signals that there is nothing further to
/// advance to, so it reports `false`.
pub fn expected_has_more(offset: u32, limit: u32, len: u32) -> bool {
    if limit == 0 {
        false
    } else {
        expected_page_end(offset, limit, len) < len
    }
}
