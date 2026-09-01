//! SC-W5-041 – Canonical event schema for SLA calculation outputs.
//!
//! This module defines the canonical schema for all contract events consumed
//! by backend indexers. Every event follows the same structural contract:
//!
//! Topic layout (3 topics):
//!   topic[0] = event name (Symbol constant)
//!   topic[1] = event version ("v1")
//!   topic[2] = event-specific context (severity, caller address, etc.)
//!
//! Payload field ordering and types are documented below per event variant.
//! These schemas MUST NOT be changed without a corresponding version bump.
//!
//! # Event Catalog
//!
//! The three decision-carrying events (`sla_calc`, `set_int`, `dup_input`)
//! share a single canonical payload field order — the `SLAResult` struct
//! order — so indexers parse one layout regardless of which decision event
//! they consume. Any divergence between them is a bug (#429). The canonical
//! order is:
//!
//!   (outage_id, status, mttr_minutes, threshold_minutes, amount,
//!    payment_type, rating, config_version_hash, recorded_at)
//!
//! ## sla_calc (`sla_calc`)
//! Emitted on every successful `calculate_sla` call.
//! - topic[2]: severity Symbol
//! - payload:  (outage_id: Symbol, status: Symbol, mttr_minutes: u32,
//!   threshold_minutes: u32, amount: i128, payment_type: Symbol,
//!   rating: Symbol, config_version_hash: u64, recorded_at: u64)
//!
//! ## set_int (`set_int`)
//! Settlement intent emitted alongside sla_calc for backend reconciliation.
//! Carries the full decision (including `mttr_minutes`, `threshold_minutes`,
//! and `rating`) so a settlement-only consumer can reconstruct the SLA
//! decision without a follow-up read (#428).
//! - topic[2]: severity Symbol
//! - payload:  (outage_id: Symbol, status: Symbol, mttr_minutes: u32,
//!   threshold_minutes: u32, amount: i128, payment_type: Symbol,
//!   rating: Symbol, config_version_hash: u64, recorded_at: u64)
//!
//! ## dup_input (`dup_input`)
//! Emitted when `calculate_sla` rejects a conflicting duplicate `outage_id`
//! (the `DuplicateOutageInput` error path). Carries the previously stored
//! `SLAResult` so consumers can reconcile the rejection without a separate
//! `get_latest_by_outage` read. (#385)
//! - topic[2]: severity Symbol
//! - payload:  (outage_id: Symbol, status: Symbol, mttr_minutes: u32,
//!   threshold_minutes: u32, amount: i128, payment_type: Symbol,
//!   rating: Symbol, config_version_hash: u64, recorded_at: u64)
//!
//! ## cfg_upd (`cfg_upd`)
//! Emitted on every successful `set_config` call.
//! - topic[2]: severity Symbol
//! - payload:  (threshold_minutes: u32, penalty_per_minute: i128,
//!   reward_base: i128)
//! - repeated writes preserve invocation order; see the regression policy in
//!   `docs/PROJECT_CONTEXT.md`
//!
//! ## sev_add (`sev_add`)
//! Emitted when `set_custom_severity` registers a **new** custom severity.
//! - topic[2]: custom severity Symbol
//! - payload:  (threshold_minutes: u32, penalty_per_minute: i128,
//!   reward_base: i128)
//!
//! ## sev_upd (`sev_upd`)
//! Emitted when `set_custom_severity` **reconfigures** an existing one.
//! - topic[2]: custom severity Symbol
//! - payload:  (threshold_minutes: u32, penalty_per_minute: i128,
//!   reward_base: i128)
//!
//! ## cfg_rem (`cfg_rem`)
//! Emitted when `remove_custom_severity` deletes a custom severity.
//! - topic[2]: custom severity Symbol
//! - payload:  ()
//!
//! ## paused (`paused`)
//! Emitted when the contract is paused.
//! - topic[2]: caller Address
//! - payload:  (true,)
//!
//! ## unpause (`unpause`)
//! Emitted when the contract is unpaused.
//! - topic[2]: caller Address
//! - payload:  (false,)
//!
//! ## op_set (`op_set`)
//! Emitted when the operator is set directly by the admin (single-step legacy path).
//! Unlike the two-step path (`op_prop` → `op_acc`), this event indicates the new
//! operator did **not** consent to the role change — `set_operator` only requires
//! the admin's authorization. Consumers that need to distinguish consented from
//! non-consented operator changes should check for this event name vs. the
//! `op_prop`+`op_acc` pair.
//! - topic[2]: caller Address (admin who performed the set)
//! - payload:  (new_operator: Address,)
//!
//! ## pruned (`pruned`)
//! Emitted after a prune_history call removes entries.
//! - topic[2]: caller Address
//! - payload:  (removed_count: u32, kept_count: u32)
//!
//! ## pruned_a (`pruned_a`)
//! Emitted after a prune_history_by_age call removes entries.
//! - topic[2]: caller Address
//! - payload:  (removed_count: u32, kept_count: u32)
//!
//! ## adm_prop (`adm_prop`)
//! Emitted when a new admin is proposed.
//! - topic[2]: caller Address
//! - payload:  (new_admin: Address,)
//!
//! ## adm_acc (`adm_acc`)
//! Emitted when a pending admin proposal is accepted.
//! - topic[2]: caller Address
//! - payload:  ()
//!
//! ## adm_can (`adm_can`)
//! Emitted when a pending admin proposal is cancelled.
//! - topic[2]: caller Address
//! - payload:  ()
//!
//! ## adm_ren (`adm_ren`)
//! Emitted when the admin renounces their role.
//! - topic[2]: caller Address
//! - payload:  ()
//!
//! ## adm_sup (`adm_sup`)
//! Emitted when a pending admin proposal is superseded by a re-proposal
//! before the prior candidate accepted or cancelled. (#468)
//! - topic[2]: caller Address
//! - payload:  (superseded_admin: Address, new_admin: Address)
//!
//! ## op_prop (`op_prop`)
//! Emitted when a new operator is proposed.
//! - topic[2]: caller Address
//! - payload:  (new_operator: Address,)
//!
//! ## op_acc (`op_acc`)
//! Emitted when a pending operator proposal is accepted.
//! - topic[2]: caller Address
//! - payload:  ()
//!
//! ## op_can (`op_can`)
//! Emitted when a pending operator proposal is cancelled.
//! - topic[2]: caller Address
//! - payload:  ()
//!
//! ## op_sup (`op_sup`)
//! Emitted when a pending operator proposal is superseded by a re-proposal
//! before the prior candidate accepted or cancelled. (#468)
//! - topic[2]: caller Address
//! - payload:  (superseded_operator: Address, new_operator: Address)
//!
//! ## cfg_frz (`cfg_frz`)
//! Emitted when the configuration is frozen by admin.
//! - topic[2]: caller Address
//! - payload:  ()
//!
//! ## cfg_unfrz (`cfg_unfrz`)
//! Emitted when the configuration is unfrozen by admin.
//! - topic[2]: caller Address
//! - payload:  ()
//!
//! ## stats_sat (`stats_sat`)
//! Emitted when a running-stats counter saturates during increment_stats
//! (e.g. total_calculations reaching u64::MAX, or an i128 total capping).
//! Signals backend indexers that the on-chain total has capped and now
//! under-reports true economic exposure. The counter is still capped at its
//! bound on-chain; this event carries the pre-cap state so consumers can
//! reconcile. (SC-W5-047)
//! - topic[2]: counter_name Symbol (which counter saturated: `totcalc`,
//!   `totviol`, `totrew`, `totpen`)
//! - payload:  (field: Symbol, previous_value: i128, attempted_increment: i128)
//!
//! ## migrate_done (`migrate_done`)
//! Emitted when a storage migration completes successfully.
//! - topic[2]: caller Address
//! - payload:  (old_version: u32, new_version: u32)
//!
//! # Schema Versioning
//!
//! Breaking changes (field removal, type changes, reordering) MUST increment
//! the version symbol from "v1" to "v2". Additive changes (new fields at the
//! end) are NOT considered breaking and do not require a version bump as long
//! as old consumers ignore unrecognised trailing fields.
//!
//! # Symbol Deprecation Protocol
//!
//! When a Result or Severity symbol needs to change, follow this deprecation
//! lifecycle to avoid breaking backend consumers:
//!
//! 1. **Introduction (minor release)**: Add the new symbol alongside the old one.
//!    Both symbols are emitted in events. `get_result_schema()` returns both
//!    with a `deprecated_symbols` entry marking the old symbol as deprecated.
//!
//! 2. **Coexistence (at least one minor release)**: The old symbol continues to
//!    be emitted alongside the new one. Backends can migrate at their own pace.
//!    The `deprecated_symbols` entry includes `removed_at` = None (TBD).
//!
//! 3. **Removal (major release)**: The old symbol is removed from event emission.
//!    The `schema_version` in `get_result_schema()` is bumped. The deprecated
//!    entry remains in `deprecated_symbols` with `removed_at` set to the schema
//!    version at which removal occurred.
//!
//! ## Example
//!
//! If we replace `"viol"` with `"violated"` as the human-readable status:
//!
//! - **v1**: `"viol"` is the only status symbol for violated SLAs.
//! - **v2**: `"violated"` is introduced. Events emit both `"viol"` and
//!   `"violated"`. `get_result_schema()` returns:
//!   ```json
//!   {
//!     "status_met": "met",
//!     "status_violated": "violated",
//!     "deprecated_symbols": [
//!       { "old_symbol": "viol", "new_symbol": "violated", "deprecated_at": 2, "removed_at": null }
//!     ]
//!   }
//!   ```
//! - **v3**: `"viol"` is removed. Events emit only `"violated"`.
//!   `deprecated_symbols` is updated with `removed_at: 3`.
//!
//! Backends MUST check `deprecated_symbols` at startup and log warnings for
//! any deprecated symbols they still rely on.
//!
//! # Review Checklist
//!
//! When adding or modifying events, refer to both the SC-099 Event-Topic Schema
//! Checklist and the SC-100 Public Method Review Checklist in `CONTRIBUTING.md`.

