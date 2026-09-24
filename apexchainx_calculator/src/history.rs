//! SLA calculation history storage, pruning, and pagination.
//!
//! This module is the **single implementation** of every history concern the
//! contract exposes (`get_history`, pagination, per-outage lookup, retention
//! limit, pruning). [`crate::SLACalculatorContract`] delegates its history
//! methods here so there is no second copy that can drift (#563); fuzz and
//! parity targets therefore exercise exactly the code the contract runs.
//!
//! Supported operations: full retrieval, retention-limited pruning, age-based
//! pruning, paginated access, and per-outage lookup.

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

/// Returns the raw log of recent SLA calculations stored on-chain.
///
/// The full retained history is returned oldest-first. This is the deployed
/// contract behaviour. Consumers that wish to bound the number of entries read
/// should use the paginated [`get_history_page_with_meta`] accessor instead.
pub fn get_history(env: &Env) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env)))
}

/// Prunes history to retain only the most recent `keep_latest` entries.
/// Admin only. Emits a `pruned` event with `(remove_count, keep_latest)`.
pub fn prune_history(env: &Env, caller: &Address, keep_latest: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let len = history.len();

    let remove_count = if len > keep_latest {
        let remove_count = len - keep_latest;
        let mut new_history = Vec::new(env);

        for i in remove_count..len {
            new_history.push_back(history.get(i).unwrap());
        }

        // Issue #463: maintain cached history length alongside history
        env.storage().instance().set(&HISTORY_KEY, &new_history);
        env.storage().instance().set(&HISTORY_LEN_KEY, &new_history.len());
        remove_count
    } else {
        0
    };
    // Always emitted so a downstream indexer sees the (possibly no-op) prune.
    env.events().publish(
        (EVENT_PRUNED, EVENT_VERSION, caller.clone()),
        (remove_count, keep_latest),
    );
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
        // Issue #463: maintain cached history length alongside history
        env.storage().instance().set(&HISTORY_KEY, &new_history);
        env.storage().instance().set(&HISTORY_LEN_KEY, &new_history.len());
    }
    // Always emitted so a downstream indexer sees the (possibly no-op) prune.
    env.events().publish(
        (EVENT_PRUNED_AGE, EVENT_VERSION, caller.clone()),
        (removed, new_history.len()),
    );

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
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let (_end, page) = page_slice(env, &history, offset, limit);
    Ok(page)
}

/// Shared pagination slice computation (issue #264).
///
/// Returns the clamped end index and the slice items for a page, and
/// encapsulates the pagination policy defined in
/// `docs/HISTORY_PAGINATION_POLICY.md`:
///
/// - `limit` is clamped to [`MAX_PAGE_SIZE`].
/// - `offset >= len` (or `limit == 0`) yields an empty page; the returned end
///   index is clamped to the real history length so consumers can derive
///   `has_more` from it without re-deriving the policy.
/// - The end index uses saturating arithmetic so extreme `u32` inputs cannot
///   wrap into a wrong slice.
fn page_slice(env: &Env, history: &Vec<SLAResult>, offset: u32, limit: u32) -> (u32, Vec<SLAResult>) {
    let limit = limit.min(MAX_PAGE_SIZE);
    let len = history.len();
    let mut page = Vec::new(env);

    if offset < len && limit > 0 {
        // Saturating arithmetic: offset + limit could wrap for extreme u32 inputs.
        // Saturation clamps to the real history length, ensuring correct slicing.
        let end = offset.saturating_add(limit).min(len);
        for i in offset..end {
            page.push_back(history.get(i).unwrap());
        }
        (end, page)
    } else {
        (offset.min(len), page)
    }
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
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let total = history.len();
    // Both paginators derive their slice from the same `page_slice` helper, so
    // `items` can never diverge from `get_history_page` (issue #464). `end`
    // comes from the same helper so `has_more` and the page share one source.
    let (end, items) = page_slice(env, &history, offset, limit);
    // `has_more` is true when the requested range stops before the end of
    // history and limit > 0. When limit == 0, the empty page signals
    // end-of-history per docs/HISTORY_PAGINATION_POLICY.md.
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
///
/// The new limit is persisted immediately and the retained history is trimmed
/// to it in the same call (a `pruned` event is emitted when entries are
/// dropped). Rejected while the configuration is frozen.
pub fn set_retention_limit(env: &Env, caller: &Address, limit: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    crate::SLACalculatorContract::require_not_frozen(env)?;
    if limit == 0 || limit > MAX_HISTORY_SIZE {
        return Err(SLAError::RetentionLimitOutOfRange);
    }
    env.storage().instance().set(&RETENTION_LIMIT_KEY, &limit);
    env.events()
        .publish((EVENT_RET_LIM, EVENT_VERSION, caller.clone()), (limit,));
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let len = history.len();
    if len > limit {
        let remove_count = len - limit;
        let mut new_history = Vec::new(env);
        for i in remove_count..len {
            new_history.push_back(history.get(i).unwrap());
        }
        env.storage().instance().set(&HISTORY_KEY, &new_history);
        env.storage().instance().set(&HISTORY_LEN_KEY, &new_history.len());
        env.events()
            .publish((EVENT_PRUNED, EVENT_VERSION, caller), (remove_count, limit));
    }
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
