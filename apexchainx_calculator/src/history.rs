//! History storage: sharded per-entry layout with a per-outage index (v3).
//!
//! This module is the **single implementation** of every history concern the
//! contract exposes (`get_history`, pagination, per-outage lookup, retention
//! limit, pruning). [`crate::SLACalculatorContract`] delegates its history
//! methods here so there is no second copy that can drift (#563); fuzz and
//! parity targets therefore exercise exactly the code the contract runs.
//!
//! Supported operations: full retrieval, retention-limited pruning, age-based
//! pruning, paginated access, and per-outage lookup.
//! Since v3 (#580/#581/#582) the SLA history is no longer a single `Vec`
//! stored under one instance-storage key (which forced a full-vector rewrite
//! on every append and a full scan for every outage lookup). Instead:
//!
//! - each entry is stored under its own key `(HIST_ENTRY_KEY, u32 index)`,
//! - `(HIST_INDEX_KEY, Symbol outage_id)` holds a `Vec<u32>` of that outage's
//!   entry indices (oldest-first), and
//! - `HIST_HEAD_KEY` / `HIST_TAIL_KEY` are monotonic counters delimiting the
//!   retained range `[head, tail)`, with `HISTORY_LEN_KEY` caching the count.
//!
//! Write amplification on the common append path is bounded: one entry
//! sub-key, one small per-outage index entry, and two counters. Reads are
//! proportional to the subset requested: `get_latest_by_outage` reads a single
//! entry, `get_history_by_outage` reads only the matching indices, and
//! pagination reads only the requested page.
//!
//! Full rebuilds (`rebuild_history`) are reserved for the rare admin prune /
//! trim / retention paths and for migration, where an O(retained) pass is
//! acceptable and deliberately documented.
//!
//! This module is a storage layer only — it performs no version or auth
//! checks. Callers run `check_version` / `require_admin` before reaching it.

use soroban_sdk::{Env, Symbol, Vec};

use crate::{SLAResult, HISTORY_LEN_KEY, HIST_ENTRY_KEY, HIST_HEAD_KEY, HIST_INDEX_KEY, HIST_TAIL_KEY};

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
/// Index of the oldest retained entry, or `0` when empty.
pub fn head(env: &Env) -> u32 {
    env.storage().instance().get(&HIST_HEAD_KEY).unwrap_or(0)
}

/// Index one past the newest retained entry (the next write index).
pub fn tail(env: &Env) -> u32 {
    env.storage().instance().get(&HIST_TAIL_KEY).unwrap_or(0)
}