use soroban_sdk::{symbol_short, Symbol};

/// Canonical event version symbol used by all events.
pub const EVENT_VERSION: Symbol = symbol_short!("v1");

/// 1-based event-ABI generation number for the current `EVENT_VERSION`.
///
/// This is the mechanical index that ties the event symbol to the rest of the
/// contract's version posture. Generation 1 corresponds to `"v1"`; a breaking
/// event change MUST bump `EVENT_VERSION` to `"v2"` **and** this constant to
/// `2` in the same commit (the two can never diverge). Removals/removals of
/// event fields are tracked by generation, not by the symbol string, so the
/// co-bump rules in `contract_info.rs` and `docs/UPGRADE_PLAYBOOK.md` can be
/// enforced numerically instead of by parsing symbols. (#497)
pub const EVENT_ABI_GENERATION: u32 = 1;

/// Required minimum `STORAGE_VERSION` / `RESULT_SCHEMA_VERSION` for each event
/// ABI generation. Indexed by `EVENT_ABI_GENERATION - 1`.
///
/// A release that bumps the event ABI to generation `g` MUST also ship a
/// storage schema and result schema at least the value in this table for index
/// `g - 1`. In other words, a breaking event change can never ride along on an
/// unchanged `STORAGE_VERSION`/`RESULT_SCHEMA_VERSION`:
///
/// | `EVENT_VERSION` | generation | required schema version |
/// |-----------------|------------|-------------------------|
/// | `"v1"`          | `1`        | `1`                     |
/// | `"v2"`          | `2`        | `2`                     |
///
/// The enforcement test `test_event_abi_cobump_invariant` in `contract_info.rs`
/// fails CI if the current generation's requirement is not met. (#497)
pub const EVENT_ABI_TO_SCHEMA_VERSION: &[u32] = &[1, 2];

