//! Unified internal history trim helper (Issue #655).
//!
//! Pruning logic previously existed in three places with subtly different
//! guarantees: `calculate_sla` (one-at-a-time append trim), `prune_history`
//! (admin trim-to-limit), and `prune_history_by_age` (age-based removal).
//!
//! This module provides `trim_history` — a single internal function that all
//! three call sites route through, ensuring consistent event emission,
//! HISTORY_LEN_KEY maintenance, and removal semantics.

use soroban_sdk::{symbol_short, Address, Env, Symbol, Vec};
use crate::{SLAResult, HISTORY_KEY, HISTORY_LEN_KEY, EVENT_PRUNED, EVENT_PRUNED_AGE, EVENT_VERSION};

/// The reason a trim was triggered — drives event selection.
pub enum TrimReason {
    /// Explicit admin call to `prune_history(keep_latest)`.
    AdminCount,
    /// Explicit admin call to `prune_history_by_age`.
    AdminAge,
    /// Automatic capacity enforcement during `calculate_sla` append.
    AutoCapacity,
}

/// Remove `entries` from the front of `history`, update `HISTORY_LEN_KEY`,
/// and emit the appropriate event. Returns the kept count.
///
/// This is the single implementation of "trim history" — all three call
/// sites must route through here so event emission and cache maintenance
/// are always consistent.
pub fn trim_history(
    env: &Env,
    new_history: Vec<SLAResult>,
    removed_count: u32,
    caller: &Address,
    reason: TrimReason,
) {
    if removed_count == 0 {
        return;
    }
    let kept = new_history.len();
    env.storage().instance().set(&HISTORY_KEY, &new_history);
    env.storage().instance().set(&HISTORY_LEN_KEY, &kept);

    match reason {
        TrimReason::AdminAge => {
            env.events().publish(
                (EVENT_PRUNED_AGE, EVENT_VERSION, caller.clone()),
                (removed_count, kept),
            );
        }
        TrimReason::AdminCount | TrimReason::AutoCapacity => {
            env.events().publish(
                (EVENT_PRUNED, EVENT_VERSION, caller.clone()),
                (removed_count, kept),
            );
        }
    }
}