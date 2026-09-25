//! Centralized event publishing helpers (Issue #656).
//!
//! Previously, events were emitted inline in many contract methods
//! (`publish_sla_event`, `publish_settlement_intent_event`,
//! `publish_duplicate_input_event`, governance emits) making the event surface
//! hard to enumerate and easy to drift from the schema catalog.
//!
//! This module provides a single `EventPublisher` struct that every emit-site
//! routes through. Topic construction, version stamping, and payload layout
//! all live here — removing the chance that a new emit-site uses a slightly
//! different topic order or forgets the version symbol.
//!
//! # Catalog
//!
//! Every event emittable by the contract is represented by a method on this
//! struct. The catalog is exhaustive by design: if an event is not here it
//! is not part of the official surface.
//!
//! | Method | Event name | Topic[2] |
//! |---|---|---|
//! | `sla_calc` | `sla_calc` | severity |
//! | `settlement_intent` | `set_int` | severity |
//! | `duplicate_input` | `dup_input` | severity |
//! | `config_updated` | `cfg_upd` | severity |
//! | `severity_added` | `sev_add` | severity |
//! | `severity_updated` | `sev_upd` | severity |
//! | `severity_removed` | `cfg_rem` | severity |
//! | `paused` | `paused` | caller |
//! | `unpaused` | `unpause` | caller |
//! | `config_frozen` | `cfg_frz` | caller |
//! | `config_unfrozen` | `cfg_unfrz` | caller |
//! | `pruned` | `pruned` | caller |
//! | `pruned_by_age` | `pruned_a` | caller |

use soroban_sdk::{Address, Env, Symbol};

use crate::{
    SLAResult, EVENT_CONFIG_FREEZE, EVENT_CONFIG_REM, EVENT_CONFIG_UNFREEZE, EVENT_CONFIG_UPD,
    EVENT_DUP_INPUT, EVENT_PAUSED, EVENT_PRUNED, EVENT_PRUNED_AGE, EVENT_SEV_ADD, EVENT_SEV_UPD,
    EVENT_SETTLE_INTENT, EVENT_SLA_CALC, EVENT_UNPAUSED, EVENT_VERSION,
};

/// Stateless event publisher — wraps every `env.events().publish()` call
/// behind a named method so the event catalog is enumerable in one place.
pub struct EventPublisher<'a> {
    env: &'a Env,
}

impl<'a> EventPublisher<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self { env }
    }

    /// Emit the primary SLA calculation event (`sla_calc`).
    pub fn sla_calc(&self, severity: Symbol, result: &SLAResult) {
        self.env.events().publish(
            (EVENT_SLA_CALC, EVENT_VERSION, severity),
            (
                result.outage_id.clone(),
                result.status.clone(),
                result.mttr_minutes,
                result.threshold_minutes,
                result.amount,
                result.payment_type.clone(),
                result.rating.clone(),
                result.config_version_hash,
                result.recorded_at,
            ),
        );
    }

    /// Emit the settlement intent event (`set_int`).
    pub fn settlement_intent(&self, severity: Symbol, result: &SLAResult) {
        self.env.events().publish(
            (EVENT_SETTLE_INTENT, EVENT_VERSION, severity),
            (
                result.outage_id.clone(),
                result.status.clone(),
                result.mttr_minutes,
                result.threshold_minutes,
                result.amount,
                result.payment_type.clone(),
                result.rating.clone(),
                result.config_version_hash,
                result.recorded_at,
            ),
        );
    }

    /// Emit the duplicate-input rejection event (`dup_input`).
    /// Includes the stored decision AND the rejected attempted inputs.
    pub fn duplicate_input(
        &self,
        severity: Symbol,
        existing: &SLAResult,
        attempted_mttr: u32,
        attempted_threshold: u32,
    ) {
        self.env.events().publish(
            (EVENT_DUP_INPUT, EVENT_VERSION, severity),
            (
                existing.outage_id.clone(),
                existing.status.clone(),
                existing.mttr_minutes,
                existing.threshold_minutes,
                existing.amount,
                existing.payment_type.clone(),
                existing.rating.clone(),
                existing.config_version_hash,
                existing.recorded_at,
                attempted_mttr,
                attempted_threshold,
            ),
        );
    }

    /// Emit a config update event (`cfg_upd`).
    pub fn config_updated(
        &self,
        severity: Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) {
        self.env.events().publish(
            (EVENT_CONFIG_UPD, EVENT_VERSION, severity),
            (threshold_minutes, penalty_per_minute, reward_base),
        );
    }

    /// Emit a new custom severity added event (`sev_add`).
    pub fn severity_added(
        &self,
        severity: Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) {
        self.env.events().publish(
            (EVENT_SEV_ADD, EVENT_VERSION, severity),
            (threshold_minutes, penalty_per_minute, reward_base),
        );
    }

    /// Emit a custom severity updated event (`sev_upd`).
    pub fn severity_updated(
        &self,
        severity: Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) {
        self.env.events().publish(
            (EVENT_SEV_UPD, EVENT_VERSION, severity),
            (threshold_minutes, penalty_per_minute, reward_base),
        );
    }

    /// Emit a custom severity removed event (`cfg_rem`).
    pub fn severity_removed(&self, severity: Symbol) {
        self.env
            .events()
            .publish((EVENT_CONFIG_REM, EVENT_VERSION, severity), ());
    }

    /// Emit the contract paused event (`paused`).
    pub fn paused(&self, caller: Address) {
        self.env
            .events()
            .publish((EVENT_PAUSED, EVENT_VERSION, caller), (true,));
    }

    /// Emit the contract unpaused event (`unpause`).
    pub fn unpaused(&self, caller: Address) {
        self.env
            .events()
            .publish((EVENT_UNPAUSED, EVENT_VERSION, caller), (false,));
    }

    /// Emit the config frozen event (`cfg_frz`).
    pub fn config_frozen(&self, caller: Address) {
        self.env
            .events()
            .publish((EVENT_CONFIG_FREEZE, EVENT_VERSION, caller), ());
    }

    /// Emit the config unfrozen event (`cfg_unfrz`).
    pub fn config_unfrozen(&self, caller: Address) {
        self.env
            .events()
            .publish((EVENT_CONFIG_UNFREEZE, EVENT_VERSION, caller), ());
    }

    /// Emit the pruned event (`pruned`).
    pub fn pruned(&self, caller: Address, removed_count: u32, kept_count: u32) {
        self.env.events().publish(
            (EVENT_PRUNED, EVENT_VERSION, caller),
            (removed_count, kept_count),
        );
    }

    /// Emit the pruned-by-age event (`pruned_a`).
    pub fn pruned_by_age(&self, caller: Address, removed_count: u32, kept_count: u32) {
        self.env.events().publish(
            (EVENT_PRUNED_AGE, EVENT_VERSION, caller),
            (removed_count, kept_count),
        );
    }
}