/// Event name constants — these form topic[0] of every event.
pub const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");
pub const EVENT_SETTLE_INTENT: Symbol = symbol_short!("set_int");
pub const EVENT_CONFIG_UPD: Symbol = symbol_short!("cfg_upd");
/// Emitted when a custom severity is removed via remove_custom_severity.
pub const EVENT_CONFIG_REM: Symbol = symbol_short!("cfg_rem");
/// Emitted when a new custom severity is registered (first creation).
/// Distinguishable from cfg_upd by indexers: the custom severity did not
/// exist before this call. (#456)
pub const EVENT_SEV_ADD: Symbol = symbol_short!("sev_add");
/// Emitted when an existing custom severity is reconfigured.
/// Distinguishable from sev_add by indexers: the custom severity already
/// existed before this call. (#456)
pub const EVENT_SEV_UPD: Symbol = symbol_short!("sev_upd");
pub const EVENT_PAUSED: Symbol = symbol_short!("paused");
pub const EVENT_UNPAUSED: Symbol = symbol_short!("unpause");
pub const EVENT_OP_SET: Symbol = symbol_short!("op_set");
pub const EVENT_PRUNED: Symbol = symbol_short!("pruned");
pub const EVENT_PRUNED_AGE: Symbol = symbol_short!("pruned_a");
pub const EVENT_ADMIN_PROP: Symbol = symbol_short!("adm_prop");
pub const EVENT_ADMIN_ACC: Symbol = symbol_short!("adm_acc");
pub const EVENT_ADMIN_CAN: Symbol = symbol_short!("adm_can");
pub const EVENT_ADMIN_REN: Symbol = symbol_short!("adm_ren");
pub const EVENT_OP_PROP: Symbol = symbol_short!("op_prop");
pub const EVENT_OP_ACC: Symbol = symbol_short!("op_acc");
pub const EVENT_OP_CAN: Symbol = symbol_short!("op_can");
/// Emitted when a pending admin proposal is superseded by a re-proposal. (#468)
pub const EVENT_ADMIN_SUP: Symbol = symbol_short!("adm_sup");
/// Emitted when a pending operator proposal is superseded by a re-proposal. (#468)
pub const EVENT_OP_SUP: Symbol = symbol_short!("op_sup");
pub const EVENT_CONFIG_FREEZE: Symbol = symbol_short!("cfg_frz");
pub const EVENT_CONFIG_UNFREEZE: Symbol = symbol_short!("cfg_unfrz");
/// Emitted when a running-stats counter saturates. (SC-W5-047)
pub const EVENT_STATS_SAT: Symbol = symbol_short!("stats_sat");
/// Emitted on the `DuplicateOutageInput` error path with the stored result. (#385)
pub const EVENT_DUP_INPUT: Symbol = symbol_short!("dup_input");
pub const EVENT_MIGRATE_DONE: &str = "migrate_done";

