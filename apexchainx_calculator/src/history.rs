//! SLA calculation history storage, pruning, and pagination.
//!
//! This module manages the on-chain history of SLA calculation results,
//! supporting full retrieval, retention-limited pruning, age-based pruning,
//! paginated access, and per-outage lookup.

use soroban_sdk::{Address, Env, Symbol, Vec};

use crate::{
    HistoryPage, SLAError, SLAResult, EVENT_PRUNED, EVENT_PRUNED_AGE, EVENT_RET_LIM, EVENT_VERSION,
    HISTORY_KEY, HISTORY_LEN_KEY, MAX_HISTORY_SIZE, RETENTION_LIMIT_KEY,
};

/// Upper bound on the number of entries a single pagination call may return.
/// Limits above this are clamped so no single call can read the full retained
/// history, enforcing the documented pagination policy server-side. Also
/// used to bound legacy full-history reads. (#409)
pub const MAX_PAGE_SIZE: u32 = 200;

/// Returns a bounded slice of the SLA history (the most recent entries).
///
/// LEGACY / EXPENSIVE: this returns at most [`MAX_PAGE_SIZE`] entries and is
/// retained for backwards compatibility. New consumers should prefer the
/// paginated [`get_history_page_with_meta`] accessor to bound reads explicitly.
pub fn get_history(env: &Env) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let len = history.len();
    let start = len.saturating_sub(MAX_PAGE_SIZE);
    let mut bounded = Vec::new(env);
    for i in start..len {
        bounded.push_back(history.get(i).unwrap());
    }
    Ok(bounded)
}

/// Prunes history to retain only the most recent `keep_latest` entries.
/// Admin only. Emits a `pruned` event.
pub fn prune_history(env: &Env, caller: &Address, keep_latest: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let len = history.len();

    if len > keep_latest {
        let remove_count = len - keep_latest;
        let mut new_history = Vec::new(env);

        for i in remove_count..len {
            new_history.push_back(history.get(i).unwrap());
        }

        // Issue #463: maintain cached history length alongside history
        env.storage().instance().set(&HISTORY_KEY, &new_history);
        env.storage().instance().set(&HISTORY_LEN_KEY, &new_history.len());
        let kept = new_history.len();
        env.events().publish(
            (EVENT_PRUNED, EVENT_VERSION, caller.clone()),
            (remove_count, kept),
        );
    }

    Ok(())
}

/// Prunes history entries older than `min_age_seconds`.
/// Admin only. Emits a `pruned_a` event.
///
/// Returns `Err(SLAError::InvalidInput)` if `min_age_seconds >= now`.
pub fn prune_history_by_age(env: &Env, caller: &Address, min_age_seconds: u64) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let now = env.ledger().timestamp();
    if min_age_seconds >= now {
        return Err(SLAError::InvalidInput);
    }
    let cutoff = now.saturating_sub(min_age_seconds);

    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));

    let mut new_history = Vec::new(env);
    let mut removed: u32 = 0;

    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.recorded_at >= cutoff {
            new_history.push_back(entry);
        } else {
            removed += 1;
        }
    }

    if removed > 0 {
        let kept = new_history.len();
        // Issue #463: maintain cached history length alongside history
        env.storage().instance().set(&HISTORY_KEY, &new_history);
        env.storage().instance().set(&HISTORY_LEN_KEY, &new_history.len());
        env.events()
            .publish((EVENT_PRUNED_AGE, EVENT_VERSION, caller.clone()), (removed, kept));
    }

    Ok(())
}

/// Returns a paginated slice of the SLA history.
///
/// # Pagination policy (issue #263)
///
/// The accessor is **offset-based** and deterministic:
///
/// - `offset` is the 0-based index of the first entry to return. History is
///   stored oldest-first, so `offset = 0` is the earliest recorded result.
/// - `limit` is the maximum number of entries returned per page. It is clamped
///   to an upper bound (`MAX_PAGE_SIZE`): the effective page is
///   `min(min(limit, MAX_PAGE_SIZE), len - offset)`, so a page shorter than the
///   requested `limit` signals end-of-history. A `limit` larger than the
///   remaining history simply returns everything that remains.
/// - An out-of-range `offset` (`offset >= len`) returns an **empty page**, not
///   an error — empty pages are the canonical end-of-history signal, so
///   consumers can loop until they see one without special-casing.
/// - `limit == 0` returns an empty page.
/// - Offsets and limits are `u32`. The interior computation `offset + limit` is
///   performed with saturating arithmetic so that extreme values (e.g.
///   `u32::MAX`) can never overflow/wrap into a wrong slice — the end index is
///   always `min(offset + limit, len)` clamped to the real history length.
///
/// See `docs/HISTORY_PAGINATION_POLICY.md` for the full policy.
pub fn get_history_page(env: &Env, offset: u32, limit: u32) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let limit = limit.min(MAX_PAGE_SIZE);
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let len = history.len();
    let mut page = Vec::new(env);
    if offset >= len || limit == 0 {
        return Ok(page);
    }
    // Saturating arithmetic: `offset + limit` could otherwise wrap for extreme
    // `u32` inputs (e.g. offset near `u32::MAX`), silently slicing the wrong
    // range. Saturation clamps the end index to the real history length, which
    // is the correct behaviour for any page that asks for more than remains.
    let end = offset.saturating_add(limit).min(len);
    for i in offset..end {
        page.push_back(history.get(i).unwrap());
    }
    Ok(page)
}

