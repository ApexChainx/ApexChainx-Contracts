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
//! ## sla_calc (`sla_calc`)
//! Emitted on every successful `calculate_sla` call.
//! - topic[2]: severity Symbol
//! - payload:  (outage_id: Symbol, status: Symbol, payment_type: Symbol,
//!   rating: Symbol, mttr_minutes: u32, threshold_minutes: u32,
//!   amount: i128)
//!
//! ## set_int (`set_int`)
//! Settlement intent emitted alongside sla_calc for backend reconciliation.
//! - topic[2]: severity Symbol
//! - payload:  (outage_id: Symbol, status: Symbol, payment_type: Symbol,
//!   amount: i128, config_version_hash: u64, recorded_at: u64)
//!
//! ## cfg_upd (`cfg_upd`)
//! Emitted on every successful `set_config` call.
//! - topic[2]: severity Symbol
//! - payload:  (threshold_minutes: u32, penalty_per_minute: i128,
//!   reward_base: i128)
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
//! Emitted on operator change.
//! - topic[2]: caller Address
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

#![allow(dead_code)]

use soroban_sdk::{symbol_short, Symbol};

/// Canonical event version symbol used by all events.
pub const EVENT_VERSION: Symbol = symbol_short!("v1");

/// Event name constants — these form topic[0] of every event.
pub const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");
pub const EVENT_SETTLE_INTENT: Symbol = symbol_short!("set_int");
pub const EVENT_CONFIG_UPD: Symbol = symbol_short!("cfg_upd");
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
pub const EVENT_CONFIG_FREEZE: Symbol = symbol_short!("cfg_frz");
pub const EVENT_CONFIG_UNFREEZE: Symbol = symbol_short!("cfg_unfrz");
pub const EVENT_MIGRATE_DONE: &str = "migrate_done";

/// Returns the canonical event version string for consumer documentation.
pub fn current_event_version() -> Symbol {
    EVENT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_event_version_is_stable() {
        assert_eq!(current_event_version(), symbol_short!("v1"));
    }

    #[test]
    fn test_event_names_are_distinct() {
        let names = [
            EVENT_SLA_CALC,
            EVENT_SETTLE_INTENT,
            EVENT_CONFIG_UPD,
            EVENT_PAUSED,
            EVENT_UNPAUSED,
            EVENT_OP_SET,
            EVENT_PRUNED,
            EVENT_PRUNED_AGE,
            EVENT_ADMIN_PROP,
            EVENT_ADMIN_ACC,
            EVENT_ADMIN_CAN,
            EVENT_ADMIN_REN,
            EVENT_OP_PROP,
            EVENT_OP_ACC,
            EVENT_OP_CAN,
            EVENT_CONFIG_FREEZE,
            EVENT_CONFIG_UNFREEZE,
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

    #[test]
    fn test_event_version_is_short_enough() {
        let version_str = format!("{:?}", current_event_version());
        assert!(version_str.len() <= 32, "Version symbol too long");
    }
}