/// Returns the canonical event version string for consumer documentation.
pub fn current_event_version() -> Symbol {
    EVENT_VERSION
}

#[cfg(test)]
mod tests {
    // #496 – allows the emit-site audit below to read crate source. This crate
    // is `#![no_std]`, so `std` must be linked explicitly (as in orphan_lint).
    extern crate std;

    use super::*;
    use alloc::format;

    #[test]
    fn test_event_version_is_stable() {
        assert_eq!(current_event_version(), symbol_short!("v1"));
    }

    #[test]
    fn test_event_abi_generation_tracks_version_symbol() {
        // The generation table must have an entry for the current generation.
        assert!(
            EVENT_ABI_GENERATION >= 1 && EVENT_ABI_GENERATION as usize <= EVENT_ABI_TO_SCHEMA_VERSION.len(),
            "EVENT_ABI_GENERATION {} out of range for the policy table",
            EVENT_ABI_GENERATION
        );
        // Generation 1 is always "v1".
        assert_eq!(EVENT_ABI_GENERATION, 1);
        assert_eq!(current_event_version(), symbol_short!("v1"));
    }

    #[test]
    fn test_event_names_are_distinct() {
        let names = [
            EVENT_SLA_CALC,
            EVENT_SETTLE_INTENT,
            EVENT_CONFIG_UPD,
            EVENT_CONFIG_REM,
            EVENT_SEV_ADD,
            EVENT_SEV_UPD,
            EVENT_PAUSED,
            EVENT_UNPAUSED,
            EVENT_OP_SET,
            EVENT_PRUNED,
            EVENT_PRUNED_AGE,
            EVENT_ADMIN_PROP,
            EVENT_ADMIN_ACC,
            EVENT_ADMIN_CAN,
            EVENT_ADMIN_REN,
            EVENT_ADMIN_SUP,
            EVENT_OP_PROP,
            EVENT_OP_ACC,
            EVENT_OP_CAN,
            EVENT_OP_SUP,
            EVENT_CONFIG_FREEZE,
            EVENT_CONFIG_UNFREEZE,
            EVENT_STATS_SAT,
            EVENT_DUP_INPUT,
        ];

        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(
                    names[i], names[j],
                    "event name collision: {:?} == {:?}",
                    names[i], names[j]
                );
            }
        }
    }

    /// #496 – The compiler can only see an event constant is unused; the
    /// divergence that matters is *declared but never emitted*. Every event
    /// name constant in the catalog below must be referenced inside a `publish`
    /// call somewhere in the crate's `src/` (excluding this catalog and the
    /// `api_stability` guardrail). Adding an event constant to the schema
    /// without wiring it to an emit site fails this test.
    #[test]
    fn test_every_declared_event_has_an_emit_site() {
        use std::string::String;
        use std::vec::Vec;

        // (event name string, source identifier of the emitting constant).
        let catalog: [(&str, &str); 24] = [
            ("sla_calc", "EVENT_SLA_CALC"),
            ("set_int", "EVENT_SETTLE_INTENT"),
            ("cfg_upd", "EVENT_CONFIG_UPD"),
            ("cfg_rem", "EVENT_CONFIG_REM"),
            ("paused", "EVENT_PAUSED"),
            ("unpause", "EVENT_UNPAUSED"),
            ("op_set", "EVENT_OP_SET"),
            ("pruned", "EVENT_PRUNED"),
            ("pruned_a", "EVENT_PRUNED_AGE"),
            ("adm_prop", "EVENT_ADMIN_PROP"),
            ("adm_acc", "EVENT_ADMIN_ACC"),
            ("adm_can", "EVENT_ADMIN_CAN"),
            ("adm_ren", "EVENT_ADMIN_REN"),
            ("adm_sup", "EVENT_ADMIN_SUP"),
            ("op_prop", "EVENT_OP_PROP"),
            ("op_acc", "EVENT_OP_ACC"),
            ("op_can", "EVENT_OP_CAN"),
            ("op_sup", "EVENT_OP_SUP"),
            ("cfg_frz", "EVENT_CONFIG_FREEZE"),
            ("cfg_unfrz", "EVENT_CONFIG_UNFREEZE"),
            ("stats_sat", "EVENT_STATS_SAT"),
            ("dup_input", "EVENT_DUP_INPUT"),
            ("migrate_done", "EVENT_MIGRATE_DONE"),
            ("ret_lim", "EVENT_RET_LIM"),
        ];

        let src_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut sources: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&src_dir).expect("src/ readable for emit-site audit") {
            let entry = entry.expect("read_dir entry");
            let fname = entry.file_name().to_string_lossy().into_owned();
            // Skip this schema catalog and the api_stability guardrail: both are
            // *declaration* sites, not emit sites.
            if fname == "event_schema.rs" || fname == "api_stability.rs" || !fname.ends_with(".rs") {
                continue;
            }
            sources.push(std::fs::read_to_string(src_dir.join(&fname)).expect("cannot read source file"));
        }

        for (event_name, ident) in catalog {
            // A publish call can span several lines, so inspect a window after
            // every occurrence of "publish" rather than matching per line.
            let emitted = sources.iter().any(|text: &String| {
                text.match_indices("publish").any(|(idx, _)| {
                    let end = text.len().min(idx + 300);
                    text[idx..end].contains(ident)
                })
            });
            assert!(
                emitted,
                "event `{}` (constant `{}`) is declared in the schema catalog but never \
                 emitted from a publish site in src/ — wire it up or remove it (#496)",
                event_name, ident
            );
        }
    }

    #[test]
    fn test_event_version_is_short_enough() {
        let version_str = format!("{:?}", current_event_version());
        assert!(version_str.len() <= 32, "Version symbol too long");
    }
}
