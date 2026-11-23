//! History storage: sharded per-entry layout with a per-outage index (v3).
//!
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

use soroban_sdk::{Address, Env, Symbol, Vec};

use crate::{
    HistoryPage, SLAError, SLAResult, EVENT_PRUNED, EVENT_PRUNED_AGE, EVENT_RET_LIM, HISTORY_LEN_KEY,
    HIST_ENTRY_KEY, HIST_HEAD_KEY, HIST_INDEX_KEY, HIST_TAIL_KEY, MAX_HISTORY_SIZE, RETENTION_LIMIT_KEY,
};

/// Upper bound on the number of entries a single pagination call may return.
/// Limits above this are clamped so no single call can read the full retained
/// history, enforcing the documented pagination policy server-side. Also
/// used to bound legacy full-history reads. (#409)
pub const MAX_PAGE_SIZE: u32 = 200;

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

/// Rebuilds the whole sharded history from `entries` (oldest-first), keeping
/// index keys consistent with the new set.
///
/// Used by the rare admin prune / trim / retention paths and by the v3
/// migration, where a full O(retained) pass is acceptable. Clears any existing
/// sharded state first, so it is idempotent. New entries are written at the
/// current head index onward, preserving the monotonic counters.
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

/// Checked history accessor shared with the contract entrypoint.
pub fn get_history(env: &Env) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(read_all_entries(env))
}

/// Checked history accessor shared with the contract entrypoint.
pub fn prune_history(env: &Env, caller: &Address, keep_latest: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let remove_count = prune_oldest(env, keep_latest);
    env.events().publish(
        (
            EVENT_PRUNED,
            crate::event_schema::event_version(EVENT_PRUNED),
            caller,
        ),
        (remove_count, keep_latest),
    );
    Ok(())
}

/// Checked history accessor shared with the contract entrypoint.
pub fn prune_history_by_age(env: &Env, caller: &Address, min_age_seconds: u64) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let now = env.ledger().timestamp();
    if min_age_seconds >= now {
        return Err(SLAError::InvalidInput);
    }
    let cutoff = now.saturating_sub(min_age_seconds);

    let history = read_all_entries(env);

    let mut new_history = Vec::new(env);
    let mut removed: u32 = 0;

    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        // Keep entries that are recent enough
        if entry.recorded_at >= cutoff {
            new_history.push_back(entry);
        } else {
            removed += 1;
        }
    }

    if removed > 0 {
        rebuild_history(env, &new_history);
    }
    let kept = new_history.len();
    env.events().publish(
        (
            EVENT_PRUNED_AGE,
            crate::event_schema::event_version(EVENT_PRUNED_AGE),
            caller,
        ),
        (removed, kept),
    );

    Ok(())
}

/// Checked history accessor shared with the contract entrypoint.
pub fn get_history_page(env: &Env, offset: u32, limit: u32) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let (head, tail) = (head(env), tail(env));
    let total = tail.saturating_sub(head);
    let limit = limit.min(MAX_PAGE_SIZE);
    if offset >= total || limit == 0 {
        return Ok(Vec::new(env));
    }
    // Shadow compute_page_slice: clamp the end index with saturating
    // arithmetic so extreme u32 inputs cannot wrap into a wrong slice.
    let end_rel = offset.saturating_add(limit).min(total);
    Ok(read_range(env, head + offset, end_rel - offset))
}

/// Checked history accessor shared with the contract entrypoint.
pub fn get_history_page_with_meta(env: &Env, offset: u32, limit: u32) -> Result<HistoryPage, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let (head, tail) = (head(env), tail(env));
    let total = tail.saturating_sub(head);
    let limit = limit.min(MAX_PAGE_SIZE);

    // Header comment documents offset/limit semantics; the number of
    // indices read is exactly the page width — no full-history traversal.
    let mut items = Vec::new(env);
    let end_rel = if offset < total && limit != 0 {
        let end_rel = offset.saturating_add(limit).min(total);
        items = read_range(env, head + offset, end_rel - offset);
        end_rel
    } else {
        offset.min(total)
    };

    // `has_more` is true when the requested range stops before the end of
    // history and limit > 0. When limit == 0, the empty page signals
    // end-of-history per docs/HISTORY_PAGINATION_POLICY.md.
    let has_more = if limit == 0 { false } else { end_rel < total };
    Ok(HistoryPage {
        items,
        total,
        has_more,
    })
}

/// Checked history accessor shared with the contract entrypoint.
pub fn get_history_by_outage(env: &Env, outage_id: Symbol) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    // Read only the outage's index subset instead of scanning the whole
    // retained history (#581): cost is proportional to the matching entries.
    Ok(entries_for_outage(env, &outage_id))
}

/// Checked history accessor shared with the contract entrypoint.
pub fn get_latest_by_outage(env: &Env, outage_id: Symbol) -> Result<Option<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    // The per-outage index stores the newest index last, so this reads a
    // single entry instead of scanning the whole retained history (#581).
    Ok(latest_for_outage(env, &outage_id))
}

/// Checked history accessor shared with the contract entrypoint.
pub fn set_retention_limit(env: &Env, caller: &Address, limit: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    crate::SLACalculatorContract::require_not_frozen(env)?;
    if limit == 0 || limit > MAX_HISTORY_SIZE {
        return Err(SLAError::RetentionLimitOutOfRange);
    }
    env.storage().instance().set(&RETENTION_LIMIT_KEY, &limit);
    env.events().publish(
        (
            EVENT_RET_LIM,
            crate::event_schema::event_version(EVENT_RET_LIM),
            caller.clone(),
        ),
        (limit,),
    );
    // Trim the oldest entries down to the new limit in place — survivors
    // keep their absolute indices, so only the dropped range is rewritten
    // (O(dropped) writes) rather than a full retained-set sweep (#582).
    let remove_count = prune_oldest(env, limit);
    if remove_count > 0 {
        env.events().publish(
            (
                EVENT_PRUNED,
                crate::event_schema::event_version(EVENT_PRUNED),
                caller,
            ),
            (remove_count, limit),
        );
    }
    Ok(())
}

/// Cached configuration count, maintained alongside the source config map.
pub fn get_config_count(env: &Env) -> Result<u32, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    env.storage()
        .instance()
        .get(&crate::CONFIG_COUNT_KEY)
        .ok_or(SLAError::NotInitialized)
}

/// Current retention limit, defaulting to the contract maximum.
pub fn get_retention_limit(env: &Env) -> Result<u32, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env
        .storage()
        .instance()
        .get(&RETENTION_LIMIT_KEY)
        .unwrap_or(MAX_HISTORY_SIZE))
}
