//! Extracted sub-step helpers for `calculate_sla` (Issue #653).
//!
//! `calculate_sla` previously mixed policy, history scan, dedup decision,
//! retention trim, stats, and event publication inline in one method body.
//! This module extracts the coherent sub-steps as documented helpers, keeping
//! `calculate_sla` as thin orchestration and making each phase independently
//! readable and testable.
//!
//! # Phases
//!
//! | Helper | Phase | Description |
//! |---|---|---|
//! | [`check_duplicate_policy`] | Dedup | Scans history for the outage, returns the stored entry and per-outage count |
//! | [`apply_retention_trim`] | Trim | Enforces the retention cap, drops the oldest entry when exceeded |
//! | [`DuplicateDecision`] | Types | Result type from the dedup scan |

use soroban_sdk::{Env, Symbol, Vec};

use crate::{SLAResult, MAX_RECALCS_PER_OUTAGE, HISTORY_KEY, RETENTION_LIMIT_KEY, MAX_HISTORY_SIZE};

/// Result of the duplicate/anti-spam scan performed at the start of `calculate_sla`.
pub enum DuplicateDecision {
    /// No prior entry for this outage — proceed with fresh calculation.
    New,
    /// Entry exists with the same config hash.
    /// The caller must check whether inputs match (idempotent replay)
    /// or differ (conflict → `DuplicateOutageInput`).
    ExistingUnderSameConfig {
        prev: SLAResult,
        stored_count: u32,
    },
    /// Entry exists but under a different config hash — treat as fresh.
    /// The caller must still check the anti-spam cap.
    ExistingUnderDifferentConfig {
        stored_count: u32,
    },
}

/// Scan history for `outage_id` and return the dedup decision.
///
/// Counts how many retained entries the outage already owns (anti-spam)
/// and returns the most recent one for hash/input comparison. This is the
/// entire history-scan cost of `calculate_sla` extracted into one place.
pub fn check_duplicate_policy(
    env: &Env,
    outage_id: &Symbol,
    config_version_hash: u64,
) -> DuplicateDecision {
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));

    let mut latest: Option<SLAResult> = None;
    let mut stored_count: u32 = 0;
    let mut same_config = false;

    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.outage_id == *outage_id {
            stored_count += 1;
            same_config = entry.config_version_hash == config_version_hash;
            latest = Some(entry);
        }
    }

    match latest {
        None => DuplicateDecision::New,
        Some(prev) if same_config => DuplicateDecision::ExistingUnderSameConfig {
            prev,
            stored_count,
        },
        Some(_) => DuplicateDecision::ExistingUnderDifferentConfig { stored_count },
    }
}

/// Enforce the retention cap on `history`, dropping the oldest entry if exceeded.
///
/// Extracted from the inline retention-trim block in `calculate_sla`. Does
/// not write to storage — the caller must persist the returned vector.
pub fn apply_retention_trim(env: &Env, mut history: Vec<SLAResult>) -> Vec<SLAResult> {
    let retention_limit: u32 = env
        .storage()
        .instance()
        .get(&RETENTION_LIMIT_KEY)
        .unwrap_or(MAX_HISTORY_SIZE);

    if history.len() > retention_limit {
        let excess = history.len() - retention_limit;
        let mut trimmed = Vec::new(env);
        for i in excess..history.len() {
            trimmed.push_back(history.get(i).unwrap());
        }
        trimmed
    } else {
        history
    }
}

/// Returns `true` when adding one more entry would exceed the per-outage
/// recalculation cap (`MAX_RECALCS_PER_OUTAGE`).
///
/// Used by the different-config branch of the dedup decision so the cap
/// is enforced in one readable call rather than an inline comparison.
pub fn exceeds_recalc_cap(stored_count: u32) -> bool {
    stored_count >= MAX_RECALCS_PER_OUTAGE
}