/// Returns a paginated slice of the SLA history with pagination metadata.
///
/// This is a metadata-carrying companion to [`get_history_page`]. The `items`
/// slice is identical to what `get_history_page` returns for the same
/// `(offset, limit)`; `total` is the full history length and `has_more` is
/// `true` when the requested range ends before the end of history **and**
/// `limit > 0`. When `limit == 0`, `has_more` is `false` (empty page signals
/// end-of-history).
///
/// Pagination semantics (offset-based, oldest-first, saturating
/// `offset + limit`, empty page when `offset >= len` or `limit == 0`) are
/// identical to [`get_history_page`] — see
/// `docs/HISTORY_PAGINATION_POLICY.md`.
pub fn get_history_page_with_meta(env: &Env, offset: u32, limit: u32) -> Result<HistoryPage, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let limit = limit.min(MAX_PAGE_SIZE);
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let total = history.len();
    let mut items = Vec::new(env);
    // Saturating arithmetic mirrors `get_history_page`: clamp the end index to
    // the real history length so extreme `u32` inputs can never wrap into a
    // wrong slice. `end` also drives `has_more`: entries remain whenever the
    // requested range stops short of the end of history and limit > 0.
    let end = offset.saturating_add(limit).min(total);
    if offset < total && limit != 0 {
        for i in offset..end {
            items.push_back(history.get(i).unwrap());
        }
    }
    // When limit == 0, the page is empty by request, which signals end-of-history
    // per the pagination policy. This ensures consistency with get_history_page.
    let has_more = if limit == 0 { false } else { end < total };
    Ok(HistoryPage {
        items,
        total,
        has_more,
    })
}

/// Returns all history entries for a specific outage ID in chronological order (oldest-first).
///
/// When an outage has multiple entries across config generations (up to
/// `MAX_RECALCS_PER_OUTAGE`), each entry carries its `config_version_hash`
/// so consumers can match records to specific config generations. The final
/// entry in the returned array represents the latest decision.
pub fn get_history_by_outage(env: &Env, outage_id: Symbol) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let mut matches = Vec::new(env);
    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.outage_id == outage_id {
            matches.push_back(entry);
        }
    }
    Ok(matches)
}

/// Returns the most recent history entry for a given outage ID, if any.
pub fn get_latest_by_outage(env: &Env, outage_id: Symbol) -> Result<Option<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let mut latest: Option<SLAResult> = None;
    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.outage_id == outage_id {
            latest = Some(entry);
        }
    }
    Ok(latest)
}

/// Returns the number of configured severity levels.
pub fn get_config_count(env: &Env) -> Result<u32, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let configs: soroban_sdk::Map<Symbol, crate::SLAConfig> = env
        .storage()
        .instance()
        .get(&crate::CONFIG_KEY)
        .ok_or(SLAError::NotInitialized)?;
    Ok(configs.len())
}

/// Sets the retention limit for history entries. Admin only.
pub fn set_retention_limit(env: &Env, caller: &Address, limit: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    if limit == 0 || limit > MAX_HISTORY_SIZE {
        return Err(SLAError::RetentionLimitOutOfRange);
    }
    env.storage().instance().set(&RETENTION_LIMIT_KEY, &limit);
    env.events()
        .publish((EVENT_RET_LIM, EVENT_VERSION, caller.clone()), (limit,));
    Ok(())
}

/// Returns the current retention limit (defaults to MAX_HISTORY_SIZE).
pub fn get_retention_limit(env: &Env) -> Result<u32, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env
        .storage()
        .instance()
        .get(&RETENTION_LIMIT_KEY)
        .unwrap_or(MAX_HISTORY_SIZE))
}
