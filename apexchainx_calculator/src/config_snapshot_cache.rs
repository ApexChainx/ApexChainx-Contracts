//! Memoized config snapshot cache keyed by config version hash (Issue #660).
//!
//! `get_config_snapshot` previously rebuilt and re-sorted the full config map
//! on every call. Since backends are advised to "bootstrap once, diff by hash",
//! this causes redundant CPU and storage work on every poll when the config
//! hasn't changed.
//!
//! # Design
//!
//! - The snapshot is cached under `SNAPSHOT_CACHE_KEY` alongside its hash.
//! - On read: if the stored hash matches the current `config_version_hash`,
//!   return the cached snapshot directly.
//! - On write: `invalidate_snapshot_cache()` removes the cached entry so the
//!   next read recomputes and re-caches a fresh snapshot.
//!
//! # Invariants
//!
//! - The cached snapshot is always consistent with the hash stored beside it.
//! - Any config write MUST call `invalidate_snapshot_cache()` before returning.
//! - The cache is a pure performance optimisation; removing it changes only
//!   the cost curve, not correctness.

use soroban_sdk::{contracttype, symbol_short, Env, Symbol};

use crate::SLAConfigSnapshot;

/// On-chain storage key for the cached `(hash, snapshot)` pair.
pub(crate) const SNAPSHOT_CACHE_KEY: Symbol = symbol_short!("SNPCCH");

/// Cached snapshot entry — stores the hash alongside the snapshot so a
/// single storage read determines whether the cache is still valid.
#[contracttype]
#[derive(Clone)]
pub struct SnapshotCacheEntry {
    pub config_version_hash: u64,
    pub snapshot: SLAConfigSnapshot,
}

/// Return the cached snapshot if the config hash is unchanged,
/// otherwise `None` (caller must recompute and call `store_snapshot_cache`).
pub fn read_snapshot_cache(env: &Env, current_hash: u64) -> Option<SLAConfigSnapshot> {
    let entry: SnapshotCacheEntry = env
        .storage()
        .instance()
        .get(&SNAPSHOT_CACHE_KEY)?;
    if entry.config_version_hash == current_hash {
        Some(entry.snapshot)
    } else {
        None
    }
}

/// Persist a freshly built snapshot alongside its hash.
/// Called after every recompute so subsequent reads are cache-hits.
pub fn store_snapshot_cache(env: &Env, hash: u64, snapshot: SLAConfigSnapshot) {
    env.storage().instance().set(
        &SNAPSHOT_CACHE_KEY,
        &SnapshotCacheEntry {
            config_version_hash: hash,
            snapshot,
        },
    );
}

/// Invalidate the snapshot cache.
/// Must be called by every config-write path so the next read rebuilds.
pub fn invalidate_snapshot_cache(env: &Env) {
    env.storage().instance().remove(&SNAPSHOT_CACHE_KEY);
}