/// Number of retained entries.
pub fn len(env: &Env) -> u32 {
    tail(env).saturating_sub(head(env))
}

    let remove_count = if len > keep_latest {
        let remove_count = len - keep_latest;
        let mut new_history = Vec::new(env);
/// Reads a single entry by its absolute index.
pub fn read_entry(env: &Env, index: u32) -> Option<SLAResult> {
    env.storage().instance().get(&(HIST_ENTRY_KEY, index))
}

/// Reads `count` contiguous entries starting at absolute index `start`.
/// Missing slots are skipped, so the result is the retained entries present.
pub fn read_range(env: &Env, start: u32, count: u32) -> Vec<SLAResult> {
    let mut out = Vec::new(env);
    for i in start..start.saturating_add(count) {
        if let Some(entry) = read_entry(env, i) {
            out.push_back(entry);
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
    out
}

/// All retained entries, oldest-first.
pub fn read_all_entries(env: &Env) -> Vec<SLAResult> {
    read_range(env, head(env), len(env))
}

/// The entry indices of all retained entries for `outage_id`, oldest-first.
pub fn entry_indices_for_outage(env: &Env, outage_id: &Symbol) -> Vec<u32> {
    env.storage()
        .instance()
        .get(&(HIST_INDEX_KEY, outage_id.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

/// All retained entries for `outage_id`, oldest-first. Reads exactly the
/// outage's index subset — no full-history scan (#581).
pub fn entries_for_outage(env: &Env, outage_id: &Symbol) -> Vec<SLAResult> {
    let indices = entry_indices_for_outage(env, outage_id);
    let mut out = Vec::new(env);
    for i in 0..indices.len() {
        let index = indices.get(i).unwrap();
        if let Some(entry) = read_entry(env, index) {
            out.push_back(entry);
        }
    }
    out
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
/// The most recent retained entry for `outage_id`, if any. Reads the last
/// entry of the outage's index instead of scanning the whole history (#581).
pub fn latest_for_outage(env: &Env, outage_id: &Symbol) -> Option<SLAResult> {
    let indices = entry_indices_for_outage(env, outage_id);
    match indices.last() {
        Some(index) => read_entry(env, index),
        None => None,
    }
}

/// Appends `entry` to history.
///
/// Writes one entry sub-key, one small per-outage index entry, and the
/// head/tail counters and cached length — the shared history vector is never
/// rewritten, bounding write amplification (#582). When the retained count
/// exceeds `retention_limit` the oldest entry and its index entry are dropped,
/// again without touching any other retained entry.
///
/// `current_indices` is the caller's already-read per-outage index vector for
/// `entry.outage_id` (empty for a first submission); passing it avoids a second
/// instance-storage read of the same index on the hot path.
pub fn append_entry(env: &Env, entry: &SLAResult, retention_limit: u32, current_indices: &Vec<u32>) {
    let mut h = head(env);
    let mut t = tail(env);

    env.storage().instance().set(&(HIST_ENTRY_KEY, t), entry);

    let mut indices = current_indices.clone();
    indices.push_back(t);
    env.storage()
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
///
/// Issue #563: this is the single implementation of the slicing policy —
/// `SLACalculatorContract`'s contract methods delegate here (via
/// `history::get_history_page`/`get_history_page_with_meta`) instead of
/// duplicating it, so the two can no longer drift apart.
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
        .set(&(HIST_INDEX_KEY, entry.outage_id.clone()), &indices);

    t += 1;
    env.storage().instance().set(&HIST_TAIL_KEY, &t);

    if t.saturating_sub(h) > retention_limit {
        // Drop the oldest entry: read before removing its sub-key, then drop
        // its index from the owning outage's index list.
        let dropped = read_entry(env, h);
        env.storage().instance().remove(&(HIST_ENTRY_KEY, h));
        if let Some(d) = dropped {
            let dx = entry_indices_for_outage(env, &d.outage_id);
            let mut kept = Vec::new(env);
            for i in 0..dx.len() {
                let idx = dx.get(i).unwrap();
                if idx != h {
                    kept.push_back(idx);
                }
            }
            env.storage()
                .instance()
                .set(&(HIST_INDEX_KEY, d.outage_id.clone()), &kept);
        }
        h += 1;
        env.storage().instance().set(&HIST_HEAD_KEY, &h);
    }

    env.storage()
        .instance()
        .set(&HISTORY_LEN_KEY, &t.saturating_sub(h));
}

/// Drops the oldest entries until `keep_count` remain, returning how many
/// were removed.
///
/// This is the bulk form of the append-time drop-oldest and is used by the
/// admin prune / retention-trim paths. Survivors keep their absolute indices,
/// so only the dropped range is touched: the dropped entry sub-keys are
/// removed and the owning outage's index lists are rewritten without the
/// dropped indices. Memory of the removed oldest entries is reclaimed
/// immediately; per-call cost is O(dropped) writes rather than O(retained).
pub fn prune_oldest(env: &Env, keep_count: u32) -> u32 {
    let h = head(env);
    let t = tail(env);
    let len = t.saturating_sub(h);
    if keep_count >= len {
        return 0;
    }
    let kept_from = t.saturating_sub(keep_count); // survivors are [kept_from, t)

    // Touch only the dropped range [h, kept_from).
    let mut touched_outages = Vec::new(env);
    for i in h..kept_from {
        let dropped = read_entry(env, i);
        env.storage().instance().remove(&(HIST_ENTRY_KEY, i));
        if let Some(d) = dropped {
            let mut already = false;
            for j in 0..touched_outages.len() {
                if touched_outages.get(j).unwrap() == d.outage_id {
                    already = true;
                    break;
                }
            }
            if !already {
                touched_outages.push_back(d.outage_id.clone());
            }
        }
    }

    // Rewrite only the affected outage indexes, dropping indices < kept_from.
    for i in 0..touched_outages.len() {
        let outage = touched_outages.get(i).unwrap();
        let idx = entry_indices_for_outage(env, &outage);
        let mut kept = Vec::new(env);
        for j in 0..idx.len() {
            let v = idx.get(j).unwrap();
            if v >= kept_from {
                kept.push_back(v);
            }
        }
        env.storage()
            .instance()
            .set(&(HIST_INDEX_KEY, outage.clone()), &kept);
    }

    env.storage().instance().set(&HIST_HEAD_KEY, &kept_from);
    env.storage().instance().set(&HISTORY_LEN_KEY, &keep_count);
    kept_from.saturating_sub(h)
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
/// Rebuilds the whole sharded history from `entries` (oldest-first), keeping
/// index keys consistent with the new set.
///
pub fn rebuild_history(env: &Env, entries: &Vec<SLAResult>) {
    let h = head(env);
    let t = tail(env);

    // Clear current per-entry sub-keys and remember which outages held data so
    // their index keys can be removed (kept consistent even across repeats).
    let mut seen_outages = Vec::new(env);
    for i in h..t {
        if let Some(entry) = read_entry(env, i) {
            env.storage().instance().remove(&(HIST_ENTRY_KEY, i));
            let mut already = false;
            for j in 0..seen_outages.len() {
                if seen_outages.get(j).unwrap() == entry.outage_id {
                    already = true;
                    break;
                }
            }
            if !already {
                seen_outages.push_back(entry.outage_id.clone());
            }
        }
    }
    for i in 0..seen_outages.len() {
        env.storage()
            .instance()
            .remove(&(HIST_INDEX_KEY, seen_outages.get(i).unwrap()));
    }

    // Write the new entries and per-outage index in one pass.
    let mut outage_indexes = soroban_sdk::Map::<Symbol, Vec<u32>>::new(env);
    let mut cur = h;
    for i in 0..entries.len() {
        let entry = entries.get(i).unwrap();
        env.storage().instance().set(&(HIST_ENTRY_KEY, cur), &entry);
        let mut idx = outage_indexes
            .get(entry.outage_id.clone())
            .unwrap_or_else(|| Vec::new(env));
        idx.push_back(cur);
        outage_indexes.set(entry.outage_id.clone(), idx);
        cur += 1;
    }
    for (outage, indices) in outage_indexes {
        env.storage().instance().set(&(HIST_INDEX_KEY, outage), &indices);
    }

    let new_tail = h.saturating_add(entries.len());
    env.storage().instance().set(&HIST_HEAD_KEY, &h);
    env.storage().instance().set(&HIST_TAIL_KEY, &new_tail);
    env.storage().instance().set(&HISTORY_LEN_KEY, &entries.len());
}

/// v3 migration entry point: converts a legacy single-vector history into the
/// sharded layout (per-entry sub-keys plus per-outage index). The legacy
/// `HISTORY_KEY` must be removed by the caller. Safe to run on an already
/// sharded deployment — `rebuild_history` clears existing state first, so this
/// migration is idempotent (#580/#581/#582).
pub fn migrate_to_sharded(env: &Env, legacy: &Vec<SLAResult>) {
    rebuild_history(env, legacy);
}
