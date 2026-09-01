//! ApexChainx SLA Calculator — Soroban smart contract for deterministic
//! SLA calculation, secure payment escrow, and multi-party settlement on the
//! Stellar network.
//!
//! This crate implements the core SLA calculator contract with configurable
//! severity levels, role-based access control, pause/unpause lifecycle,
//! cumulative statistics, event emission for backend indexing, and storage
//! version migration.
#![no_std]
// Note: `missing_docs` is intentionally NOT enabled as a crate lint here.
// Soroban's `#[contract]`/`#[contracttype]`/`#[contracterror]` proc macros
// generate undocumented public items that cannot be suppressed at the item
// level, so enabling `missing_docs` makes `cargo clippy -- -D warnings`
// (a CI gate) impossible to satisfy. Doc discipline is enforced by review
// instead. See git history for `5a4be57` (deny -> warn -> off).
extern crate alloc;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, String, Symbol, Vec,
};

#[allow(missing_docs)]
#[contract]
pub struct SLACalculatorContract;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fuzz_tests;

pub mod api_stability;
pub mod audit_state;
pub mod calculation;
pub mod config;
pub mod config_bundle;
pub mod config_freeze;
pub mod config_metadata;
pub mod contract_info;
pub mod coordination_harness;
pub mod cross_contract_safety;
pub mod deployment_policy;
pub mod error_responses;
pub mod event;
pub mod event_correlation;
mod event_schema;
/// Spec-assertion bodies shared by the `cargo-fuzz` targets in `fuzz/`.
/// The targets stay a few lines long and every invariant they check is
/// compiled and unit-tested here. See `docs/FUZZING_GUARANTEES.md`.
pub mod fuzz_spec;
pub mod governance;
pub mod history;
pub mod metadata;
pub mod metrics;
/// #422 – event payload-optimization helpers (consumer-side validation).
pub mod payload_optimizer;
pub mod policy;
/// Parity checker: compares current `compute_result` against the locked-in
/// canonical golden vectors in `test_snapshots/tests/parity_baseline.json`.
/// Run with `cargo test --lib parity_tests::` or `just parity-check`.
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod prune_benchmark;
#[cfg(test)]
mod schema_migration_tests;
/// Executable restatement of the contract's documented pure semantics —
/// the single source of truth imported by tests, the fuzz targets, and the
/// TypeScript parity fixtures instead of being re-derived in each.
pub mod spec;
pub mod storage_estimation;
#[cfg(test)]
mod storage_footprint_tests;
pub mod storage_version;
/// Generates the contract-derived fixtures that `ts/` parity-checks against.
/// Test-only: it executes the real contract in an `Env` and writes
/// `ts/fixtures/contract-read-semantics.json`.
#[cfg(test)]
mod ts_parity_fixtures;
/// #422 – formerly-orphan test-only modules, now declared so their guarantees
/// (threshold boundaries, auth matrix, event ordering/stability, payload
/// versioning, outage-id, pruning performance) are actually compiled and run.
#[cfg(test)]
mod auth_matrix_tests;
#[cfg(test)]
mod event_ordering_tests;
#[cfg(test)]
mod event_state_tests;
#[cfg(test)]
mod outage_id_tests;
#[cfg(test)]
mod payload_versioning_tests;
#[cfg(test)]
mod pruning_perf;
#[cfg(test)]
mod threshold_config;
#[cfg(test)]
mod topic_stability_tests;
pub mod version_negotiation;
#[cfg(test)]
mod orphan_lint_tests;

use crate::audit_state::AuditState;
use crate::config_bundle::ConfigBundle;

// -----------------------------------------------------------------------
// Storage Keys
// -----------------------------------------------------------------------
//
// These constants define all on-chain storage keys used by the contract.
// Each key maps to a specific semantic domain. Keys must be:
//   - Unique (no duplicate semantic domains)
//   - Stable across contract upgrades (new versions add new keys)
//   - Within the 9-character Symbol limit for Soroban
//
// Adding, removing, or renaming a key? Work through the checklist first:
//   docs/STORAGE_KEY_MIGRATION_CHECKLIST.md  (SC-509)
//
// References: Issue numbers track the original feature requirements.
//
// MAINTENANCE: When you add a new storage key constant here (or re-export
// one via `pub use`), you MUST also add it to the collision regression test:
//   apexchainx_calculator/src/tests.rs
//   → test_storage_key_namespace_symbols_are_distinct
// That test is the sole enforcement mechanism for namespace uniqueness.
// A missing entry is a silent aliasing bug waiting to happen.

/// Admin address — set during initialize, governs config and roles.
pub(crate) const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

/// Marker set by `renounce_admin` so that missing admin authority after a
/// permanent renounce is distinguishable from a never-initialized contract.
pub(crate) const ADMIN_RENOUNCED_KEY: Symbol = symbol_short!("ADMINRN");

/// Operator address — authorized to call calculate_sla. (#28)
pub(crate) const OPERATOR_KEY: Symbol = symbol_short!("OPERATOR");

/// Pending admin for two-step transfer. (#63)
pub(crate) const PENDING_ADMIN_KEY: Symbol = symbol_short!("PADMIN");
/// Pending operator for two-step handoff. (#64)
pub(crate) const PENDING_OP_KEY: Symbol = symbol_short!("POP");
/// Ledger timestamp when the pending admin proposal was made. (#405)
pub(crate) const PENDING_ADMIN_TS_KEY: Symbol = symbol_short!("PADMINTS");
/// Ledger timestamp when the pending operator proposal was made. (#405)
pub(crate) const PENDING_OP_TS_KEY: Symbol = symbol_short!("POPTS");

/// Map of severity -> SLAConfig for all configured severity levels.
pub(crate) const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

/// Map of severity -> SLAConfig for admin-defined custom severity levels,
/// distinct from the four canonical entries (critical/high/medium/low). (#93)
pub(crate) const CUSTOM_CONFIG_KEY: Symbol = symbol_short!("CUSTCFG");

/// Registry of `config_version_hash -> SLAConfigSnapshot`, recorded on every
/// config write so historical configs can be recovered for replay. (#408)
pub(crate) const CONFIG_REGISTRY_KEY: Symbol = symbol_short!("CFGREG");

/// Boolean flag: true when contract is paused. (#27)
pub(crate) const PAUSED_KEY: Symbol = symbol_short!("PAUSED");

/// Pause metadata (reason, timestamp, caller). (#66)
pub(crate) const PAUSE_INFO_KEY: Symbol = symbol_short!("PAUSEINF");

/// Maximum length (in bytes) for the pause reason string. (#68)
pub(crate) const MAX_REASON_LEN: usize = 256;

/// Cumulative SLA statistics (SLAStats struct). (#29)
pub(crate) const STATS_KEY: Symbol = symbol_short!("STATS");

/// Per-severity weekly calculation counters for telemetry. (#101)
pub(crate) const SEVERITY_CALC_COUNTS_KEY: Symbol = symbol_short!("CALCCNT");

/// Per-severity weekly violation counters for telemetry. (#101)
pub(crate) const SEVERITY_VIOL_COUNTS_KEY: Symbol = symbol_short!("VIOLCNT");

/// Per-severity last calculation timestamp for weekly windowing. (#101)
pub(crate) const LAST_CALCULATION_TS_KEY: Symbol = symbol_short!("CALCTS");

/// Per-severity last violation timestamp for weekly windowing. (#101)
pub(crate) const LAST_VIOLATION_TS_KEY: Symbol = symbol_short!("VIOLTS");

/// Ordered list of historical SLAResult entries.
pub(crate) const HISTORY_KEY: Symbol = symbol_short!("HIST");

/// Cached count of history entries (maintained alongside HISTORY_KEY).
/// This allows get_full_audit_state to report history length without deserializing
/// the full vector, addressing issue #463 (one-shot bootstrap efficiency).
pub(crate) const HISTORY_LEN_KEY: Symbol = symbol_short!("HISTLEN");

/// Current on-chain storage schema version number.
// INVARIANT: Once written, the value at STORAGE_VERSION_KEY must only be
// incremented by migrate(). All other read paths treat this key as read-only.
// No two storage keys may share the same Symbol value — this key is stable
// across all contract versions and may never be repurposed.
pub(crate) const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");

/// The storage schema version this contract binary expects.
/// Incremented when breaking state changes are introduced.
// v2 adds HISTORY_LEN_KEY.  This must stay in lockstep with migrate(): a
// deployed v1 contract has history but not the cached counter.
pub(crate) const STORAGE_VERSION: u32 = 2;

/// Version of the SLAResult schema exposed via get_result_schema().
/// Incremented when result encoding changes in a breaking way.
pub(crate) const RESULT_SCHEMA_VERSION: u32 = 1;

/// Number of named fields in `SLAResult`.
///
/// This constant is the migration guardrail for `get_result_schema()`.
/// It must be updated in the same commit that adds or removes a field from
/// `SLAResult`. The companion test `test_result_schema_field_count_sentinel`
/// in `schema_migration_tests.rs` will fail CI if the struct layout changes
/// without a corresponding update to this constant and `RESULT_SCHEMA_VERSION`.
///
/// **How to update when adding a field:**
/// 1. Add the field to `SLAResult`.
/// 2. Increment this constant.
/// 3. Increment `RESULT_SCHEMA_VERSION` (breaking change).
/// 4. Update `get_result_schema()` if a new symbol descriptor is needed.
/// 5. Add a CHANGELOG entry under `[Unreleased]` → `Changed`.
/// 6. See `docs/result-schema-migration-guard.md` for the full process.
pub(crate) const RESULT_SCHEMA_FIELD_COUNT: u32 = 9;

/// Version label of the SLAConfigSnapshot schema exposed via get_config_snapshot().
/// Incremented/bumped when snapshot layout changes in a breaking way.
pub(crate) const CONFIG_SNAPSHOT_SCHEMA_VERSION: Symbol = symbol_short!("v1");

/// Number of named fields in `SLAConfigSnapshot`.
///
/// This constant is the migration guardrail for `SLAConfigSnapshot`.
/// It must be updated in the same commit that adds or removes a field from
/// `SLAConfigSnapshot`. The companion test `test_config_snapshot_schema_field_count_sentinel`
/// in `schema_migration_tests.rs` will fail CI if the struct layout changes
/// without a corresponding update to this constant and `CONFIG_SNAPSHOT_SCHEMA_VERSION`.
pub(crate) const CONFIG_SNAPSHOT_SCHEMA_FIELD_COUNT: u32 = 2;

/// Hard upper bound on retained history entries. (SC-062)
/// Configurable down to 1 via set_retention_limit().
pub(crate) const MAX_HISTORY_SIZE: u32 = 1000;

/// Upper bound on the number of entries a single pagination call may return.
/// Limits above this are clamped so no single call can read the full retained
/// history, enforcing the documented pagination policy server-side.
pub(crate) const MAX_PAGE_SIZE: u32 = 200;

/// Anti-spam cap on how many retained history entries a single `outage_id` may
/// occupy.
///
/// `calculate_sla` is idempotent while the config hash is unchanged, so the only
/// way one outage can accumulate entries is a config change between submissions
/// (each change opens a new "generation" for that outage). Left uncapped, an
/// operator that resubmits the same outage after every config update can evict
/// every other outage from the retained window, so this bounds a single outage's
/// share of it.
///
/// Counted from the history scan `calculate_sla` already performs: no extra
/// storage key, no migration, and no dependency on call ordering. Admin pruning
/// (`prune_history` / `prune_history_by_age`) frees headroom again.
pub(crate) const MAX_RECALCS_PER_OUTAGE: u32 = 16;

/// Optional configurable retention limit override. (SC-013)
/// When set, overrides MAX_HISTORY_SIZE for history trimming.
pub(crate) const RETENTION_LIMIT_KEY: Symbol = symbol_short!("RETLIM");

/// Cumulative count of history entries removed by pruning (admin prune, age
/// prune, and automatic trim in calculate_sla). Used by `get_retention_metrics`
/// to compute retention ratio. (#461)
pub(crate) const TOTAL_PRUNED_KEY: Symbol = symbol_short!("TPRUNED");

/// Cumulative count of history entries ever stored (retained + pruned). Used
/// by `get_retention_metrics` to compute total_entries. (#461)
pub(crate) const TOTAL_ENTRIES_KEY: Symbol = symbol_short!("TTOTENT");

/// On-chain key storing the ledger sequence of the last config update. Re-exported
/// here so the storage-key namespace regression test catches any future collisions.
pub use crate::config_metadata::LAST_CFG_UPDATE_KEY;

// -----------------------------------------------------------------------
// Event Constants
// -----------------------------------------------------------------------
//
// All events use a standardised 3-topic layout:
//   topic[0] = event name (Symbol constant below)
//   topic[1] = event version ("v1")
//   topic[2] = event-specific context (severity, caller address, etc.)
//
// Payload field ordering and types are defined below per event variant.
// Breaking changes must increment the version symbol (v2, v3, ...).
// Additive fields (appended to the end) are NOT considered breaking.
//
// Full schema documentation: event_schema.rs
//
// ===== Event Payload Schemas =====
//
// The three decision-carrying events (sla_calc, set_int, dup_input) share a
// single canonical field order — the SLAResult struct order — so indexers
// parse one layout regardless of which decision event they consume (#429):
//
//   decision → (outage_id, status, mttr_minutes, threshold_minutes, amount,
//               payment_type, rating, config_version_hash, recorded_at)
//
// sla_calc  → (outage_id: Symbol, status: Symbol, mttr_minutes: u32,
//              threshold_minutes: u32, amount: i128, payment_type: Symbol,
//              rating: Symbol, config_version_hash: u64, recorded_at: u64)
//   context: severity Symbol
//
// cfg_upd   → (threshold_minutes: u32, penalty_per_minute: i128,
//              reward_base: i128)
//   context: severity Symbol
//
// paused    → (true,)
//   context: caller Address
//
// unpause   → (false,)
//   context: caller Address
//
// op_set    → (new_operator: Address,)
//   context: caller Address
//
// pruned    → (removed_count: u32, kept_count: u32)
//   context: caller Address
//
// pruned_a  → (removed_count: u32, kept_count: u32)
//   context: caller Address
//
// adm_prop  → (new_admin: Address,)
//   context: caller Address
//
// adm_acc   → ()
//   context: caller Address
//
// adm_can   → ()
//   context: caller Address
//
// adm_ren   → ()
//   context: caller Address
//
// adm_sup   → (superseded_admin: Address, new_admin: Address)
//   context: caller Address
//
// op_prop   → (new_operator: Address,)
//   context: caller Address
//
// op_acc    → ()
//   context: caller Address
//
// op_can    → ()
//   context: caller Address
//
// set_int   → (outage_id: Symbol, status: Symbol, mttr_minutes: u32,
//              threshold_minutes: u32, amount: i128, payment_type: Symbol,
//              rating: Symbol, config_version_hash: u64, recorded_at: u64)
//   context: severity Symbol
//
// dup_input → (outage_id: Symbol, status: Symbol, mttr_minutes: u32,
//              threshold_minutes: u32, amount: i128, payment_type: Symbol,
//              rating: Symbol, config_version_hash: u64, recorded_at: u64)
//   context: severity Symbol
//
// stats_sat → (field: Symbol, previous_value: i128, attempted_increment: i128)
//   context: counter_name Symbol
// -----------------------------------------------------------------------

/// Emitted on successful SLA calculation. Primary event for backend consumers.
///
/// Compatibility decision: appending new fields to the end of the payload tuple
/// is NOT breaking. Field reordering, removal, or type changes require bumping
/// EVENT_VERSION from "v1" to "v2". See event_schema.rs for the full payload
/// schema and the SC-099 checklist in CONTRIBUTING.md for the review process.
pub(crate) const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");

/// Emitted alongside sla_calc for settlement intent reconciliation.
///
/// Carries the full SLA decision (including `mttr_minutes`,
/// `threshold_minutes`, and `rating`) so a consumer processing only the
/// settlement stream can reconstruct the decision without a follow-up read.
///
/// Compatibility decision: shares the canonical decision field order
/// (`outage_id, status, mttr_minutes, threshold_minutes, amount,
/// payment_type, rating, config_version_hash, recorded_at`). Field additions
/// go at the end; any reorder or removal requires a version bump.
pub(crate) const EVENT_SETTLE_INTENT: Symbol = symbol_short!("set_int");

/// Emitted when configuration is updated via set_config.
///
/// Compatibility decision: payload is (threshold_minutes, penalty_per_minute,
/// reward_base) — the full config triple. Appending fields is safe; reordering
/// or removing fields requires a version bump.
pub(crate) const EVENT_CONFIG_UPD: Symbol = symbol_short!("cfg_upd");

/// Emitted when a custom severity is removed via remove_custom_severity.
///
/// Compatibility decision: payload is intentionally empty `()`, mirroring
/// other removal-style events. The removed severity is carried in topic[2].
pub(crate) const EVENT_CONFIG_REM: Symbol = symbol_short!("cfg_rem");

/// Emitted when a new custom severity is registered (first creation).
/// Distinguishable from cfg_upd by indexers: the custom severity did not
/// exist before this call. (#456)
///
/// Compatibility decision: payload is `(threshold_minutes, penalty_per_minute,
/// reward_base)` — same shape as cfg_upd. The distinct event name lets
/// indexers separate creation from update without state inspection.
pub(crate) const EVENT_SEV_ADD: Symbol = symbol_short!("sev_add");

/// Emitted when an existing custom severity is reconfigured.
/// Distinguishable from sev_add by indexers: the custom severity already
/// existed before this call. (#456)
///
/// Compatibility decision: payload is `(threshold_minutes, penalty_per_minute,
/// reward_base)` — same shape as cfg_upd.
pub(crate) const EVENT_SEV_UPD: Symbol = symbol_short!("sev_upd");

/// Emitted when the contract is paused by admin. (#27)
///
/// Compatibility decision: payload is `(true,)`. Empty-tuple expansion is
/// additive and safe; changing the boolean to a different type requires a
/// version bump.
pub(crate) const EVENT_PAUSED: Symbol = symbol_short!("paused");

/// Emitted when the contract is unpaused by admin. (#27)
///
/// Compatibility decision: payload is `(false,)`. Same rules as EVENT_PAUSED.
pub(crate) const EVENT_UNPAUSED: Symbol = symbol_short!("unpause");

/// Emitted when the operator address is changed. (#28)
///
/// Compatibility decision: payload is `(new_operator: Address,)`. Appending
/// additional fields is safe; changing the address field type requires a
/// version bump.
pub(crate) const EVENT_OP_SET: Symbol = symbol_short!("op_set");

/// Emitted after a prune_history call removes entries.
///
/// Compatibility decision: payload is `(removed_count: u32, kept_count: u32)`.
/// Appending fields is safe; reordering or type changes require a version bump.
pub(crate) const EVENT_PRUNED: Symbol = symbol_short!("pruned");

/// Emitted after a prune_history_by_age call removes entries. (SC-063)
///
/// Compatibility decision: same shape as EVENT_PRUNED — intentional parallel.
/// Field additions are safe; any structural change requires a version bump.
pub(crate) const EVENT_PRUNED_AGE: Symbol = symbol_short!("pruned_a");

/// Emitted when the retention limit is changed via set_retention_limit. (SC-013)
///
/// Compatibility decision: payload is `(new_limit: u32,)`. Appending fields is
/// safe; changing the type requires a version bump.
pub(crate) const EVENT_RET_LIM: Symbol = symbol_short!("ret_lim");

/// Emitted when a new admin is proposed. (#63)
///
/// Compatibility decision: payload is `(new_admin: Address,)`. Additive field
/// appends are safe; type or order changes require a version bump.
pub(crate) const EVENT_ADMIN_PROP: Symbol = symbol_short!("adm_prop");

/// Emitted when a pending admin proposal is accepted. (#63)
///
/// Compatibility decision: payload is intentionally empty `()`. If a payload
/// is ever added, EVENT_VERSION must be bumped.
pub(crate) const EVENT_ADMIN_ACC: Symbol = symbol_short!("adm_acc");

/// Emitted when a pending admin proposal is cancelled. (SC-024)
///
/// Compatibility decision: payload is intentionally empty `()`. Same rules as
/// EVENT_ADMIN_ACC.
pub(crate) const EVENT_ADMIN_CAN: Symbol = symbol_short!("adm_can");

/// Emitted when a pending admin proposal is superseded by a re-proposal
/// before the prior candidate accepted or cancelled. (#468)
///
/// Compatibility decision: payload is `(superseded_admin: Address,
/// new_admin: Address)`. Additive event name; appending fields is safe.
pub(crate) const EVENT_ADMIN_SUP: Symbol = symbol_short!("adm_sup");

/// Emitted when the admin permanently renounces their role. (#65)
///
/// Compatibility decision: payload is intentionally empty `()`. Renounce is
/// irreversible; the empty payload signals the transition without extra data.
pub(crate) const EVENT_ADMIN_REN: Symbol = symbol_short!("adm_ren");

/// Emitted when a new operator is proposed. (#64)
///
/// Compatibility decision: payload is `(new_operator: Address,)`. Same rules
/// as EVENT_ADMIN_PROP.
pub(crate) const EVENT_OP_PROP: Symbol = symbol_short!("op_prop");

/// Emitted when a pending operator proposal is accepted. (#64)
///
/// Compatibility decision: payload is intentionally empty `()`. Same rules as
/// EVENT_ADMIN_ACC.
pub(crate) const EVENT_OP_ACC: Symbol = symbol_short!("op_acc");

/// Emitted when a pending operator proposal is cancelled. (SC-024)
///
/// Compatibility decision: payload is intentionally empty `()`. Same rules as
/// EVENT_ADMIN_CAN.
pub(crate) const EVENT_OP_CAN: Symbol = symbol_short!("op_can");

/// Emitted when a pending operator proposal is superseded by a re-proposal
/// before the prior candidate accepted or cancelled. (#468)
///
/// Compatibility decision: payload is `(superseded_operator: Address,
/// new_operator: Address)`. Additive event name; appending fields is safe.
pub(crate) const EVENT_OP_SUP: Symbol = symbol_short!("op_sup");

/// Emitted when the configuration is frozen by admin.
///
/// Compatibility decision: payload is intentionally empty `()`. The freeze
/// event carries no payload because the caller address is in topic[2].
pub(crate) const EVENT_CONFIG_FREEZE: Symbol = symbol_short!("cfg_frz");

/// Emitted when the configuration is unfrozen by admin.
///
/// Compatibility decision: payload is intentionally empty `()`. Same rules as
/// EVENT_CONFIG_FREEZE.
pub(crate) const EVENT_CONFIG_UNFREEZE: Symbol = symbol_short!("cfg_unfrz");

/// Emitted when a running-stats counter saturates during increment_stats.
/// Signals backend indexers that the on-chain total capped and now
/// under-reports true economic exposure. (SC-W5-047)
///
/// Compatibility decision: payload is `(field: Symbol, previous_value: i128,
/// attempted_increment: i128)`. This is a diagnostic-only event; appending
/// fields is safe, but existing fields must not be reordered or re-typed
/// without a version bump, as backend alerting pipelines parse this shape.
pub(crate) const EVENT_STATS_SAT: Symbol = symbol_short!("stats_sat");

/// Emitted when `calculate_sla` rejects a conflicting duplicate `outage_id`
/// under an unchanged config version hash (the `DuplicateOutageInput` error
/// path). Carries the previously stored `SLAResult` so consumers can reconcile
/// the rejection without a separate `get_latest_by_outage` read. (#385)
///
/// Compatibility decision: payload mirrors the full `SLAResult` field order
/// (outage_id, status, mttr_minutes, threshold_minutes, amount, payment_type,
/// rating, config_version_hash, recorded_at). Field additions go at the end;
/// reordering, removal, or type changes require a version bump.
pub(crate) const EVENT_DUP_INPUT: Symbol = symbol_short!("dup_input");

/// Canonical event version symbol used by all events.
pub(crate) const EVENT_VERSION: Symbol = symbol_short!("v1");

// -----------------------------------------------------------------------
// Error Codes
// -----------------------------------------------------------------------
//
// All contract errors are represented as a u32 discriminant in the SLAError
// enum. Backend consumers can retrieve the full catalogue via
// `get_failure_schema()` which maps each code to a machine-readable label
// and human-readable description.
//
// Error codes are stable: once assigned, a code is never reused.
// New codes are appended to the end of the enum.
//
// For the formal failure taxonomy (categories, severity, consumer impact,
// and recovery strategies for every error code), see
// [`docs/FAILURE_TAXONOMY.md`](../docs/FAILURE_TAXONOMY.md).
// -----------------------------------------------------------------------

/// Contract has already been initialized — cannot initialize twice.
#[allow(missing_docs)]
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SLAError {
    /// initialize() was already called.
    AlreadyInitialized = 1,
    /// Contract has not been initialized yet.
    NotInitialized = 2,
    /// Caller lacks the required role (admin or operator).
    Unauthorized = 3,
    /// No configuration found for the given severity.
    ConfigNotFound = 4,
    /// On-chain storage version does not match binary expectation.
    VersionMismatch = 5,
    /// Contract is paused — state-changing operations are blocked. (#27)
    ContractPaused = 6,
    /// No pending transfer exists to accept or cancel. (#63, #64)
    NoPendingTransfer = 7,
    /// Threshold minutes outside valid range or severity-specific limit. (#70)
    InvalidThreshold = 8,
    /// Penalty per minute outside valid range or severity-specific limit. (#70)
    InvalidPenalty = 9,
    /// Reward base outside valid range. (#70)
    InvalidReward = 10,
    /// Severity not in supported list. (#70)
    InvalidSeverity = 11,
    /// Retention limit must be between 1 and MAX_HISTORY_SIZE. (SC-013)
    RetentionLimitOutOfRange = 12,
    /// Duplicate `outage_id` with conflicting inputs detected. (SC-W5-046)
    ///
    /// # Semantics
    ///
    /// `calculate_sla` enforces a deterministic duplicate-detection policy on
    /// every call:
    ///
    /// | Condition | Behaviour |
    /// |---|---|
    /// | `outage_id` is **new** (never seen before) | Calculation proceeds normally; result appended to history |
    /// | `outage_id` exists **and** the config version hash is **unchanged** **and** the inputs (`mttr_minutes`, `threshold_minutes`) **match** the previous entry exactly | **Idempotent** — returns the previously stored `SLAResult` without mutating state or emitting events |
    /// | `outage_id` exists **and** the config version hash is **unchanged** **but** the inputs **differ** | **DuplicateOutageInput** error — the caller submitted contradictory data for the same outage under the same config |
    /// | `outage_id` exists **and** the config version hash **changed** | Treated as a **fresh calculation** — the config update invalidates the previous entry, so the new result is appended to history |
    ///
    /// # Severity-Blind Detection
    ///
    /// The duplicate detection is **severity-blind**: it compares only `mttr_minutes`
    /// and `threshold_minutes` (via the config hash), not the severity argument.
    /// This means that if two severities have identical configuration parameters
    /// (e.g., both high and medium configured with threshold 30 / penalty 50 / reward 750),
    /// resubmitting the same outage under a different severity with the same MTTR is
    /// treated as an idempotent replay, not a conflict.
    ///
    /// **Rationale:** The stored `SLAResult` does not carry a severity field, so
    /// the contract cannot distinguish severity-only changes from true replays.
    /// Adding severity to the result schema requires a breaking migration. Until
    /// that migration is implemented, the contract treats severity as a routing
    /// parameter rather than a data dimension for duplicate detection.
    ///
    /// # Consumer guidance
    ///
    /// Backend callers that receive this error should:
    /// 1. Check whether the submitted `mttr_minutes` was entered incorrectly
    ///    (typo, stale measurement).
    /// 2. If the previous calculation was incorrect, the admin must call
    ///    `prune_history` to remove the conflicting entry before
    ///    re-submitting with corrected values — or wait for a config
    ///    update (which changes the version hash and allows a fresh entry).
    /// 3. If the intent is genuinely to re-evaluate the same outage under
    ///    the same config with different MTTR, the outage must receive a
    ///    new unique `outage_id`.
    ///
    /// The contract accompanies this error with a `dup_input` event
    /// (`EVENT_DUP_INPUT`) carrying the previously stored `SLAResult`, so
    /// consumers can read the conflicting result from the same transaction's
    /// event log instead of issuing a follow-up `get_latest_by_outage` call.
    DuplicateOutageInput = 13,
    /// Computed penalty amount is invalid (e.g., overflowed to zero). (SC-W5-046)
    InvalidPenaltyAmount = 14,
    /// Computed reward amount is invalid (e.g., zero or negative). (SC-W5-046)
    InvalidRewardAmount = 15,
    /// Configuration is frozen — config changes are blocked.
    ConfigFrozen = 16,
    /// Input parameter violates documented constraints (e.g., reason too long, mttr_minutes exceeds maximum). (#68)
    InvalidInput = 17,
    /// Custom severity referenced but not registered. (#93)
    SeverityNotInSet = 18,
    /// Outage already occupies MAX_RECALCS_PER_OUTAGE retained history entries.
    OutageRecalcLimit = 19,
    /// Pending admin/operator proposal has exceeded its expiry window.
    ProposalExpired = 20,
    /// Admin authority was permanently renounced — admin-gated calls are no longer possible. (#406)
    AdminRenounced = 21,
    /// Aggregate exposure totals overflowed i128 during summation.
    ///
    /// The read-only `get_economic_exposure` view sums per-severity
    /// `max_reward` and `penalty_per_minute` values using checked arithmetic.
    /// This error is returned when either aggregate overflows, preventing
    /// silent capping at `i128::MAX`. (SC-W5-047 alignment)
    ExposureOverflow = 22,
}

// -----------------------------------------------------------------------
// Core Data Types
// -----------------------------------------------------------------------
//
// These types form the contract's public API surface. They are serialised
// and deserialised by the Soroban SDK and exposed to backend consumers
// through read-only views and event payloads.
//
// All types derive Clone, Debug, and PartialEq for testability.
// Types marked #[contracttype] are Soroban-contract-compatible.
// -----------------------------------------------------------------------

/// Configuration parameters for a single severity level.
/// Each severity (critical, high, medium, low) has its own SLAConfig.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfig {
    /// Maximum allowed repair time in minutes before SLA is violated.
    pub threshold_minutes: u32,
    /// Penalty amount charged per minute of overtime (positive integer).
    pub penalty_per_minute: i128,
    /// Base reward amount for meeting SLA targets (positive integer).
    pub reward_base: i128,
}

/// Complete result of an SLA calculation, returned by calculate_sla
/// and calculate_sla_view.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAResult {
    /// Unique identifier for the outage event.
    pub outage_id: Symbol,
    /// SLA outcome: "met" (achieved) or "viol" (violated).
    pub status: Symbol,
    /// Measured time to repair in minutes.
    pub mttr_minutes: u32,
    /// Threshold that was applied for this severity.
    pub threshold_minutes: u32,
    /// Financial outcome: negative = penalty, positive = reward.
    pub amount: i128,
    /// Payment classification: "rew" (reward) or "pen" (penalty).
    pub payment_type: Symbol,
    /// Performance rating: "top" | "excel" | "good" | "poor".
    pub rating: Symbol,
    /// Deterministic hash of the config used for this evaluation.
    pub config_version_hash: u64,
    /// Ledger timestamp at calculation time. (SC-063)
    pub recorded_at: u64,
}

/// A single page of SLA history with pagination metadata.
///
/// `get_history_page_with_meta` returns this instead of a bare `Vec` so
/// consumers can detect the end of history and the total size in one read,
/// without a separate `get_history` or `get_retention_limit` call.
///
/// The `items` slice is identical to what `get_history_page` returns for the
/// same `(offset, limit)`; `total` is the full history length and `has_more`
/// is `true` when the requested range ends before the end of history **and**
/// `limit > 0`. When `limit == 0`, `has_more` is `false` (empty page signals
/// end-of-history).
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryPage {
    /// The entries in this page (oldest-first), up to `limit` items.
    pub items: Vec<SLAResult>,
    /// Total number of history entries currently stored.
    pub total: u32,
    /// Whether the requested range ends before the end of history (more
    /// entries can be fetched by advancing `offset`). When `limit == 0`,
    /// this is `false` (empty page signals end-of-history).
    pub has_more: bool,
}

/// A single severity-to-config mapping entry in a config snapshot.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfigEntry {
    /// Severity level (critical, high, medium, low).
    pub severity: Symbol,
    /// Configuration parameters for this severity.
    pub config: SLAConfig,
}

/// Ordered snapshot of all severity configurations, suitable for backend
/// consumption. Entries are in canonical severity order.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfigSnapshot {
    /// Schema version label (e.g., "v1").
    pub version: Symbol,
    /// Config entries in canonical severity order.
    pub entries: Vec<SLAConfigEntry>,
}

/// Describes the result encoding schema for backend consumers.
/// Backends use this to interpret SLA result symbols without
/// hard-coding symbol values.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAResultSchema {
    /// Schema version label.
    pub version: Symbol,
    /// Numeric schema version (incremented on breaking changes).
    pub schema_version: u32,
    /// Number of named fields in `SLAResult` at this schema version.
    /// Backends can compare this against their own deserialization code to
    /// detect layout drift without parsing the full field list.
    pub result_field_count: u32,
    /// Symbol for SLA met status.
    pub status_met: Symbol,
    /// Symbol for SLA violated status.
    pub status_violated: Symbol,
    /// Symbol for reward payment type.
    pub payment_reward: Symbol,
    /// Symbol for penalty payment type.
    pub payment_penalty: Symbol,
    /// Symbol for exceptional rating.
    pub rating_exceptional: Symbol,
    /// Symbol for excellent rating.
    pub rating_excellent: Symbol,
    /// Symbol for good rating.
    pub rating_good: Symbol,
    /// Symbol for poor rating.
    pub rating_poor: Symbol,
    /// Whether the SLAResult includes config_version_hash.
    pub includes_config_version_hash: bool,
    /// Deprecated symbols that are still emitted for backward compatibility.
    /// Each entry is (deprecated_symbol, replacement_symbol, deprecated_at_schema_version).
    pub deprecated_symbols: Vec<DeprecatedSymbol>,
    /// #239 – Deprecated severity alias mappings for historical result interpretation.
    /// Backends use this to translate old severity symbols to current ones.
    pub severity_aliases: Vec<SeverityAliasMapping>,
}

/// A deprecated symbol mapping that is still emitted for backward compatibility.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeprecatedSymbol {
    /// The deprecated symbol still present in events.
    pub old_symbol: Symbol,
    /// The replacement symbol that supersedes it.
    pub new_symbol: Symbol,
    /// The schema version at which this deprecation was introduced.
    pub deprecated_at: u32,
    /// The schema version at which the old symbol will be removed (None = not yet determined).
    pub removal_version: Option<u32>,
}

/// #239 – A deprecated severity alias mapping for historical result interpretation.
///
/// When a severity alias is renamed (e.g., "critical" → "crit"), historical SLAResult
/// entries retain the old severity symbol. This struct allows backends to translate
/// old severity symbols to current ones when querying historical data.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeverityAliasMapping {
    /// The deprecated severity symbol still present in historical results.
    pub old_severity: Symbol,
    /// The replacement severity symbol that supersedes it.
    pub new_severity: Symbol,
    /// The schema version at which this deprecation was introduced.
    pub deprecated_at: u32,
    /// The schema version at which the old severity was removed from active config.
    /// None indicates the severity is still valid (coexistence phase).
    pub removal_version: Option<u32>,
}

/// #60 – Single introspection call for backend clients.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    pub contract_name: Symbol,
    pub storage_version: u32,
    pub result_schema_version: u32,
    pub supported_severities: Vec<Symbol>,
    pub features: Vec<Symbol>,
}

// -----------------------------------------------------------------------
// #244 – Public API descriptor for stable frontend/backend discovery
// -----------------------------------------------------------------------

/// Describes a single public contract method for runtime API discovery.
///
/// Backend and frontend consumers can use this to programmatically discover
/// the contract's surface area without hard-coding method names, auth
/// requirements, or event contracts.
///
/// # Stability
///
/// This struct is **append-only** — new fields may be added at the end in
/// future versions. Removing or reordering fields is a breaking change that
/// requires a `RESULT_SCHEMA_VERSION` bump.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicApiMethod {
    /// The name of the contract method (e.g. "initialize", "calculate_sla").
    pub name: Symbol,
    /// Whether the method mutates storage (`true`) or is read-only (`false`).
    pub mutates: bool,
    /// Auth classification. Values:
    /// - `"admin"` – caller must hold the admin role.
    /// - `"operator"` – caller must hold the operator role.
    /// - `"addr"` – only a specific stored address may call (the pending
    ///   proposal slot holder must sign, e.g. `accept_admin`/`accept_operator`)
    ///   (#426).
    /// - `"none"` – no authorization gate (read-only / public).
    pub auth: Symbol,
    /// The primary event name emitted by this method, or `Symbol::new(env, "")` if none.
    pub event: Symbol,
}

/// Contract-level public API descriptor returned by `get_public_api()`.
///
/// Enumerates all public methods with their mutability, auth requirements,
/// and emitted events. Backend consumers can call this once at startup to
/// validate that the deployed contract matches their expected API surface.
///
/// # Stability
///
/// This struct is **append-only** — new fields may be added at the end in
/// future versions. The `version` field identifies the descriptor schema.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicApiDescriptor {
    /// Schema version for the descriptor ("v1").
    pub version: Symbol,
    /// The contract name ("sla_calc").
    pub contract_name: Symbol,
    /// All public methods in alphabetical order.
    pub methods: Vec<PublicApiMethod>,
}

/// #29 – Cumulative on-chain SLA performance metrics.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAStats {
    pub total_calculations: u64,
    pub total_violations: u64,
    pub total_rewards: i128,   // sum of all reward amounts paid out
    pub total_penalties: i128, // sum of all penalty amounts (stored positive)
}

/// #96 – Per-severity economic exposure for a single SLA event.
///
/// `max_reward` is the top-tier reward (`reward_base * 200 / 100`) — the most
/// a single perfectly-resolved event of this severity can pay out.
///
/// `penalty_per_minute` is the configured per-minute penalty rate — the
/// marginal cost of each overtime minute for a single violated event.
/// The total penalty for one event grows linearly: `overtime_minutes *
/// penalty_per_minute`. There is no contract-level cap on overtime, so the
/// dashboard must apply its own horizon when projecting worst-case exposure.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeverityExposure {
    /// Severity level (critical, high, medium, low).
    pub severity: Symbol,
    /// Maximum reward for a single top-tier event of this severity.
    pub max_reward: i128,
    /// Per-overtime-minute penalty rate for a single violated event.
    pub penalty_per_minute: i128,
}

/// #96 – Aggregate economic exposure view for backend dashboarding.
///
/// Summarises the maximum potential reward and the per-minute penalty rate
/// across all configured severities. This is a pure view — it reads only
/// the current severity configs and performs no state mutation.
///
/// `total_max_reward` is the sum of `max_reward` across all severities —
/// the total that would be paid out if one top-tier event occurred per
/// severity simultaneously.
///
/// `total_penalty_per_minute` is the sum of `penalty_per_minute` across all
/// severities — the aggregate cost rate if every severity had one ongoing
/// violation simultaneously.
///
/// `breakdown` contains one entry per canonical severity in canonical order
/// (critical → high → medium → low).
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EconomicExposure {
    /// Sum of `max_reward` across all severities.
    pub total_max_reward: i128,
    /// Sum of `penalty_per_minute` across all severities.
    pub total_penalty_per_minute: i128,
    /// Per-severity breakdown in canonical order.
    pub breakdown: Vec<SeverityExposure>,
}

/// #101 – Per-severity weekly violation-rate telemetry snapshot.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeverityTelemetry {
    pub severity: Symbol,
    pub calculations: u32,
    pub violations: u32,
    pub violation_rate: u32,
}

/// #66 – Pause metadata stored when the contract is paused.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseInfo {
    pub reason: String,
    pub paused_at: u64, // ledger timestamp (seconds)
    pub paused_by: Address,
}

/// #4 – Metadata about the most recent configuration update.
///
/// Wrapping the ledger sequence in a contract type (rather than exposing it
/// directly as `Option<u32>`) preserves the `Some`/`None` distinction when
/// the value crosses the Soroban contract client boundary — primitives
/// wrapped in `Option` are otherwise flattened and lose the null case.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigUpdateInfo {
    /// Ledger sequence at which the most recent `set_config` succeeded.
    pub sequence: u32,
}

/// SC-021 – Storage version and migration posture for off-chain consumers.
///
/// Backend consumers should call `get_migration_state` after any contract upgrade
/// to confirm the storage version matches expectations before resuming operations.
/// If `needs_migration` is true, the admin must call `migrate` before the contract
/// will accept versioned calls.
///
/// # Consumption Guide (Backend / Operator Tooling)
///
/// ## Startup Handshake Flow
///
/// 1. **Call `get_migration_state()`** immediately after establishing a connection
///    to the contract (e.g. at backend startup, after a reconnect).
/// 2. **Inspect `needs_migration`:**
///    - `false` → the contract is on the expected version. Proceed with normal
///      operations. No action needed.
///    - `true`  → the contract storage version differs from what this binary
///      expects. **Do not** issue operational transactions (`calculate_sla`,
///      `set_config`, `pause`, etc.) — they will fail with `VersionMismatch`.
/// 3. **If `needs_migration` is `true`, compare the versions:**
///    - If `stored_version < expected_version` → the contract is behind.
///      An admin must invoke `migrate()` to upgrade the storage layout.
///    - If `stored_version > expected_version` → the contract is ahead.
///      The backend binary is outdated and must be upgraded first.
/// 4. **Poll after migration:** Re-call `get_migration_state()` after `migrate()`
///    completes to confirm `needs_migration` is now `false`.
///
/// ## Operator Tooling Integration
///
/// - **Monitoring alerts:** Set up a health-check loop that calls
///   `get_migration_state()` every N blocks. Alert if `needs_migration` flips to
///   `true` unexpectedly (indicates an unauthorised upgrade or state corruption).
/// - **Deployment pipelines:** Add a pre-deployment step that reads the current
///   `stored_version` from the live contract and compares it against the
///   `expected_version` of the binary being deployed. Block the deployment if
///   `needs_migration` would be `true` after upgrade.
/// - **Canary checks:** Before rolling out a new backend version to all
///   instances, have a canary instance call `get_migration_state()` against a
///   staging contract that has been upgraded. Verify `needs_migration` is
///   `false` before promoting the release.
///
/// ## Error Handling
///
/// - `NotInitialized` is returned if the contract has never been initialized
///   (no `STORAGE_VERSION_KEY` present). This is a permanent error — the
///   contract must be initialized before any operations are possible.
/// - `get_migration_state()` intentionally **bypasses** `check_version()` so it
///   remains callable even when the contract is in a pre-migration state.
///
/// ## Example: Backend Startup Sequence
///
/// ```ignore
/// // Pseudocode — adapt to your language/runtime
/// let state = contract.get_migration_state();
/// if state.needs_migration {
///     if state.stored_version < state.expected_version {
///         log.warn("Contract needs upgrade. Calling migrate()...");
///         admin_wallet.invoke(contract.migrate());
///         // Re-check after migration
///         state = contract.get_migration_state();
///         if state.needs_migration {
///             throw new Error("Migration did not resolve version mismatch");
///         }
///     } else {
///         throw new Error(
///             "Backend is outdated. Expected version " +
///             state.expected_version + " but contract is at " +
///             state.stored_version);
///     }
/// }
/// log.info("Contract is ready. Storage version: " + state.stored_version);
/// ```
///
/// See [`docs/MIGRATION_STATE_CONSUMPTION.md`] for the full consumption guide
/// with diagrams, troubleshooting, and operator runbook.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageVersionInfo {
    /// The version currently stamped in storage.
    pub stored_version: u32,
    /// The version this contract binary expects.
    pub expected_version: u32,
    /// True when stored_version != expected_version (migration required).
    pub needs_migration: bool,
}

/// SC-W5-046 – Typed failure code mapping entry for backend bridge consumption.
///
/// Each `FailureCode` maps a numeric error code to a machine-readable Symbol
/// label and a short human-readable description. Backends call
/// `get_failure_schema` to obtain the full catalogue at startup.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureCode {
    /// The numeric error code matching the SLAError discriminant.
    pub code: u32,
    /// A machine-readable Symbol label (e.g. "AlreadyInitialized").
    pub label: Symbol,
    /// A short description of the failure condition.
    pub description: Symbol,
}

/// SC-W5-046 – Full catalogue of typed failure codes for backend bridge.
///
/// Backend consumers can call `get_failure_schema` once at startup to
/// pre-load all possible failure codes the contract may return. The schema
/// is versioned to allow backwards-compatible additions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureSchema {
    /// Schema version for the failure code catalogue.
    pub version: Symbol,
    /// All known failure codes in ascending order.
    pub codes: Vec<FailureCode>,
}

/// #218 – Read-only healthcheck result for backend startup readiness.
///
/// Backend consumers call `healthcheck` before any other operation to confirm
/// the contract is in a safe state. Unlike `get_version_info` which also
/// bypasses `check_version`, this endpoint returns a single boolean outcome
/// for simple load-balancer probes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HealthcheckResult {
    /// True when the contract is initialised and on the current storage version.
    pub ready: bool,
    /// Human-readable contract name for log correlation.
    pub contract_name: Symbol,
    /// Human-readable status label: "ok", "not_initialized", "needs_migration".
    pub status: Symbol,
}

/// #261 – Contract state fingerprint for release review and upgrade planning.
///
/// Provides a compact, deterministic snapshot of the contract's live state by
/// combining storage version, configuration hash, pause state, and migration
/// posture into a single reviewable summary. This is safe to call on a live
/// contract without mutating state.
///
/// Backend consumers can call this before and after upgrades or during incident
/// response to quickly audit the contract's current posture without issuing
/// separate queries for each field.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractStateFingerprint {
    /// Human-readable contract name for log correlation.
    pub contract_name: Symbol,
    /// Storage schema version stamped in contract storage.
    pub storage_version: u32,
    /// Result schema version for SLAResult field layout.
    pub result_schema_version: u32,
    /// Deterministic hash of the current configuration snapshot.
    /// Changes whenever set_config is called with different values.
    pub config_version_hash: u64,
    /// True when the contract is currently paused.
    pub is_paused: bool,
    /// True when stored storage version differs from the binary's expected version.
    pub needs_migration: bool,
    /// True when the configuration is frozen (admin cannot call set_config).
    pub is_config_frozen: bool,
    /// Ledger timestamp when this fingerprint was captured (seconds).
    pub captured_at: u64,
}

/// SC-W5-029 – Combined version negotiation response for backend startup handshake.
///
/// Backend consumers call `get_version_info` once at startup (or after an upgrade)
/// to determine whether the contract is safe to use. All version-relevant fields
/// are returned in a single read to minimise round-trips.
///
/// Decision logic for backends:
/// - `needs_migration == true`  → block operations, alert admin to call `migrate`
/// - `is_paused == true`        → surface pause reason, retry after `unpause`
/// - `storage_version != result_schema_version` (unexpected) → log and alert
/// - otherwise                  → contract is ready; proceed normally
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionInfo {
    /// Storage schema version stamped in contract storage.
    pub storage_version: u32,
    /// Result schema version for SLAResult field layout.
    pub result_schema_version: u32,
    /// True when stored storage version differs from the binary's expected version.
    pub needs_migration: bool,
    /// True when the contract is currently paused.
    pub is_paused: bool,
    /// Human-readable contract name for log correlation.
    pub contract_name: Symbol,
}

// -----------------------------------------------------------------------
// Contract implementation
// -----------------------------------------------------------------------
//
// CONTRIBUTOR NOTICE:
// Any new public methods or modifications to existing ones must be reviewed
// against the SC-100 Public Method Review Checklist in CONTRIBUTING.md.
// This ensures event schema, versioning, and migration rules are upheld.
#[contractimpl]
impl SLACalculatorContract {
    // -------------------------------------------------------------------
    // Initialisation
    // -------------------------------------------------------------------

    /// Deploy the contract.
    /// `admin`    – may update config, pause/unpause, and assign the operator.
    /// `operator` – may call `calculate_sla`.
    ///
    /// # Role Distinctness & Single-Address Mode
    /// Both `admin` and `operator` signatures are required at initialization. However, `admin` and
    /// `operator` may be set to the same address for single-key / merged-role deployments where
    /// role separation is not required. In this case, both authorization checks are satisfied by a single signature.
    pub fn initialize(env: Env, admin: Address, operator: Address) -> Result<(), SLAError> {
        if env.storage().instance().has(&ADMIN_KEY) {
            return Err(SLAError::AlreadyInitialized);
        }

        admin.require_auth();
        operator.require_auth();

        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage().instance().set(&OPERATOR_KEY, &operator); // #28
        env.storage().instance().set(&PAUSED_KEY, &false); // #27

        // #29 – initialise zeroed stats
        env.storage().instance().set(
            &STATS_KEY,
            &SLAStats {
                total_calculations: 0,
                total_violations: 0,
                total_rewards: 0,
                total_penalties: 0,
            },
        );
        env.storage().instance().set(&SEVERITY_CALC_COUNTS_KEY, &0u128);
        env.storage().instance().set(&SEVERITY_VIOL_COUNTS_KEY, &0u128);
        env.storage().instance().set(&LAST_CALCULATION_TS_KEY, &0u128);
        env.storage().instance().set(&LAST_VIOLATION_TS_KEY, &0u128);
        env.storage()
            .instance()
            .set(&HISTORY_KEY, &Vec::<SLAResult>::new(&env));
        // Issue #463: initialize cached history length
        env.storage().instance().set(&HISTORY_LEN_KEY, &0u32);

        let mut configs = Map::<Symbol, SLAConfig>::new(&env);
        configs.set(
            symbol_short!("critical"),
            SLAConfig {
                threshold_minutes: 15,
                penalty_per_minute: 100,
                reward_base: 750,
            },
        );
        configs.set(
            symbol_short!("high"),
            SLAConfig {
                threshold_minutes: 30,
                penalty_per_minute: 50,
                reward_base: 750,
            },
        );
        configs.set(
            symbol_short!("medium"),
            SLAConfig {
                threshold_minutes: 60,
                penalty_per_minute: 25,
                reward_base: 750,
            },
        );
        configs.set(
            symbol_short!("low"),
            SLAConfig {
                threshold_minutes: 120,
                penalty_per_minute: 10,
                reward_base: 600,
            },
        );

        env.storage().instance().set(&CONFIG_KEY, &configs);
        // #455 – Seed CUSTOM_CONFIG_KEY so fresh and migrated contracts
        // have the same instance-storage key layout.
        env.storage()
            .instance()
            .set(&CUSTOM_CONFIG_KEY, &Map::<Symbol, SLAConfig>::new(&env));
        Self::write_version(&env);
        Ok(())
    }

    // Initialise any storage keys that may be missing from older schema
    // versions. This is intentionally conservative: only set a value when
    // the key is absent so migration is idempotent and does not overwrite
    // existing state.
    fn init_missing_storage_defaults(env: &Env) {
        let inst = env.storage().instance();

        if !inst.has(&PAUSED_KEY) {
            inst.set(&PAUSED_KEY, &false);
        }

        if !inst.has(&STATS_KEY) {
            inst.set(
                &STATS_KEY,
                &SLAStats {
                    total_calculations: 0,
                    total_violations: 0,
                    total_rewards: 0,
                    total_penalties: 0,
                },
            );
        }

        if !inst.has(&SEVERITY_CALC_COUNTS_KEY) {
            inst.set(&SEVERITY_CALC_COUNTS_KEY, &0u128);
        }

        if !inst.has(&SEVERITY_VIOL_COUNTS_KEY) {
            inst.set(&SEVERITY_VIOL_COUNTS_KEY, &0u128);
        }

        if !inst.has(&LAST_CALCULATION_TS_KEY) {
            inst.set(&LAST_CALCULATION_TS_KEY, &0u128);
        }

        if !inst.has(&LAST_VIOLATION_TS_KEY) {
            inst.set(&LAST_VIOLATION_TS_KEY, &0u128);
        }

        if !inst.has(&HISTORY_KEY) {
            inst.set(&HISTORY_KEY, &Vec::<SLAResult>::new(env));
        }

        // Issue #463: HISTORY_LEN_KEY caches the history length so
        // `get_full_audit_state` can report it without materializing the full
        // history vector. A contract migrated from a schema that predates this
        // key must backfill it from the *actual* history length — which may be
        // non-empty — rather than defaulting to 0, or the bootstrap read would
        // under-report history size until the next write refreshes the cache.
        // This one-time O(n) read runs during migration, not on the hot path.
        if !inst.has(&HISTORY_LEN_KEY) {
            let history: Vec<SLAResult> = inst.get(&HISTORY_KEY).unwrap_or_else(|| Vec::new(env));
            inst.set(&HISTORY_LEN_KEY, &history.len());
        }

        if !inst.has(&CONFIG_KEY) {
            let mut configs = Map::<Symbol, SLAConfig>::new(env);
            configs.set(
                symbol_short!("critical"),
                SLAConfig {
                    threshold_minutes: 15,
                    penalty_per_minute: 100,
                    reward_base: 750,
                },
            );
            configs.set(
                symbol_short!("high"),
                SLAConfig {
                    threshold_minutes: 30,
                    penalty_per_minute: 50,
                    reward_base: 750,
                },
            );
            configs.set(
                symbol_short!("medium"),
                SLAConfig {
                    threshold_minutes: 60,
                    penalty_per_minute: 25,
                    reward_base: 750,
                },
            );
            configs.set(
                symbol_short!("low"),
                SLAConfig {
                    threshold_minutes: 120,
                    penalty_per_minute: 10,
                    reward_base: 600,
                },
            );
            inst.set(&CONFIG_KEY, &configs);
        }

        if !inst.has(&CUSTOM_CONFIG_KEY) {
            inst.set(&CUSTOM_CONFIG_KEY, &Map::<Symbol, SLAConfig>::new(env));
        }
    }

    // -------------------------------------------------------------------
    // #61 – Storage migration harness
    // -------------------------------------------------------------------

    /// Migrate storage from a previous version to the current one.
    ///
    /// Must be called by admin after a contract upgrade that bumps STORAGE_VERSION.
    /// The harness applies each step in sequence (v0→v1, v1→v2, …) so a contract
    /// that is multiple versions behind is brought fully up to date in one call.
    /// Re-invoking when already current is a safe no-op (idempotent).
    /// If an unknown stored version is encountered the call returns
    /// `VersionMismatch` without mutating any state.
    pub fn migrate(env: Env, caller: Address) -> Result<(), SLAError> {
        // Require admin without going through check_version (state may be unversioned)
        caller.require_auth();
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if caller != admin {
            return Err(SLAError::Unauthorized);
        }

        let stored: u32 = env.storage().instance().get(&STORAGE_VERSION_KEY).unwrap_or(0);

        // Already current – idempotent no-op
        if stored == STORAGE_VERSION {
            return Ok(());
        }

        // Reject versions newer than what this binary knows about
        if stored > STORAGE_VERSION {
            return Err(SLAError::VersionMismatch);
        }

        // Apply each step in sequence.  Each arm must be a pure, atomic
        // transformation: read old state → write new state → bump version.
        // A future version bump adds a new arm here; existing arms are never
        // modified so older migration paths remain auditable.
        let mut current = stored;

        // v0 → v1: stamp the version; all other fields were set by initialize
        if current == 0 {
            // Ensure any storage keys that might be missing from older
            // deployments are initialised to deterministic defaults before
            // we mark the storage version as migrated. This codifies the
            // contract: migration arms must initialise newly-added keys.
            Self::init_missing_storage_defaults(&env);
            env.storage().instance().set(&STORAGE_VERSION_KEY, &1u32);
            current = 1;
        }

        // v1 → v2: backfill the cached history length introduced by #463.
        // This is deliberately derived from the source-of-truth vector once
        // during migration; read paths thereafter use the counter directly.
        if current == 1 {
            let history: Vec<SLAResult> = env
                .storage()
                .instance()
                .get(&HISTORY_KEY)
                .unwrap_or_else(|| Vec::new(&env));
            env.storage().instance().set(&HISTORY_LEN_KEY, &history.len());
            env.storage().instance().set(&STORAGE_VERSION_KEY, &2u32);
            current = 2;
        }

        // Sanity: after all steps we must be at STORAGE_VERSION
        if current != STORAGE_VERSION {
            return Err(SLAError::VersionMismatch);
        }

        env.events().publish(
            (
                soroban_sdk::Symbol::new(&env, event_schema::EVENT_MIGRATE_DONE),
                event_schema::EVENT_VERSION,
                caller,
            ),
            (stored, current),
        );

        Ok(())
    }

    // -------------------------------------------------------------------
    // Role queries
    // -------------------------------------------------------------------

    /// Returns the current admin address.
    pub fn get_admin(env: Env) -> Result<Address, SLAError> {
        Self::check_version(&env)?;
        env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    /// #28 – Returns the current operator address.
    pub fn get_operator(env: Env) -> Result<Address, SLAError> {
        Self::check_version(&env)?;
        env.storage()
            .instance()
            .get(&OPERATOR_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    // -------------------------------------------------------------------
    // #28 – Operator management (admin only)
    // -------------------------------------------------------------------

    /// Replace the operator address directly (single-step, admin only).
    ///
    /// This is the legacy break-glass path. It does **not** require the new
    /// operator's consent — only the admin authorizes the change. Emits an
    /// `op_set` event (distinguishable from the two-step `op_prop`/`op_acc`
    /// trail). For routine rotations, prefer `propose_operator` +
    /// `accept_operator`.
    pub fn set_operator(env: Env, caller: Address, new_operator: Address) -> Result<(), SLAError> {
        governance::set_operator(&env, &caller, &new_operator)
    }

    // -------------------------------------------------------------------
    // #63 – Two-step admin transfer
    // -------------------------------------------------------------------

    /// Propose a new admin. The current admin initiates; the new admin must call `accept_admin`.
    pub fn propose_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), SLAError> {
        governance::propose_admin(&env, &caller, &new_admin)
    }

    /// Accept a pending admin transfer. Must be called by the proposed new admin.
    /// On success the caller becomes admin and the pending proposal is cleared.
    pub fn accept_admin(env: Env, caller: Address) -> Result<(), SLAError> {
        governance::accept_admin(&env, &caller)
    }

    /// Cancel a pending admin transfer. Only the current admin may cancel.
    /// Clears the pending proposal without changing the active admin.
    /// Returns `NoPendingTransfer` if there is nothing to cancel.
    pub fn cancel_admin_proposal(env: Env, caller: Address) -> Result<(), SLAError> {
        governance::cancel_admin_proposal(&env, &caller)
    }

    /// Returns the pending admin address, if any.
    pub fn get_pending_admin(env: Env) -> Result<Option<Address>, SLAError> {
        governance::get_pending_admin(&env)
    }

    /// Returns the estimated total storage footprint size in bytes.
    pub fn get_storage_footprint_estimate(env: Env) -> Result<u64, SLAError> {
        storage_estimation::get_storage_footprint_estimate(&env)
    }

    /// Returns an approximate per-ledger rent cost in stroops based on storage footprint.
    ///
    /// **Note (#459):** This is a relative growth proxy, not an authoritative
    /// rent figure. See `storage_estimation::get_rent_estimate` for details.
    pub fn get_rent_estimate(env: Env) -> Result<i128, SLAError> {
        storage_estimation::get_rent_estimate(&env)
    }

    // -------------------------------------------------------------------
    // #64 – Two-step operator handoff
    // -------------------------------------------------------------------

    /// Propose a new operator (step 1 of the canonical two-step handoff).
    /// The current admin initiates; the new operator must call `accept_operator`
    /// to consent and complete the transfer. Emits `op_prop`.
    pub fn propose_operator(env: Env, caller: Address, new_operator: Address) -> Result<(), SLAError> {
        governance::propose_operator(&env, &caller, &new_operator)
    }

    /// Accept a pending operator handoff (step 2 of the canonical two-step handoff).
    /// Must be called by the proposed new operator, requiring their explicit consent.
    /// Emits `op_acc`.
    pub fn accept_operator(env: Env, caller: Address) -> Result<(), SLAError> {
        governance::accept_operator(&env, &caller)
    }

    /// Cancel a pending operator proposal. Only the current admin may cancel.
    /// Clears the pending proposal without changing the active operator.
    /// Returns `NoPendingTransfer` if there is nothing to cancel.
    pub fn cancel_operator_proposal(env: Env, caller: Address) -> Result<(), SLAError> {
        governance::cancel_operator_proposal(&env, &caller)
    }

    /// Returns the pending operator address, if any.
    pub fn get_pending_operator(env: Env) -> Result<Option<Address>, SLAError> {
        governance::get_pending_operator(&env)
    }

    // -------------------------------------------------------------------
    // #65 – Admin renounce
    // -------------------------------------------------------------------

    /// Permanently renounce admin authority. This is irreversible: no admin will
    /// exist after this call and admin-gated functions will be permanently locked.
    /// Any pending admin proposal is also cleared.
    pub fn renounce_admin(env: Env, caller: Address) -> Result<(), SLAError> {
        governance::renounce_admin(&env, &caller)
    }

    /// Pause the contract with a reason and timestamp.
    /// `calculate_sla` will be blocked until unpaused.
    /// Emits a `paused` event.
    pub fn pause(env: Env, caller: Address, reason: String) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        if reason.len() > MAX_REASON_LEN as u32 {
            return Err(SLAError::InvalidInput);
        }

        let paused_at = env.ledger().timestamp();
        env.storage().instance().set(&PAUSED_KEY, &true);
        env.storage().instance().set(
            &PAUSE_INFO_KEY,
            &PauseInfo {
                reason,
                paused_at,
                paused_by: caller.clone(),
            },
        );
        env.events()
            .publish((EVENT_PAUSED, EVENT_VERSION, caller), (true,));
        Ok(())
    }

    /// Unpause the contract. Clears pause metadata.
    /// Emits an `unpause` event.
    pub fn unpause(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        env.storage().instance().set(&PAUSED_KEY, &false);
        env.storage().instance().remove(&PAUSE_INFO_KEY);
        env.events()
            .publish((EVENT_UNPAUSED, EVENT_VERSION, caller), (false,));
        Ok(())
    }

    /// Returns `true` when the contract is paused.
    pub fn is_paused(env: Env) -> Result<bool, SLAError> {
        Self::check_version(&env)?;
        Ok(env.storage().instance().get(&PAUSED_KEY).unwrap_or(false))
    }

    /// Returns pause metadata (reason + timestamp) if currently paused, else None.
    pub fn get_pause_info(env: Env) -> Result<Option<PauseInfo>, SLAError> {
        Self::check_version(&env)?;
        Ok(env.storage().instance().get(&PAUSE_INFO_KEY))
    }

    // -------------------------------------------------------------------
    // Config freeze / unfreeze (admin only)
    // -------------------------------------------------------------------

    /// Freezes the configuration, blocking further config updates.
    /// Admin only. Emits a `cfg_frz` event.
    pub fn freeze_config(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        config_freeze::freeze_config(&env);
        env.events()
            .publish((EVENT_CONFIG_FREEZE, EVENT_VERSION, caller), ());
        Ok(())
    }

    /// Unfreezes the configuration, re-allowing config updates.
    /// Admin only. Emits a `cfg_unfrz` event.
    pub fn unfreeze_config(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        config_freeze::unfreeze_config(&env);
        env.events()
            .publish((EVENT_CONFIG_UNFREEZE, EVENT_VERSION, caller), ());
        Ok(())
    }

    /// Returns `true` when the configuration is currently frozen.
    pub fn is_config_frozen(env: Env) -> Result<bool, SLAError> {
        Self::check_version(&env)?;
        Ok(config_freeze::is_config_frozen(&env))
    }

    // -------------------------------------------------------------------
    // Config management (admin only)                                 #28
    // -------------------------------------------------------------------

    /// Updates the configuration for a canonical severity level. Admin only.
    pub fn set_config(
        env: Env,
        caller: Address,
        severity: Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?; // #28 – admin role enforced
        Self::require_not_frozen(&env)?;

        // #70 – Validate configuration parameters
        Self::validate_config(&severity, threshold_minutes, penalty_per_minute, reward_base)?;

        // #92 – Cross-severity penalty ordering: enforce severity progression
        // so higher-severity penalties are never lower than lower-severity ones.
        Self::validate_cross_severity_penalty_ordering(&env, &severity, penalty_per_minute)?;

        // Cross-severity threshold ordering: enforce that critical <= high <= medium <= low
        // so that more severe outages always have shorter response windows.
        Self::validate_cross_severity_threshold_ordering(&env, &severity, threshold_minutes)?;

        let mut configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;

        configs.set(
            severity.clone(),
            SLAConfig {
                threshold_minutes,
                penalty_per_minute,
                reward_base,
            },
        );
        env.storage().instance().set(&CONFIG_KEY, &configs);

        // Issue #4 – stamp the ledger sequence of the most recent config
        // update so backends can detect when their cached configuration is
        // stale. Called after the storage write so the recorded sequence
        // always reflects a successful update.
        config_metadata::record_config_update(&env);

        // #408 – record the config snapshot under its new version hash so
        // historical configs remain recoverable for deterministic replay.
        Self::record_config_registry(&env)?;

        env.events().publish(
            (EVENT_CONFIG_UPD, EVENT_VERSION, severity),
            (threshold_minutes, penalty_per_minute, reward_base),
        );
        Ok(())
    }

    /// Returns the configuration for the given severity.
    pub fn get_config(env: Env, severity: Symbol) -> Result<SLAConfig, SLAError> {
        Self::check_version(&env)?;
        Self::load_config(&env, &severity)
    }

    // -------------------------------------------------------------------
    // #93 – Custom severity-level support (admin only)
    // -------------------------------------------------------------------

    /// Registers or updates a custom (non-canonical) severity level with its
    /// own SLAConfig. Stored in a separate map from the four canonical
    /// entries (critical/high/medium/low) so `get_config_snapshot()` and
    /// `compute_config_version_hash()` remain untouched. Admin only.
    pub fn set_custom_severity(
        env: Env,
        caller: Address,
        severity: Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        Self::require_not_frozen(&env)?;

        // Custom severities must never shadow a canonical one.
        if Self::is_canonical_severity(&severity) {
            return Err(SLAError::InvalidSeverity);
        }

        // Only the general bounds apply to custom severities — the
        // per-severity branches in validate_config are canonical-only.
        Self::validate_general_bounds(threshold_minutes, penalty_per_minute, reward_base)?;

        let mut custom: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CUSTOM_CONFIG_KEY)
            .unwrap_or_else(|| Map::new(&env));

        // #456 – Determine whether this is a first registration or a
        // reconfiguration so the emitted event distinguishes the two
        // lifecycle transitions. Indexers reconstructing the custom-severity
        // set from events need this to tell "who added" from "who changed".
        let is_update = custom.contains_key(severity.clone());

        custom.set(
            severity.clone(),
            SLAConfig {
                threshold_minutes,
                penalty_per_minute,
                reward_base,
            },
        );
        env.storage().instance().set(&CUSTOM_CONFIG_KEY, &custom);

        config_metadata::record_config_update(&env);
        // #408 – record the config snapshot under its new version hash.
        Self::record_config_registry(&env)?;

        // Emit the lifecycle-appropriate event: sev_add for first
        // registration, sev_upd for reconfiguration. The payload shape
        // is identical (threshold, penalty, reward) so consumers that only
        // care about values can parse either; consumers that need the
        // lifecycle distinction check topic[0].
        let event_name = if is_update {
            EVENT_SEV_UPD
        } else {
            EVENT_SEV_ADD
        };
        env.events().publish(
            (event_name, EVENT_VERSION, severity),
            (threshold_minutes, penalty_per_minute, reward_base),
        );
        Ok(())
    }

    /// Removes a previously registered custom severity level. Admin only.
    /// Returns `SeverityNotInSet` if the severity was never registered.
    pub fn remove_custom_severity(env: Env, caller: Address, severity: Symbol) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        Self::require_not_frozen(&env)?;

        let mut custom: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CUSTOM_CONFIG_KEY)
            .unwrap_or_else(|| Map::new(&env));

        if !custom.contains_key(severity.clone()) {
            return Err(SLAError::SeverityNotInSet);
        }

        custom.remove(severity.clone());
        env.storage().instance().set(&CUSTOM_CONFIG_KEY, &custom);

        env.events()
            .publish((EVENT_CONFIG_REM, EVENT_VERSION, severity), ());
        Ok(())
    }

    /// Returns the SLAConfig for a registered custom severity.
    /// Returns `SeverityNotInSet` if the severity was never registered.
    pub fn get_custom_severity(env: Env, severity: Symbol) -> Result<SLAConfig, SLAError> {
        Self::check_version(&env)?;
        let custom: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CUSTOM_CONFIG_KEY)
            .unwrap_or_else(|| Map::new(&env));
        custom.get(severity).ok_or(SLAError::SeverityNotInSet)
    }

    /// Returns a deterministic snapshot of all registered custom severity
    /// configurations, in insertion order. Mirrors the shape of
    /// `get_config_snapshot()` but is a distinct endpoint — the canonical
    /// snapshot is never mixed with custom entries. (#93)
    pub fn get_custom_config_snapshot(env: Env) -> Result<SLAConfigSnapshot, SLAError> {
        Self::check_version(&env)?;

        let custom: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CUSTOM_CONFIG_KEY)
            .unwrap_or_else(|| Map::new(&env));

        let mut entries = Vec::new(&env);
        for (severity, config) in custom.iter() {
            entries.push_back(SLAConfigEntry { severity, config });
        }

        Ok(SLAConfigSnapshot {
            version: CONFIG_SNAPSHOT_SCHEMA_VERSION,
            entries,
        })
    }

    /// Lists all configured severity-to-config mappings.
    pub fn list_configs(env: Env) -> Result<Map<Symbol, SLAConfig>, SLAError> {
        Self::check_version(&env)?;
        env.storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    /// #4 – Returns metadata about the most recent configuration update,
    /// or `None` if no `set_config` call has been recorded since
    /// initialization.
    ///
    /// Backend consumers compare `update.sequence` against the ledger
    /// sequence they observed at their last `get_config_snapshot()` to
    /// decide whether their cached configuration is stale and needs to be
    /// re-fetched. This enables cheap cache invalidation without polling the
    /// full configuration on every health check.
    ///
    /// The result is wrapped in `Option<ConfigUpdateInfo>` (rather than
    /// `Option<u32>`) so the `Some`/`None` distinction survives the
    /// Soroban contract client boundary.
    pub fn get_last_config_update(env: Env) -> Result<Option<ConfigUpdateInfo>, SLAError> {
        Self::check_version(&env)?;
        Ok(config_metadata::get_last_config_update(&env).map(|seq| ConfigUpdateInfo { sequence: seq }))
    }

    /// Returns a deterministic backend-friendly snapshot of all config values.
    pub fn get_config_snapshot(env: Env) -> Result<SLAConfigSnapshot, SLAError> {
        Self::check_version(&env)?;
        Self::build_config_snapshot(&env)
    }

    /// Returns the config snapshot recorded for a given version hash, if any. (#408)
    pub fn get_config_snapshot_by_version(
        env: Env,
        hash: u64,
    ) -> Result<Option<SLAConfigSnapshot>, SLAError> {
        Self::check_version(&env)?;
        let registry: Map<u64, SLAConfigSnapshot> = env
            .storage()
            .instance()
            .get(&CONFIG_REGISTRY_KEY)
            .unwrap_or_else(|| Map::new(&env));
        Ok(registry.get(hash))
    }

    /// Records the current config snapshot under the current version hash so
    /// historical configs remain recoverable for deterministic replay. (#408)
    fn record_config_registry(env: &Env) -> Result<(), SLAError> {
        let hash = Self::compute_config_version_hash(env)?;
        let snapshot = Self::build_config_snapshot(env)?;
        let mut registry: Map<u64, SLAConfigSnapshot> = env
            .storage()
            .instance()
            .get(&CONFIG_REGISTRY_KEY)
            .unwrap_or_else(|| Map::new(env));
        registry.set(hash, snapshot);
        env.storage().instance().set(&CONFIG_REGISTRY_KEY, &registry);
        Ok(())
    }

    /// Builds the canonical config snapshot (canonical severities only). (#408)
    fn build_config_snapshot(env: &Env) -> Result<SLAConfigSnapshot, SLAError> {
        let configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;

        let mut entries = Vec::new(env);
        for severity in Self::canonical_severities(env) {
            let config = configs.get(severity.clone()).ok_or(SLAError::ConfigNotFound)?;
            entries.push_back(SLAConfigEntry { severity, config });
        }

        Ok(SLAConfigSnapshot {
            version: CONFIG_SNAPSHOT_SCHEMA_VERSION,
            entries,
        })
    }

    /// Returns a deterministic config version hash for cheap config-change
    /// detection by backend sync logic.
    ///
    /// The hash uses a polynomial rolling hash with a prime base and modulus
    /// to provide strong collision resistance while remaining deterministic.
    /// It processes all severity config fields in canonical order
    /// (critical → high → medium → low) and is stable across repeated reads
    /// when config is unchanged.
    pub fn get_config_version_hash(env: Env) -> Result<u64, SLAError> {
        Self::check_version(&env)?;
        Self::compute_config_version_hash(&env)
    }

    /// SC-W5-046 – Returns the full catalogue of typed failure codes.
    ///
    /// Backend bridge consumers call this once at startup to pre-load all
    /// contract error codes and their human-readable labels. The schema is
    /// versioned ("v1") so backends can detect additions across upgrades.
    /// SC-W5-046 – Returns the full catalogue of typed failure codes.
    ///
    /// Backend bridge consumers call this once at startup to pre-load all
    /// contract error codes and their human-readable labels. The schema is
    /// versioned ("v1") so backends can detect additions across upgrades.
    pub fn get_failure_schema(env: Env) -> Result<FailureSchema, SLAError> {
        Self::check_version(&env)?;
        let mut codes = Vec::new(&env);

        // Emit in numeric order for deterministic consumption
        // All descriptions must be <= 32 bytes (Soroban Symbol constraint)
        let entries: [(u32, &str, &str); 22] = [
            (1, "AlreadyInitialized", "Contract already initialized"),
            (2, "NotInitialized", "Contract not yet initialized"),
            (3, "Unauthorized", "Caller lacks required role"),
            (4, "ConfigNotFound", "No config for severity"),
            (5, "VersionMismatch", "Storage version mismatch"),
            (6, "ContractPaused", "Contract is paused"),
            (7, "NoPendingTransfer", "No pending transfer"),
            (8, "InvalidThreshold", "Threshold out of range"),
            (9, "InvalidPenalty", "Penalty out of range"),
            (10, "InvalidReward", "Reward out of range"),
            (11, "InvalidSeverity", "Severity not supported"),
            (12, "RetentionLimitOutOfRange", "Retention limit out of range"),
            (13, "DuplicateOutageInput", "Conflicting duplicate outage_id"),
            (14, "InvalidPenaltyAmount", "Invalid penalty amount"),
            (15, "InvalidRewardAmount", "Invalid reward amount"),
            (16, "ConfigFrozen", "Configuration is frozen"),
            (17, "InvalidInput", "Invalid input parameter"),
            (18, "SeverityNotInSet", "Custom severity not registered"),
            (19, "OutageRecalcLimit", "Outage recalc limit reached"),
            (20, "ProposalExpired", "Proposal expired"),
            (21, "AdminRenounced", "Admin authority renounced"),
            (22, "ExposureOverflow", "Exposure totals overflow i128"),
        ];

        for (code, label, description) in entries {
            codes.push_back(FailureCode {
                code,
                label: Symbol::new(&env, label),
                description: Symbol::new(&env, description),
            });
        }

        Ok(FailureSchema {
            version: symbol_short!("v1"),
            codes,
        })
    }

    /// Returns the result schema descriptor for backend symbol mapping.
    pub fn get_result_schema(env: Env) -> Result<SLAResultSchema, SLAError> {
        Self::check_version(&env)?;
        Ok(SLAResultSchema {
            version: symbol_short!("v1"),
            schema_version: RESULT_SCHEMA_VERSION,
            result_field_count: RESULT_SCHEMA_FIELD_COUNT,
            status_met: symbol_short!("met"),
            status_violated: symbol_short!("viol"),
            payment_reward: symbol_short!("rew"),
            payment_penalty: symbol_short!("pen"),
            rating_exceptional: symbol_short!("top"),
            rating_excellent: symbol_short!("excel"),
            rating_good: symbol_short!("good"),
            rating_poor: symbol_short!("poor"),
            includes_config_version_hash: true,
            deprecated_symbols: Vec::new(&env),
            severity_aliases: Vec::new(&env),
        })
    }

    /// #1 – Combined configuration snapshot and result schema for one-shot
    /// backend bootstrap reads.
    ///
    /// Returns the result of composing `get_config_snapshot()` with
    /// `get_result_schema()` into a single [`ConfigBundle`] so consumers
    /// can populate their config cache and symbol map in a single RPC.
    /// `check_version()` is enforced by the two delegated methods, so a
    /// pre-migration contract transparently reports its migration error.
    ///
    /// The auto-generated client unwraps the contract method's
    /// `Result<Option<T>, SLAError>` envelope, surfacing
    /// `Option<ConfigBundle>` here. `Some(_)` is returned once the
    /// contract is initialised and on the current storage version.
    pub fn get_config_bundle(env: Env) -> Result<Option<ConfigBundle>, SLAError> {
        let snapshot = Self::get_config_snapshot(env.clone())?;
        let schema = Self::get_result_schema(env.clone())?;
        let config_version_hash = Self::compute_config_version_hash(&env)?;
        Ok(Some(ConfigBundle {
            snapshot,
            schema,
            config_version_hash,
        }))
    }

    /// Returns the full audit state including roles, config, stats, and history.
    ///
    /// Performs a single version check and direct single-pass storage key reads
    /// (admin, operator, pending slots, pause state/info, configs, stats, history len)
    /// to eliminate redundant delegated version checks and storage key deserializations.
    pub fn get_full_audit_state(env: Env) -> Result<AuditState, SLAError> {
        Self::check_version(&env)?;

        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(SLAError::NotInitialized)?;
        let operator: Address = env
            .storage()
            .instance()
            .get(&OPERATOR_KEY)
            .ok_or(SLAError::NotInitialized)?;
        let pending_admin: Option<Address> = env.storage().instance().get(&PENDING_ADMIN_KEY);
        let pending_operator: Option<Address> = env.storage().instance().get(&PENDING_OP_KEY);
        let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
        let pause_info: Option<PauseInfo> = env.storage().instance().get(&PAUSE_INFO_KEY);
        let config_snapshot = Self::build_config_snapshot(&env)?;
        let stats: SLAStats = env
            .storage()
            .instance()
            .get(&STATS_KEY)
            .ok_or(SLAError::NotInitialized)?;

        // Issue #463: use cached history length instead of materializing full vector
        let history_len: u32 = env.storage().instance().get(&HISTORY_LEN_KEY).unwrap_or(0);

        let result_schema = SLAResultSchema {
            version: symbol_short!("v1"),
            schema_version: RESULT_SCHEMA_VERSION,
            result_field_count: RESULT_SCHEMA_FIELD_COUNT,
            status_met: symbol_short!("met"),
            status_violated: symbol_short!("viol"),
            payment_reward: symbol_short!("rew"),
            payment_penalty: symbol_short!("pen"),
            rating_exceptional: symbol_short!("top"),
            rating_excellent: symbol_short!("excel"),
            rating_good: symbol_short!("good"),
            rating_poor: symbol_short!("poor"),
            includes_config_version_hash: true,
            deprecated_symbols: Vec::new(&env),
            severity_aliases: Vec::new(&env),
        };

        Ok(AuditState {
            admin,
            operator,
            pending_admin,
            pending_operator,
            paused,
            // Empty when unpaused, single-element when paused: `Option<PauseInfo>`
            // cannot be a `#[contracttype]` field (the SDK's ScVal conversion
            // needs `From<&PauseInfo>`, which `#[contracttype]` does not derive).
            pause_info: match pause_info {
                Some(info) => soroban_sdk::vec![&env, info],
                None => Vec::new(&env),
            },
            config_snapshot,
            stats,
            history_len,
            result_schema,
        })
    }

    /// #60 – Returns static contract capabilities for backend introspection.
    pub fn get_contract_metadata(env: Env) -> Result<ContractMetadata, SLAError> {
        Self::check_version(&env)?;
        let severities = Self::canonical_severities(&env);

        // #424 – feature list derived from the single shared source so this
        // legacy endpoint can never disagree with get_contract_info.
        let mut features = Vec::new(&env);
        for f in crate::contract_info::CONTRACT_FEATURES.iter() {
            features.push_back(Symbol::new(&env, f));
        }

        Ok(ContractMetadata {
            contract_name: symbol_short!("sla_calc"),
            storage_version: STORAGE_VERSION,
            result_schema_version: RESULT_SCHEMA_VERSION,
            supported_severities: severities,
            features,
        })
    }

    /// #191 – Returns the comprehensive, versioned contract-info object.
    ///
    /// This is the recommended startup handshake for backend consumers.
    /// It supersedes `get_contract_metadata()` (#60) and
    /// `get_version_info()` (SC-W5-029) by combining all identity,
    /// version posture, feature set, and operational status into a single
    /// typed, versioned response.
    pub fn get_contract_info(env: Env) -> Result<contract_info::ContractInfo, SLAError> {
        contract_info::get_contract_info(&env)
    }

    // -------------------------------------------------------------------
    // #244 – Public API descriptor
    // -------------------------------------------------------------------

    /// Returns a stable, typed descriptor of the contract's public API surface.
    ///
    /// Backend and frontend consumers call this once at startup to
    /// programmatically discover all public methods, their mutability, auth
    /// requirements, and emitted events — without hard-coding method names.
    ///
    /// # Use Cases
    ///
    /// - **Backend startup validation:** verify the deployed contract exposes
    ///   the expected set of methods before issuing operational transactions.
    /// - **Frontend discovery:** dynamically build UI elements (admin panels,
    ///   operator dashboards) based on the contract's actual API surface.
    /// - **Integration testing:** compare the descriptor against a known-good
    ///   snapshot to detect unintended API changes across upgrades.
    /// - **SDK generation:** generate client-side bindings programmatically.
    ///
    /// # Returns
    ///
    /// A `PublicApiDescriptor` with:
    /// - `version`: descriptor schema version ("v1")
    /// - `contract_name`: fixed to "sla_calc"
    /// - `methods`: list of `PublicApiMethod` entries, one per public function,
    ///   sorted alphabetically by method name
    ///
    /// Each `PublicApiMethod` contains:
    /// - `name`: the contract method name (e.g. "calculate_sla")
    /// - `mutates`: `true` if the method modifies storage
    /// - `auth`: auth classification — `"admin"`, `"operator"`, `"addr"`
    ///   (a specific pending address, e.g. `accept_admin`/`accept_operator`),
    ///   or `"none"`.
    /// - `event`: the primary event name emitted, or empty if none
    ///
    /// # Errors
    ///
    /// Returns `NotInitialized` if the contract has never been initialized.
    ///
    /// # Design
    ///
    /// This function intentionally bypasses `check_version` so it remains
    /// callable even when the contract is in a pre-migration state — backends
    /// must be able to discover the API before deciding whether to migrate.
    pub fn get_public_api(env: Env) -> Result<PublicApiDescriptor, SLAError> {
        // Require the contract to be initialized (must have a storage version)
        let _stored: u32 = env
            .storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)?;

        let mut methods = Vec::new(&env);

        // Helper to create a PublicApiMethod entry
        let method = |name: &str, mutates: bool, auth: &str, event: &str| -> PublicApiMethod {
            PublicApiMethod {
                name: Symbol::new(&env, name),
                mutates,
                auth: Symbol::new(&env, auth),
                event: if event.is_empty() {
                    Symbol::new(&env, "")
                } else {
                    Symbol::new(&env, event)
                },
            }
        };

        // All public methods added in alphabetical order for deterministic output.
        // Lifecycle:
        // Note: accept_admin/accept_operator are called by the proposed address
        // (not the current role holder). They still call `caller.require_auth()`
        // and enforce that the caller equals the pending address, so they are
        // NOT "none" — they are address-scoped ("addr"— the pending slot holder
        // must sign) (#426).
        methods.push_back(method("accept_admin", true, "addr", "adm_acc"));
        methods.push_back(method("accept_operator", true, "addr", "op_acc"));
        // Calculation:
        methods.push_back(method("calculate_sla", true, "operator", "sla_calc"));
        methods.push_back(method("calculate_sla_view", false, "none", ""));
        methods.push_back(method("cancel_admin_proposal", true, "admin", "adm_can"));
        methods.push_back(method("cancel_operator_proposal", true, "admin", "op_can"));
        // Config:
        methods.push_back(method("freeze_config", true, "admin", "cfg_frz"));
        methods.push_back(method("get_admin", false, "none", ""));
        methods.push_back(method("get_config", false, "none", ""));
        methods.push_back(method("get_config_bundle", false, "none", ""));
        methods.push_back(method("get_config_count", false, "none", ""));
        methods.push_back(method("get_config_snapshot", false, "none", ""));
        methods.push_back(method("get_config_version_hash", false, "none", ""));
        methods.push_back(method("get_contract_info", false, "none", ""));
        methods.push_back(method("get_contract_metadata", false, "none", ""));
        methods.push_back(method("get_contract_state_fingerprint", false, "none", ""));
        methods.push_back(method("get_custom_config_snapshot", false, "none", ""));
        methods.push_back(method("get_custom_severity", false, "none", ""));
        methods.push_back(method("get_economic_exposure", false, "none", ""));
        methods.push_back(method("get_failure_schema", false, "none", ""));
        methods.push_back(method("get_full_audit_state", false, "none", ""));
        methods.push_back(method("get_history", false, "none", ""));
        methods.push_back(method("get_history_by_outage", false, "none", ""));
        methods.push_back(method("get_history_page", false, "none", ""));
        methods.push_back(method("get_history_page_with_meta", false, "none", ""));
        methods.push_back(method("get_latest_by_outage", false, "none", ""));
        methods.push_back(method("get_last_config_update", false, "none", ""));
        methods.push_back(method("get_migration_state", false, "none", ""));
        methods.push_back(method("get_operator", false, "none", ""));
        methods.push_back(method("get_pause_info", false, "none", ""));
        methods.push_back(method("get_pending_admin", false, "none", ""));
        methods.push_back(method("get_pending_operator", false, "none", ""));
        methods.push_back(method("get_public_api", false, "none", ""));
        methods.push_back(method("get_rent_estimate", false, "none", ""));
        methods.push_back(method("get_result_schema", false, "none", ""));
        methods.push_back(method("get_retention_limit", false, "none", ""));
        methods.push_back(method("get_retention_metrics", false, "none", ""));
        methods.push_back(method("get_severity_telemetry", false, "none", ""));
        methods.push_back(method("get_stats", false, "none", ""));
        methods.push_back(method("get_storage_footprint_estimate", false, "none", ""));
        methods.push_back(method("get_storage_version", false, "none", ""));
        methods.push_back(method("get_version_info", false, "none", ""));
        methods.push_back(method("get_version_negotiation_info", false, "none", ""));
        // Health:
        methods.push_back(method("healthcheck", false, "none", ""));
        // Init:
        // initialize requires BOTH the admin and the operator address to
        // authorize (admin.require_auth(); operator.require_auth();) — not
        // single-party "admin" (#425).
        methods.push_back(method("initialize", true, "multi", ""));
        methods.push_back(method("is_config_frozen", false, "none", ""));
        methods.push_back(method("is_paused", false, "none", ""));
        // Config queries:
        methods.push_back(method("list_configs", false, "none", ""));
        // Migration:
        methods.push_back(method("migrate", true, "admin", "migrate_done"));
        // Pause:
        methods.push_back(method("pause", true, "admin", "paused"));
        methods.push_back(method("propose_admin", true, "admin", "adm_prop"));
        methods.push_back(method("propose_operator", true, "admin", "op_prop"));
        methods.push_back(method("prune_history", true, "admin", "pruned"));
        methods.push_back(method("prune_history_by_age", true, "admin", "pruned_a"));
        methods.push_back(method("remove_custom_severity", true, "admin", "cfg_rem"));
        methods.push_back(method("renounce_admin", true, "admin", "adm_ren"));
        methods.push_back(method("replay_calculate_sla", true, "operator", "sla_calc"));
        // Setters:
        methods.push_back(method("set_config", true, "admin", "cfg_upd"));
        methods.push_back(method("set_custom_severity", true, "admin", "sev_add"));
        methods.push_back(method("set_operator", true, "admin", "op_set"));
        methods.push_back(method("set_retention_limit", true, "admin", ""));
        methods.push_back(method("unfreeze_config", true, "admin", "cfg_unfrz"));
        methods.push_back(method("unpause", true, "admin", "unpause"));

        Ok(PublicApiDescriptor {
            version: symbol_short!("v1"),
            contract_name: symbol_short!("sla_calc"),
            methods,
        })
    }

    // -------------------------------------------------------------------
    // #29 – Stats view
    // -------------------------------------------------------------------

    /// Returns the cumulative SLA performance statistics.
    ///
    /// This is the contract's dashboard aggregate: together with
    /// `get_severity_telemetry` (per-severity weekly windows) it is the
    /// supported surface for dashboard telemetry. Cumulative totals here
    /// subsume the windowed view; consumers that need a windowed summary
    /// should read `get_severity_telemetry` rather than re-scan history.
    pub fn get_stats(env: Env) -> Result<SLAStats, SLAError> {
        Self::check_version(&env)?;
        env.storage()
            .instance()
            .get(&STATS_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    // -------------------------------------------------------------------
    // #96 – Economic exposure view for backend dashboarding
    // -------------------------------------------------------------------

    /// Returns the maximum potential reward and per-minute penalty rate for
    /// every configured severity, plus aggregate totals.
    ///
    /// This is a read-only view — it does **not** require auth, does not
    /// mutate state, and does not emit events. It can be called while the
    /// contract is paused because it only reads configuration data.
    ///
    /// The top-tier reward multiplier (200 %) is applied to `reward_base` to
    /// yield `max_reward` — matching the `compute_result` path for
    /// `performance_ratio < 50`.
    ///
    /// # Errors
    ///
    /// Returns [`SLAError::ExposureOverflow`] if any aggregate total
    /// overflows `i128` during summation. With the current validation bounds
    /// (`reward_base ≤ 100 000`, `penalty_per_minute ≤ 10 000`, 4 canonical
    /// severities) the maximum totals are `800 000` and `40 000` respectively,
    /// so this error is unreachable today. Checked arithmetic is used
    /// regardless for correctness-by-construction: a future bound relaxation
    /// or custom-severity expansion will produce a clear error instead of
    /// silently capping totals at `i128::MAX`. (SC-W5-047)
    pub fn get_economic_exposure(env: Env) -> Result<EconomicExposure, SLAError> {
        Self::check_version(&env)?;

        let mut breakdown = Vec::new(&env);
        let mut total_max_reward: i128 = 0;
        let mut total_penalty_per_minute: i128 = 0;

        for severity in Self::canonical_severities(&env) {
            let cfg = Self::load_config(&env, &severity)?;

            // Top-tier reward: performance_ratio < 50 → multiplier = 200 %
            // Mirrors compute_result exactly: reward_base * 200 / 100
            let max_reward = cfg
                .reward_base
                .checked_mul(200)
                .map(|v| v.div_euclid(100))
                .ok_or(SLAError::ExposureOverflow)?;

            let penalty_rate = cfg.penalty_per_minute;

            total_max_reward = total_max_reward
                .checked_add(max_reward)
                .ok_or(SLAError::ExposureOverflow)?;
            total_penalty_per_minute = total_penalty_per_minute
                .checked_add(penalty_rate)
                .ok_or(SLAError::ExposureOverflow)?;

            breakdown.push_back(SeverityExposure {
                severity,
                max_reward,
                penalty_per_minute: penalty_rate,
            });
        }

        Ok(EconomicExposure {
            total_max_reward,
            total_penalty_per_minute,
            breakdown,
        })
    }

    /// Per-severity counters are packed as four `u32` lanes inside one `u128`,
    /// one lane per canonical severity. Rust arrays are not valid Soroban
    /// storage values, and a `Vec<u32>` object costs materially more CPU to
    /// (de)serialise on every invocation — instance storage is read and written
    /// whole each call, so a scalar keeps unrelated operations inside budget.
    fn load_counts(env: &Env, key: &Symbol) -> u128 {
        env.storage().instance().get(key).unwrap_or(0u128)
    }

    /// Reads the counter lane for `index` (0..4).
    fn count_lane(packed: u128, index: u32) -> u32 {
        ((packed >> (index * 32)) & 0xFFFF_FFFF) as u32
    }

    /// Returns `packed` with the lane at `index` replaced by `value`.
    fn set_count_lane(packed: u128, index: u32, value: u32) -> u128 {
        let mask = !(0xFFFF_FFFFu128 << (index * 32));
        (packed & mask) | ((value as u128) << (index * 32))
    }

    /// #101 – Returns per-severity weekly violation-rate telemetry.
    pub fn get_severity_telemetry(env: Env) -> Result<Vec<SeverityTelemetry>, SLAError> {
        Self::check_version(&env)?;
        let mut telemetry = Vec::new(&env);
        let severities = Self::canonical_severities(&env);
        let calculations = Self::load_counts(&env, &SEVERITY_CALC_COUNTS_KEY);
        let violations = Self::load_counts(&env, &SEVERITY_VIOL_COUNTS_KEY);

        for index in 0..severities.len() {
            let severity = severities.get(index).unwrap();
            let calc_count = Self::count_lane(calculations, index);
            let violation_count = Self::count_lane(violations, index);
            let violation_rate = if calc_count == 0 {
                0u32
            } else {
                (violation_count.saturating_mul(100) / calc_count).min(100)
            };
            telemetry.push_back(SeverityTelemetry {
                severity: severity.clone(),
                calculations: calc_count,
                violations: violation_count,
                violation_rate,
            });
        }

        let custom: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CUSTOM_CONFIG_KEY)
            .unwrap_or_else(|| Map::new(&env));
        for (severity, _) in custom.iter() {
            telemetry.push_back(SeverityTelemetry {
                severity,
                calculations: 0,
                violations: 0,
                violation_rate: 0,
            });
        }

        Ok(telemetry)
    }

    // -------------------------------------------------------------------
    // #31 - SLA Audit Mode (View-only calculation)
    // -------------------------------------------------------------------

    /// Recalculates SLA deterministically without mutating any state or emitting events.
    /// Can be called by anyone for verification and audit purposes.
    ///
    /// # Input constraints
    ///
    /// - `mttr_minutes` must be ≤ 525,600 (365 days). Values exceeding this bound are rejected with `InvalidInput`.
    pub fn calculate_sla_view(
        env: Env,
        outage_id: Symbol,
        severity: Symbol,
        mttr_minutes: u32,
    ) -> Result<SLAResult, SLAError> {
        Self::check_version(&env)?;
        // We bypass pause and operator checks to allow continuous, public verification
        let cfg = Self::load_config(&env, &severity)?;
        let config_version_hash = Self::compute_config_version_hash(&env)?;

        // Apply duplicate/replay policy read-only against recorded history
        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        let mut existing: Option<SLAResult> = None;
        for i in 0..history.len() {
            let entry = history.get(i).unwrap();
            if entry.outage_id == outage_id {
                existing = Some(entry);
            }
        }
        if let Some(prev) = existing {
            if prev.config_version_hash == config_version_hash {
                if prev.mttr_minutes != mttr_minutes || prev.threshold_minutes != cfg.threshold_minutes {
                    return Err(SLAError::DuplicateOutageInput);
                }
                return Ok(prev);
            }
        }

        Self::compute_result(
            outage_id,
            mttr_minutes,
            &cfg,
            config_version_hash,
            env.ledger().timestamp(),
        )
    }

    // -------------------------------------------------------------------
    // Replay SLA calculation (view)                                    #95
    // -------------------------------------------------------------------

    /// Deterministic replay view for backend reconciliation.
    ///
    /// Returns the same `(SLAResult, config_version_hash)` pair that the
    /// mutating `calculate_sla` path would have produced, without writing
    /// state or emitting events.
    ///
    /// NOTE: The contract does not currently store per-ledger config
    /// snapshots, so `recorded_at_ledger` is stored in the result for
    /// audit purposes but the current config is used for evaluation.
    /// Once per-ledger config snapshots are added, this function will
    /// look up the config active at `recorded_at_ledger`.
    pub fn replay_calculate_sla(
        env: Env,
        outage_id: Symbol,
        severity: Symbol,
        mttr_minutes: u32,
        recorded_at_ledger: u64,
    ) -> Result<(SLAResult, u64), SLAError> {
        Self::check_version(&env)?;
        let cfg = Self::load_config(&env, &severity)?;
        let config_version_hash = Self::compute_config_version_hash(&env)?;

        let result = Self::compute_result(
            outage_id,
            mttr_minutes,
            &cfg,
            config_version_hash,
            recorded_at_ledger,
        )?;
        Ok((result, config_version_hash))
    }

    // -------------------------------------------------------------------
    // SLA calculation (operator only)                                #28
    // -------------------------------------------------------------------

    /// Calculate the SLA outcome for an outage event.
    ///
    /// # Duplicate detection
    ///
    /// If `outage_id` was already submitted under the same config version hash,
    /// the call is **idempotent** when the inputs match exactly (returns the
    /// cached `SLAResult` without side-effects). When the inputs differ, the
    /// call returns [`SLAError::DuplicateOutageInput`]. See that error's
    /// documentation for the full duplicate-detection matrix.
    ///
    /// # Access control
    ///
    /// Only the current **operator** may call this function. The caller must
    /// pass `require_auth()` before invoking.
    ///
    /// # Errors
    ///
    /// | Error | Condition |
    /// |---|---|
    /// | `NotInitialized` | Contract has not been initialized |
    /// | `VersionMismatch` | Storage version does not match binary expectation |
    /// | `ContractPaused` | Contract is currently paused |
    /// | `Unauthorized` | Caller is not the operator |
    /// | `ConfigNotFound` | No configuration exists for the requested severity |
    /// | `DuplicateOutageInput` | Same `outage_id` submitted with conflicting inputs; emits a `dup_input` event carrying the stored result |
    /// | `InvalidInput` | Input parameter violates documented constraints (e.g., mttr_minutes exceeds maximum allowed) |
    /// | `InvalidPenaltyAmount` | Penalty computation overflowed or produced a non-negative value |
    /// | `InvalidRewardAmount` | Reward computation overflowed or produced a non-positive value |
    /// Records an SLA decision for `outage_id`. Operator only.
    ///
    /// # Input constraints
    ///
    /// - `mttr_minutes` must be ≤ 525,600 (365 days). Values exceeding this bound are rejected with `InvalidInput`.
    ///
    /// # Repeated submissions for the same outage_id
    ///
    /// Anti-spam policy, applied in this order:
    ///
    /// 1. **Replay** — an unchanged config hash with identical inputs returns the
    ///    stored result and writes nothing at all: no history entry, no stats, no
    ///    telemetry, no events. Retrying a call whose response was lost is
    ///    therefore free of state drift, however many times it is repeated.
    /// 2. **Conflict** — an unchanged config hash with a different MTTR or
    ///    threshold is rejected with `DuplicateOutageInput`, so a stored decision
    ///    can never be silently restated. The rejection is accompanied by a
    ///    `dup_input` event carrying the stored result, so consumers need no
    ///    follow-up `get_latest_by_outage` read.
    /// 3. **Recalculation** — a changed config hash opens a new generation for the
    ///    outage, capped at `MAX_RECALCS_PER_OUTAGE` retained entries. Beyond that
    ///    the call is rejected with `OutageRecalcLimit`, bounding how much of the
    ///    retained window one outage can occupy. Admin pruning frees headroom.
    ///
    /// Telemetry is recorded only once the calculation is certain to be stored, so
    /// neither a replay nor a rejected submission can inflate the per-severity
    /// counters behind `get_severity_telemetry`.
    pub fn calculate_sla(
        env: Env,
        caller: Address, // #28 – operator must identify themselves
        outage_id: Symbol,
        severity: Symbol,
        mttr_minutes: u32,
    ) -> Result<SLAResult, SLAError> {
        Self::check_version(&env)?;
        Self::require_not_paused(&env)?; // #27
        Self::require_operator(&env, &caller)?; // #28

        let cfg = Self::load_config(&env, &severity)?;
        let config_version_hash = Self::compute_config_version_hash(&env)?;
        let result = Self::compute_result(
            outage_id.clone(),
            mttr_minutes,
            &cfg,
            config_version_hash,
            env.ledger().timestamp(),
        )?;
        let mut history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        // The scan that finds the newest entry for this outage also counts how
        // many retained entries the outage already owns (anti-spam accounting).
        let mut existing: Option<SLAResult> = None;
        let mut stored_for_outage: u32 = 0;
        for i in 0..history.len() {
            let entry = history.get(i).unwrap();
            if entry.outage_id == outage_id {
                stored_for_outage += 1;
                existing = Some(entry);
            }
        }
        if let Some(prev) = existing {
            if prev.config_version_hash == config_version_hash {
                // Explicit duplicate policy: same outage_id is idempotent only when
                // execution inputs resolve to the same deterministic result.
                if prev.mttr_minutes != mttr_minutes || prev.threshold_minutes != cfg.threshold_minutes {
                    // #385 – publish the stored result alongside the rejection so
                    // consumers can reconcile the conflict from this transaction's
                    // event log without a second get_latest_by_outage read.
                    Self::publish_duplicate_input_event(&env, severity.clone(), &prev);
                    return Err(SLAError::DuplicateOutageInput);
                }
                // Replay: return the stored decision without touching state.
                return Ok(prev);
            }
            // Config changed: treat as a fresh calculation rather than a conflict,
            // but never let one outage grow past the anti-spam cap.
            if stored_for_outage >= MAX_RECALCS_PER_OUTAGE {
                return Err(SLAError::OutageRecalcLimit);
            }
        }

        // Past this point the result is guaranteed to be stored, so telemetry can
        // no longer be inflated by replays or by rejected submissions.
        let met = result.status != symbol_short!("viol");
        Self::record_severity_telemetry(&env, &severity, met);

        history.push_back(result.clone());

        // SC-013: use configurable retention limit (falls back to MAX_HISTORY_SIZE)
        let retention_limit: u32 = env
            .storage()
            .instance()
            .get(&RETENTION_LIMIT_KEY)
            .unwrap_or(MAX_HISTORY_SIZE);

        // SC-062: enforce bounded retention – drop oldest entry when cap is exceeded
        if history.len() > retention_limit {
            let mut trimmed = Vec::new(&env);
            for i in 1..history.len() {
                trimmed.push_back(history.get(i).unwrap());
            }
            Self::update_history_and_cache(&env, &trimmed);
        } else {
            Self::update_history_and_cache(&env, &history);
        }

        // Mutate stats and emit events depending on outcome
        if result.status == symbol_short!("viol") {
            // #29 – update stats (pass positive penalty value)
            Self::increment_stats(&env, false, 0, -result.amount);
        } else {
            // #29 – update stats
            Self::increment_stats(&env, true, result.amount, 0);
        }

        Self::publish_sla_event(&env, severity.clone(), &result);
        Self::publish_settlement_intent_event(&env, severity, &result);

        Ok(result)
    }

    // -------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------

    /// Pure helper to generate the SLAResult deterministically.
    /// `config_version_hash` binds the result to the exact config snapshot used
    /// during evaluation. `recorded_at` is the ledger timestamp at call time
    /// for both mutating and view paths (see issue #465 for audit-mode semantics).
    ///
    /// # Timestamp Semantics (Issue #465)
    ///
    /// - Mutating path (`calculate_sla`): `recorded_at` is the current ledger
    ///   timestamp; the result is stored to history.
    /// - View path (`calculate_sla_view`): `recorded_at` is the current ledger
    ///   timestamp to ensure view results match the mutating path when executed
    ///   in the same ledger; the result is NOT stored.
    /// - Replay path (`replay_calculate_sla`): Uses a provided historical
    ///   timestamp for audit purposes.
    fn compute_result(
        outage_id: Symbol,
        mttr_minutes: u32,
        cfg: &SLAConfig,
        config_version_hash: u64,
        recorded_at: u64,
    ) -> Result<SLAResult, SLAError> {
        let threshold = cfg.threshold_minutes;

        // Case 1: SLA violated → penalty
        if mttr_minutes > threshold {
            let overtime = (mttr_minutes - threshold) as i128;
            // Use checked_mul so an overflowing penalty surfaces a deterministic
            // error instead of silently saturating (which would under-penalise).
            let penalty = match overtime.checked_mul(cfg.penalty_per_minute) {
                Some(val) => val,
                None => return Err(SLAError::InvalidPenaltyAmount),
            };
            let amount = match penalty.checked_neg() {
                Some(val) => val,
                None => return Err(SLAError::InvalidPenaltyAmount),
            };
            if amount >= 0 {
                return Err(SLAError::InvalidPenaltyAmount);
            }

            Ok(SLAResult {
                outage_id,
                status: symbol_short!("viol"),
                mttr_minutes,
                threshold_minutes: threshold,
                amount,
                payment_type: symbol_short!("pen"),
                rating: symbol_short!("poor"),
                config_version_hash,
                recorded_at,
            })
        } else {
            // Case 2: SLA met → reward
            let performance_ratio = (mttr_minutes as u64 * 100)
                .checked_div(threshold as u64)
                .unwrap_or(0);

            let (multiplier, rating) = if performance_ratio < 50 {
                (200u32, symbol_short!("top"))
            } else if performance_ratio < 75 {
                (150u32, symbol_short!("excel"))
            } else {
                (100u32, symbol_short!("good"))
            };

            // Use checked_mul so an overflowing reward surfaces a deterministic
            // error instead of silently saturating.
            let reward = match cfg.reward_base.checked_mul(multiplier as i128) {
                Some(val) => val.div_euclid(100),
                None => return Err(SLAError::InvalidRewardAmount),
            };
            if reward <= 0 {
                return Err(SLAError::InvalidRewardAmount);
            }

            Ok(SLAResult {
                outage_id,
                status: symbol_short!("met"),
                mttr_minutes,
                threshold_minutes: threshold,
                amount: reward,
                payment_type: symbol_short!("rew"),
                rating,
                config_version_hash,
                recorded_at,
            })
        }
    }

    fn write_version(env: &Env) {
        env.storage()
            .instance()
            .set(&STORAGE_VERSION_KEY, &STORAGE_VERSION);
    }

    pub(crate) fn check_version(env: &Env) -> Result<(), SLAError> {
        let v: u32 = env
            .storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if v != STORAGE_VERSION {
            return Err(SLAError::VersionMismatch);
        }
        Ok(())
    }

    pub(crate) fn require_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
        caller.require_auth();
        // #406 – distinguish a permanent admin renounce from a fresh contract.
        if env.storage().instance().has(&ADMIN_RENOUNCED_KEY) {
            return Err(SLAError::AdminRenounced);
        }
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if caller != &admin {
            return Err(SLAError::Unauthorized);
        }
        Ok(())
    }

    /// #28 – Ensures the caller holds the operator role.
    pub(crate) fn require_operator(env: &Env, caller: &Address) -> Result<(), SLAError> {
        caller.require_auth();
        let operator: Address = env
            .storage()
            .instance()
            .get(&OPERATOR_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if caller != &operator {
            return Err(SLAError::Unauthorized);
        }
        Ok(())
    }

    fn require_not_paused(env: &Env) -> Result<(), SLAError> {
        let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
        if paused {
            return Err(SLAError::ContractPaused);
        }
        Ok(())
    }

    fn require_not_frozen(env: &Env) -> Result<(), SLAError> {
        if config_freeze::is_config_frozen(env) {
            return Err(SLAError::ConfigFrozen);
        }
        Ok(())
    }

    /// #93 – General bounds shared by canonical and custom severities.
    /// Extracted from validate_config so custom severities get the same
    /// baseline safety checks without the canonical-only per-severity branches.
    pub(crate) fn validate_general_bounds(
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) -> Result<(), SLAError> {
        if threshold_minutes == 0 || threshold_minutes > 1440 {
            return Err(SLAError::InvalidThreshold);
        }
        if penalty_per_minute <= 0 || penalty_per_minute > 10000 {
            return Err(SLAError::InvalidPenalty);
        }
        if reward_base <= 0 || reward_base > 100000 {
            return Err(SLAError::InvalidReward);
        }
        Ok(())
    }

    /// #70 – Validates configuration parameters to ensure safe and meaningful values.
    ///
    /// Delegates the shared baseline checks (range, positivity, cross-parameter
    /// consistency) to [`validate_general_bounds`] so canonical and custom
    /// severity paths share a single source of truth.
    pub(crate) fn validate_config(
        severity: &Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) -> Result<(), SLAError> {
        // Validate severity is one of the canonical values
        if !Self::is_canonical_severity(severity) {
            return Err(SLAError::InvalidSeverity);
        }

        // Shared baseline: range checks + cross-parameter consistency
        Self::validate_general_bounds(threshold_minutes, penalty_per_minute, reward_base)?;

        // Severity-specific validation to ensure logical consistency
        if *severity == symbol_short!("critical") {
            // Critical should have shortest thresholds and highest penalties
            if threshold_minutes > 60 {
                return Err(SLAError::InvalidThreshold);
            }
            if penalty_per_minute < 50 {
                return Err(SLAError::InvalidPenalty);
            }
        } else if *severity == symbol_short!("high") {
            // High severity thresholds should be reasonable
            if threshold_minutes > 120 {
                return Err(SLAError::InvalidThreshold);
            }
            if penalty_per_minute < 25 {
                return Err(SLAError::InvalidPenalty);
            }
        } else if *severity == symbol_short!("medium") {
            // Medium severity thresholds
            if threshold_minutes > 240 {
                return Err(SLAError::InvalidThreshold);
            }
            if penalty_per_minute < 10 {
                return Err(SLAError::InvalidPenalty);
            }
        } else if *severity == symbol_short!("low") {
            // Low severity can have longer thresholds but lower penalties
            if penalty_per_minute > 100 {
                return Err(SLAError::InvalidPenalty);
            }
        }

        // Cross-parameter consistency: rewards must materially exceed penalties.
        // penalty_per_minute * 1.5 < reward_base  →  penalty * 3 < reward_base * 2
        if penalty_per_minute.checked_mul(3).ok_or(SLAError::InvalidReward)?
            >= reward_base.checked_mul(2).ok_or(SLAError::InvalidReward)?
        {
            return Err(SLAError::InvalidReward);
        }

        Ok(())
    }

    pub(crate) fn canonical_severities(env: &Env) -> Vec<Symbol> {
        let mut severities = Vec::new(env);
        severities.push_back(symbol_short!("critical"));
        severities.push_back(symbol_short!("high"));
        severities.push_back(symbol_short!("medium"));
        severities.push_back(symbol_short!("low"));
        severities
    }

    pub(crate) fn canonical_severity_index(severity: &Symbol) -> Option<u32> {
        if *severity == symbol_short!("critical") {
            Some(0)
        } else if *severity == symbol_short!("high") {
            Some(1)
        } else if *severity == symbol_short!("medium") {
            Some(2)
        } else if *severity == symbol_short!("low") {
            Some(3)
        } else {
            None
        }
    }

    pub(crate) fn is_canonical_severity(severity: &Symbol) -> bool {
        Self::canonical_severity_index(severity).is_some()
    }

    /// #92 – Cross-severity penalty ordering validation.
    ///
    /// Ensures that higher-severity thresholds have an equal or greater
    /// penalty_per_minute than lower-severity ones for the canonical higher
    /// tiers (critical ≥ high ≥ medium). The low severity is exempt from
    /// the upper-direction check because its per-severity cap (100) can
    /// exceed medium's minimum (10) by design.
    ///
    /// The rule is: for critical, high, and medium, we enforce
    /// `higher.penalty >= lower.penalty`. This prevents accidental
    /// inversion where a moderate severity penalises more heavily than a
    /// higher one (e.g. medium.penalty > critical.penalty).
    ///
    /// Checks performed:
    ///   - critical:  new_penalty >= existing_high.penalty
    ///   - high:      new_penalty <= existing_critical.penalty AND
    ///                new_penalty >= existing_medium.penalty
    ///   - medium:    new_penalty <= existing_high.penalty
    ///   - low:       no cross-severity check (capped independently at 100)
    pub(crate) fn validate_cross_severity_penalty_ordering(
        env: &Env,
        updated_severity: &Symbol,
        new_penalty: i128,
    ) -> Result<(), SLAError> {
        let configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;

        let index = Self::canonical_severity_index(updated_severity).ok_or(SLAError::InvalidSeverity)?;
        let severities = Self::canonical_severities(env);

        // Check against the next-lower severity (if any):
        //   this severity's penalty >= next-lower severity's penalty
        //
        // `canonical_severities` always returns exactly 4 entries, so
        // `index + 1` is within bounds whenever the condition holds. The
        // `ok_or` guard makes that assumption explicit and converts an
        // unexpected out-of-bounds condition into a deterministic error
        // rather than a panic.
        if index + 1 < severities.len() {
            let lower_sev = severities.get(index + 1).ok_or(SLAError::InvalidSeverity)?;
            if let Some(lower_cfg) = configs.get(lower_sev.clone()) {
                if new_penalty < lower_cfg.penalty_per_minute {
                    return Err(SLAError::InvalidPenalty);
                }
            }
        }

        // Check against the next-higher severity (if any) — but only for
        // the three higher tiers. Low (index 3) is exempt from this check
        // because its per-severity cap (100) intentionally exceeds medium's
        // minimum (10), making a strict `low <= medium` rule impossible.
        //
        // Same defensive `ok_or` pattern: the bounds check above guarantees
        // `index - 1` is valid for index in 1..=2, but the explicit error
        // conversion prevents a silent panic if that invariant is ever broken.
        if (1..=2).contains(&index) {
            let higher_sev = severities.get(index - 1).ok_or(SLAError::InvalidSeverity)?;
            if let Some(higher_cfg) = configs.get(higher_sev.clone()) {
                if new_penalty > higher_cfg.penalty_per_minute {
                    return Err(SLAError::InvalidPenalty);
                }
            }
        }

        Ok(())
    }

    /// Cross-severity threshold ordering validation.
    ///
    /// Ensures that more-severe tiers have shorter thresholds than less-severe
    /// ones, preserving the documented progression:
    ///   critical.threshold <= high.threshold <= medium.threshold <= low.threshold
    ///
    /// This prevents configurations where a low-severity outage would violate
    /// faster than a critical-severity one (e.g. low threshold < critical threshold),
    /// which would invert the severity model's meaning for operators.
    ///
    /// The check is symmetrical to `validate_cross_severity_penalty_ordering`:
    /// it compares the new threshold against adjacent canonical severities.
    pub(crate) fn validate_cross_severity_threshold_ordering(
        env: &Env,
        updated_severity: &Symbol,
        new_threshold: u32,
    ) -> Result<(), SLAError> {
        let configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;

        let index = Self::canonical_severity_index(updated_severity).ok_or(SLAError::InvalidSeverity)?;
        let severities = Self::canonical_severities(env);

        // Check against the next-lower severity (if any):
        //   this severity's threshold <= next-lower severity's threshold
        // (critical <= high <= medium <= low)
        if index + 1 < severities.len() {
            let lower_sev = severities.get(index + 1).ok_or(SLAError::InvalidSeverity)?;
            if let Some(lower_cfg) = configs.get(lower_sev.clone()) {
                if new_threshold > lower_cfg.threshold_minutes {
                    return Err(SLAError::InvalidThreshold);
                }
            }
        }

        // Check against the next-higher severity (if any):
        //   this severity's threshold >= next-higher severity's threshold
        // Only enforced for high, medium, low (index 1..=3).
        if (1..=3).contains(&index) {
            let higher_sev = severities.get(index - 1).ok_or(SLAError::InvalidSeverity)?;
            if let Some(higher_cfg) = configs.get(higher_sev.clone()) {
                if new_threshold < higher_cfg.threshold_minutes {
                    return Err(SLAError::InvalidThreshold);
                }
            }
        }

        Ok(())
    }

    /// Shared config lookup that borrows env (avoids consuming it).
    pub(crate) fn compute_config_version_hash(env: &Env) -> Result<u64, SLAError> {
        let severities = [
            symbol_short!("critical"),
            symbol_short!("high"),
            symbol_short!("medium"),
            symbol_short!("low"),
        ];

        const BASE: u64 = 91138233;
        const MODULUS: u64 = (1u64 << 63) - 25;

        let mut hash: u64 = 1;
        let mut power: u64 = 1;

        for sev in severities {
            let cfg = Self::load_config(env, &sev)?;

            hash = hash
                .wrapping_mul(BASE)
                .wrapping_add(cfg.threshold_minutes as u64)
                .wrapping_mul(power)
                % MODULUS;
            power = power.wrapping_mul(BASE) % MODULUS;

            hash = hash
                .wrapping_mul(BASE)
                .wrapping_add(cfg.penalty_per_minute as u64)
                .wrapping_mul(power)
                % MODULUS;
            power = power.wrapping_mul(BASE) % MODULUS;

            hash = hash
                .wrapping_mul(BASE)
                .wrapping_add(cfg.reward_base as u64)
                .wrapping_mul(power)
                % MODULUS;
            power = power.wrapping_mul(BASE) % MODULUS;
        }

        Ok(hash.wrapping_mul(BASE).wrapping_add(0x9e3779b97f4a7c15u64) % MODULUS)
    }

    /// Config lookup used by calculate_sla / calculate_sla_view / get_config.
    /// Canonical severities are checked first (fast path, unchanged behaviour).
    /// Non-canonical severities fall back to the custom severity map (#93),
    /// so calculate_sla can evaluate outages against admin-registered custom
    /// severities the same way it does canonical ones.
    pub(crate) fn load_config(env: &Env, severity: &Symbol) -> Result<SLAConfig, SLAError> {
        if Self::is_canonical_severity(severity) {
            let configs: Map<Symbol, SLAConfig> = env
                .storage()
                .instance()
                .get(&CONFIG_KEY)
                .ok_or(SLAError::NotInitialized)?;
            return configs.get(severity.clone()).ok_or(SLAError::ConfigNotFound);
        }

        let custom: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CUSTOM_CONFIG_KEY)
            .unwrap_or_else(|| Map::new(env));
        custom.get(severity.clone()).ok_or(SLAError::ConfigNotFound)
    }

    /// #29 – Read-modify-write the stats entry.
    /// `met`     – true when SLA was met (reward path), false for violation.
    /// `reward`  – reward amount to add (0 on violation path).
    /// `penalty` – penalty amount to add, stored positive (0 on met path).
    fn increment_stats(env: &Env, met: bool, reward: i128, penalty: i128) {
        let mut stats: SLAStats = env.storage().instance().get(&STATS_KEY).unwrap_or(SLAStats {
            total_calculations: 0,
            total_violations: 0,
            total_rewards: 0,
            total_penalties: 0,
        });

        // Each counter uses checked_* so a saturating increment can be detected
        // and surfaced as a stats_sat event. On overflow the counter is capped
        // at its bound (preserving the previous fire-and-forget contract) but the
        // pre-cap state is emitted so backends know the total now under-reports.
        match stats.total_calculations.checked_add(1) {
            Some(v) => stats.total_calculations = v,
            None => {
                Self::emit_stats_saturated(
                    env,
                    symbol_short!("totcalc"),
                    stats.total_calculations as i128,
                    1,
                );
                stats.total_calculations = u64::MAX;
            }
        }

        if met {
            match stats.total_rewards.checked_add(reward) {
                Some(v) => stats.total_rewards = v,
                None => {
                    Self::emit_stats_saturated(env, symbol_short!("totrew"), stats.total_rewards, reward);
                    stats.total_rewards = if reward > 0 { i128::MAX } else { i128::MIN };
                }
            }
        } else {
            match stats.total_violations.checked_add(1) {
                Some(v) => stats.total_violations = v,
                None => {
                    Self::emit_stats_saturated(
                        env,
                        symbol_short!("totviol"),
                        stats.total_violations as i128,
                        1,
                    );
                    stats.total_violations = u64::MAX;
                }
            }
            match stats.total_penalties.checked_add(penalty) {
                Some(v) => stats.total_penalties = v,
                None => {
                    Self::emit_stats_saturated(env, symbol_short!("totpen"), stats.total_penalties, penalty);
                    stats.total_penalties = if penalty > 0 { i128::MAX } else { i128::MIN };
                }
            }
        }

        env.storage().instance().set(&STATS_KEY, &stats);
    }

    /// Emits a `stats_sat` event when a running-stats counter saturates.
    /// topic[0]=stats_sat, topic[1]=version, topic[2]=counter_name;
    /// payload=(field, previous_value, attempted_increment). See event_schema.rs.
    fn emit_stats_saturated(env: &Env, counter: Symbol, previous_value: i128, attempted_increment: i128) {
        env.events().publish(
            (EVENT_STATS_SAT, EVENT_VERSION, counter.clone()),
            (counter, previous_value, attempted_increment),
        );
    }

    /// Records per-severity calculation/violation counters for telemetry.
    ///
    /// Each severity lane is a `u32`. Counters are incremented with
    /// `saturating_add(1)`, so they saturate at `u32::MAX` instead of wrapping
    /// (release) or panicking (debug). A saturated lane is treated as "many"
    /// and is never reset to zero by overflow.
    fn record_severity_telemetry(env: &Env, severity: &Symbol, met: bool) {
        let index = Self::canonical_severity_index(severity).unwrap_or(0);
        let mut calculations = Self::load_counts(env, &SEVERITY_CALC_COUNTS_KEY);
        let mut violations = Self::load_counts(env, &SEVERITY_VIOL_COUNTS_KEY);
        let mut last_calculations = Self::load_counts(env, &LAST_CALCULATION_TS_KEY);
        let mut last_violations = Self::load_counts(env, &LAST_VIOLATION_TS_KEY);

        let now = env.ledger().timestamp();
        let week_seconds = 7u64 * 24u64 * 60u64 * 60u64;
        let last_calc = Self::count_lane(last_calculations, index) as u64;
        let last_violation = Self::count_lane(last_violations, index) as u64;
        let calc_stale = last_calc != 0 && now.saturating_sub(last_calc) >= week_seconds;
        let violation_stale = last_violation != 0 && now.saturating_sub(last_violation) >= week_seconds;
        if calc_stale {
            calculations = Self::set_count_lane(calculations, index, 0);
        }
        if violation_stale {
            violations = Self::set_count_lane(violations, index, 0);
        }

        calculations = Self::set_count_lane(
            calculations,
            index,
            Self::count_lane(calculations, index).saturating_add(1),
        );
        if !met {
            violations = Self::set_count_lane(
                violations,
                index,
                Self::count_lane(violations, index).saturating_add(1),
            );
        }

        let current_ts = if now > u64::from(u32::MAX) {
            u32::MAX
        } else {
            now as u32
        };
        last_calculations = Self::set_count_lane(last_calculations, index, current_ts);
        if !met {
            last_violations = Self::set_count_lane(last_violations, index, current_ts);
        }

        env.storage()
            .instance()
            .set(&SEVERITY_CALC_COUNTS_KEY, &calculations);
        env.storage()
            .instance()
            .set(&SEVERITY_VIOL_COUNTS_KEY, &violations);
        env.storage()
            .instance()
            .set(&LAST_CALCULATION_TS_KEY, &last_calculations);
        env.storage()
            .instance()
            .set(&LAST_VIOLATION_TS_KEY, &last_violations);
    }

    fn publish_sla_event(env: &Env, severity: Symbol, result: &SLAResult) {
        // Canonical decision field order (#429): shares the SLAResult struct
        // order with set_int and dup_input so indexers parse one layout.
        env.events().publish(
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

    fn publish_settlement_intent_event(env: &Env, severity: Symbol, result: &SLAResult) {
        // Canonical decision field order (#429) carrying the full decision
        // (mttr_minutes, threshold_minutes, rating) so a settlement-only
        // consumer can reconstruct the SLA decision (#428).
        env.events().publish(
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

    fn publish_duplicate_input_event(env: &Env, severity: Symbol, existing: &SLAResult) {
        env.events().publish(
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
            ),
        );
    }

    // -------------------------------------------------------------------
    // #33 - History & Compaction (Admin only)
    // -------------------------------------------------------------------

    /// Returns the raw log of recent SLA calculations stored on-chain.
    pub fn get_history(env: Env) -> Result<Vec<SLAResult>, SLAError> {
        Self::check_version(&env)?;
        Ok(env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env)))
    }

    /// Prunes the SLA calculation history to prevent indefinite storage growth.
    /// `keep_latest` dictates how many of the most recent records to retain.
    pub fn prune_history(env: Env, caller: Address, keep_latest: u32) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let len = history.len();

        let remove_count = if len > keep_latest {
            let remove_count = len - keep_latest;
            let mut new_history = Vec::new(&env);

            // Rebuild the vector keeping only the most recent entries
            for i in remove_count..len {
                new_history.push_back(history.get(i).unwrap());
            }

            Self::update_history_and_cache(&env, &new_history);
            remove_count
        } else {
            0
        };
        env.events()
            .publish((EVENT_PRUNED, EVENT_VERSION, caller), (remove_count, keep_latest));
        Ok(())
    }

    /// SC-063 – Prune history entries older than `min_age_seconds` before the
    /// current ledger timestamp. Admin-only. Emits a `pruned_a` event.
    ///
    /// # Timestamp Semantics (Issue #465)
    ///
    /// All stored results carry `recorded_at` = the ledger timestamp at calculation
    /// time. View-mode results (from `calculate_sla_view`) are never stored to history,
    /// so the empty-edge case of `recorded_at == 0` does not occur in practice.
    pub fn prune_history_by_age(env: Env, caller: Address, min_age_seconds: u64) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        let now = env.ledger().timestamp();
        if min_age_seconds >= now {
            return Err(SLAError::InvalidInput);
        }
        let cutoff = now.saturating_sub(min_age_seconds);

        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));

        let mut new_history = Vec::new(&env);
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
            Self::update_history_and_cache(&env, &new_history);
        }
        let kept = new_history.len();
        env.events()
            .publish((EVENT_PRUNED_AGE, EVENT_VERSION, caller), (removed, kept));

        Ok(())
    }

    // -------------------------------------------------------------------
    // SC-059: History pagination
    // -------------------------------------------------------------------

    /// Returns a bounded page of history entries.
    /// `offset` is zero-based; entries are ordered oldest-first (insertion order).
    /// Returns an empty Vec when `offset` is beyond the end of history.
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
    pub fn get_history_page(env: Env, offset: u32, limit: u32) -> Result<Vec<SLAResult>, SLAError> {
        Self::check_version(&env)?;
        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let (_end, page) = Self::compute_page_slice(&env, &history, offset, limit);
        Ok(page)
    }

    /// Returns a bounded page of history entries together with pagination
    /// metadata.
    ///
    /// This is a metadata-carrying companion to `get_history_page`. The
    /// `items` slice is identical to what `get_history_page` returns for the
    /// same `(offset, limit)`; `total` is the full history length and
    /// `has_more` is `true` when the requested range ends before the end of
    /// history **and** `limit > 0`. When `limit == 0`, `has_more` is `false` to
    /// signal end-of-history (empty page).
    ///
    /// Pagination semantics (offset-based, oldest-first, saturating
    /// `offset + limit`, empty page when `offset >= len` or `limit == 0`) are
    /// identical to `get_history_page` — see
    /// `docs/HISTORY_PAGINATION_POLICY.md`.
    pub fn get_history_page_with_meta(env: Env, offset: u32, limit: u32) -> Result<HistoryPage, SLAError> {
        Self::check_version(&env)?;
        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let total = history.len();
        let (end, items) = Self::compute_page_slice(&env, &history, offset, limit);
        Ok(HistoryPage {
            items,
            total,
            has_more,
        })
    }

    // -------------------------------------------------------------------
    // SC-060: History query by outage identifier
    // -------------------------------------------------------------------

    /// Returns all history entries whose `outage_id` matches the given value in
    /// chronological order (oldest-first).
    ///
    /// When an outage has multiple entries across config generations (up to
    /// `MAX_RECALCS_PER_OUTAGE`), each entry carries its `config_version_hash`
    /// so consumers can match records to specific config generations. The final
    /// entry in the returned array represents the latest decision.
    pub fn get_history_by_outage(env: Env, outage_id: Symbol) -> Result<Vec<SLAResult>, SLAError> {
        Self::check_version(&env)?;
        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let mut matches = Vec::new(&env);
        for i in 0..history.len() {
            let entry = history.get(i).unwrap();
            if entry.outage_id == outage_id {
                matches.push_back(entry);
            }
        }
        Ok(matches)
    }

    // -------------------------------------------------------------------
    // SC-061: Latest result by outage identifier
    // -------------------------------------------------------------------

    /// Returns the most recent history entry for the given `outage_id`, or `None`
    /// if no entry exists for that outage.
    pub fn get_latest_by_outage(env: Env, outage_id: Symbol) -> Result<Option<SLAResult>, SLAError> {
        Self::check_version(&env)?;
        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let mut latest: Option<SLAResult> = None;
        for i in (0..history.len()).rev() {
            let entry = history.get(i).unwrap();
            if entry.outage_id == outage_id {
                latest = Some(entry);
                break;
            }
        }
        Ok(latest)
    }

    // -------------------------------------------------------------------
    // SC-079: Read-only history / retention helpers
    // -------------------------------------------------------------------

    /// Returns the number of severity tiers currently configured.
    /// Off-chain consumers can inspect retention state without fetching the full map.
    pub fn get_config_count(env: Env) -> Result<u32, SLAError> {
        Self::check_version(&env)?;
        let configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;
        Ok(configs.len())
    }

    /// Returns the current storage schema version so off-chain consumers can
    /// detect whether a migration has occurred.
    pub fn get_storage_version(env: Env) -> Result<u32, SLAError> {
        env.storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    // -------------------------------------------------------------------
    // SC-013 – Configurable retention limit (admin only)
    // -------------------------------------------------------------------

    /// Set the maximum number of history entries to retain.
    /// Must be between 1 and MAX_HISTORY_SIZE (1000). Admin only.
    /// The new limit takes effect on the next `calculate_sla` call.
    pub fn set_retention_limit(env: Env, caller: Address, limit: u32) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        Self::require_not_frozen(&env)?;
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
            .unwrap_or_else(|| Vec::new(&env));
        let len = history.len();
        if len > limit {
            let remove_count = len - limit;
            let mut new_history = Vec::new(&env);
            for i in remove_count..len {
                new_history.push_back(history.get(i).unwrap());
            }
            Self::update_history_and_cache(&env, &new_history);
            env.events()
                .publish((EVENT_PRUNED, EVENT_VERSION, caller), (remove_count, limit));
        }
        Ok(())
    }

    /// Returns the current configurable retention limit.
    /// Defaults to MAX_HISTORY_SIZE (1000) if never explicitly set.
    pub fn get_retention_limit(env: Env) -> Result<u32, SLAError> {
        Self::check_version(&env)?;
        Ok(env
            .storage()
            .instance()
            .get(&RETENTION_LIMIT_KEY)
            .unwrap_or(MAX_HISTORY_SIZE))
    }

    /// Internal helper to update history and maintain cached length atomically.
    /// Addresses issue #463: allows get_full_audit_state to report history_len
    /// without deserializing the full vector, keeping "one-shot bootstrap" cheap.
    fn update_history_and_cache(env: &Env, history: &Vec<SLAResult>) {
        env.storage().instance().set(&HISTORY_KEY, history);
        env.storage().instance().set(&HISTORY_LEN_KEY, &history.len());
    }

    /// Internal helper for pagination slice computation (issue #264).
    /// Returns the clamped end index and the slice items for a page.
    /// Encapsulates the pagination policy defined in HISTORY_PAGINATION_POLICY.md.
    fn compute_page_slice(
        env: &Env,
        history: &Vec<SLAResult>,
        offset: u32,
        limit: u32,
    ) -> (u32, Vec<SLAResult>) {
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

    /// SC-021 – Migration state read helper
    ///
    /// Returns the storage version and migration posture.
    ///
    /// Backend consumers should call this after any contract upgrade to confirm
    /// the storage version matches expectations. If `needs_migration` is true,
    /// the admin must call `migrate` before versioned endpoints will respond.
    ///
    /// # Startup Handshake Protocol
    ///
    /// 1. Call `get_migration_state()` at backend startup or after reconnect.
    /// 2. Check `needs_migration`:
    ///    - `false` → contract is ready. Proceed with normal operations.
    ///    - `true`  → Do NOT issue operational transactions until migration
    ///      is resolved. They will fail with `VersionMismatch`.
    /// 3. If `stored_version < expected_version` → admin must call `migrate()`.
    ///    If `stored_version > expected_version` → backend binary is outdated.
    /// 4. Re-check after migration to confirm `needs_migration` is `false`.
    ///
    /// # Operator Monitoring
    ///
    /// - **Health-check loop:** Poll every N blocks. Alert if
    ///   `needs_migration` flips to `true` unexpectedly.
    /// - **Pre-deployment gate:** Compare `stored_version` of live contract
    ///   against `expected_version` of binary being deployed. Block if
    ///   migration would be required.
    /// - **Canary verification:** Before rolling out a new backend version,
    ///   have a canary instance verify against a staging contract.
    ///
    /// See [`docs/MIGRATION_STATE_CONSUMPTION.md`](../docs/MIGRATION_STATE_CONSUMPTION.md)
    /// for the full consumption guide with diagrams, troubleshooting, and
    /// operator runbook.
    ///
    /// # Design
    ///
    /// This function intentionally bypasses `check_version` so it remains
    /// callable even when the contract is in a pre-migration state.
    pub fn get_migration_state(env: Env) -> Result<StorageVersionInfo, SLAError> {
        let stored_version: u32 = env
            .storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)?;
        Ok(StorageVersionInfo {
            stored_version,
            expected_version: STORAGE_VERSION,
            needs_migration: stored_version != STORAGE_VERSION,
        })
    }

    // -------------------------------------------------------------------
    // SC-W5-029 – Version negotiation endpoint for backend handshake
    // -------------------------------------------------------------------

    /// Returns a combined version negotiation snapshot for backend startup.
    ///
    /// Intentionally bypasses `check_version` so it remains callable even when
    /// the contract is in a pre-migration state — backends must be able to read
    /// this before deciding whether to call `migrate`.
    pub fn get_version_info(env: Env) -> Result<VersionInfo, SLAError> {
        let stored_version: u32 = env
            .storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)?;
        let is_paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
        Ok(VersionInfo {
            storage_version: stored_version,
            result_schema_version: RESULT_SCHEMA_VERSION,
            needs_migration: stored_version != STORAGE_VERSION,
            is_paused,
            contract_name: symbol_short!("sla_calc"),
        })
    }

    // -------------------------------------------------------------------
    // SC-W5-078 – Version negotiation endpoint for multi-contract handshake
    // -------------------------------------------------------------------

    /// Returns the `VersionNegotiationInfo` for this contract, exposing the
    /// version-negotiation protocol data (`protocol_version`,
    /// `min_compatible_protocol`, storage version, pause & migration state)
    /// over a live contract method.
    ///
    /// This makes the multi-contract handshake documented in
    /// `version_negotiation.rs` and `docs/VERSION_NEGOTIATION_CONTRIBUTOR_GUIDE.md`
    /// actually runnable: a coordinator/backend calls this on each peer to
    /// obtain the `protocol_version`/`min_compatible_protocol` it needs to
    /// feed `negotiate_contract_versions` off-chain (or via a cross-contract
    /// coordinator), instead of the data living only in dead code (#427).
    ///
    /// Like `get_version_info`, this intentionally bypasses `check_version`
    /// so it remains callable even in a pre-migration or pre-init state.
    ///
    /// # Returns
    /// The `VersionNegotiationInfo` for this contract (empty peers: a
    /// coordinator can then run the negotiation rules against a peer list it
    /// assembles from these responses).
    pub fn get_version_negotiation_info(
        env: Env,
    ) -> Result<crate::version_negotiation::VersionNegotiationInfo, SLAError> {
        let stored_version: u32 = env
            .storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)?;
        let is_paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
        Ok(crate::version_negotiation::build_negotiation_info(
            stored_version,
            STORAGE_VERSION,
            is_paused,
        ))
    }

    // -------------------------------------------------------------------
    // #218 – Read-only healthcheck path for backend startup readiness
    // -------------------------------------------------------------------

    /// Returns a simple healthcheck result for load-balancer probes.
    ///
    /// Does **not** require authentication, does **not** modify state, and
    /// does **not** emit events. Returns `ready: true` when the contract is
    /// initialised and on the expected storage version. Any other state
    /// returns `ready: false` with a descriptive status label so operators
    /// can diagnose the issue without decoding error codes.
    ///
    /// This function intentionally bypasses `check_version` (like
    /// `get_version_info` and `get_migration_state`) so it remains callable
    /// even when the contract is in a pre-migration or pre-init state.
    ///
    /// # Readiness Definition
    ///
    /// The healthcheck returns `ready: true` only when the contract is:
    /// - Initialized (storage version matches expected version)
    /// - Has an admin (not permanently renounced)
    ///
    /// This definition focuses on operational readiness for governance functions.
    /// Pause/freeze states are not included in the readiness check to keep the
    /// probe simple; operators should use `get_contract_state_fingerprint` for
    /// full state visibility.
    ///
    /// # Status Vocabulary
    ///
    /// - `noinit`: Contract has never been initialized
    /// - `migrate`: Storage version mismatch, migration required
    /// - `noadmin`: Admin has been permanently renounced (governance-dead)
    /// - `ok`: Contract is operational and has an admin
    pub fn healthcheck(env: Env) -> HealthcheckResult {
        let stored_version: Option<u32> = env.storage().instance().get(&STORAGE_VERSION_KEY);
        let admin_renounced: Option<bool> = env.storage().instance().get(&ADMIN_RENOUNCED_KEY);

        let (ready, status) = match stored_version {
            None => (false, symbol_short!("noinit")),
            Some(v) if v != STORAGE_VERSION => (false, symbol_short!("migrate")),
            Some(_) => {
                // Check if admin has been permanently renounced
                if admin_renounced == Some(true) {
                    (false, symbol_short!("noadmin"))
                } else {
                    (true, symbol_short!("ok"))
                }
            }
        };
        HealthcheckResult {
            ready,
            contract_name: symbol_short!("sla_calc"),
            status,
        }
    }

    // -------------------------------------------------------------------
    // #261 – Contract state fingerprint for release review and upgrade planning
    // -------------------------------------------------------------------

    /// Returns a compact, deterministic snapshot of the contract's live state.
    ///
    /// Combines storage version, configuration hash, pause state, configuration
    /// freeze state, and migration posture into a single fingerprint object.
    /// This is a read-only view that does **not** require authentication,
    /// does **not** mutate state, and does **not** emit events.
    ///
    /// # Use Cases
    ///
    /// - **Pre-upgrade audit**: capture the fingerprint before deploying a new
    ///   contract version, then compare it against the post-upgrade fingerprint
    ///   to verify only expected state changed.
    /// - **Incident response**: quickly surface the contract's posture during an
    ///   incident without issuing separate queries for version, config, pause, etc.
    /// - **Backend health checks**: backends can cache this fingerprint and poll
    ///   it periodically to detect unexpected state drift (e.g., an admin paused
    ///   the contract or froze the config without notifying the backend).
    ///
    /// # Returns
    ///
    /// A `ContractStateFingerprint` containing:
    /// - `contract_name`: fixed to "sla_calc"
    /// - `storage_version`: the version currently stamped in storage
    /// - `result_schema_version`: the `SLAResult` schema version this binary expects
    /// - `config_version_hash`: deterministic hash of the current config snapshot
    /// - `is_paused`: true when the contract is paused
    /// - `needs_migration`: true when `storage_version != STORAGE_VERSION`
    /// - `is_config_frozen`: true when the config is frozen
    /// - `captured_at`: ledger timestamp when this fingerprint was captured (seconds)
    ///
    /// # Errors
    ///
    /// Returns `NotInitialized` if the contract has never been initialized (no
    /// `STORAGE_VERSION_KEY` present).
    ///
    /// Returns `NotInitialized` or `ConfigNotFound` when the configuration is
    /// unreadable (e.g. `CONFIG_KEY` is missing or a canonical severity is absent
    /// from the stored map). This is deliberate (#494): a corrupt config must not
    /// be reported as a valid fingerprint with a bogus `config_version_hash` of
    /// `0` — an operator comparing pre/post-upgrade fingerprints would otherwise
    /// be unable to tell a corrupt config from a merely different one. The
    /// endpoint's audit purpose (release review, upgrade planning, incident
    /// response) requires the unreadable-config case to surface as an error.
    ///
    /// All *readable* contract states return a valid fingerprint, including
    /// pre-migration and paused states.
    ///
    /// # Safety
    ///
    /// This function intentionally bypasses `check_version` so it remains callable
    /// even when the contract is in a pre-migration state (`needs_migration == true`).
    /// The fingerprint itself reports the migration state, so backends can decide
    /// whether to proceed or wait for `migrate()` to complete.
    pub fn get_contract_state_fingerprint(env: Env) -> Result<ContractStateFingerprint, SLAError> {
        // Read storage version — this is the only field that can cause NotInitialized.
        let stored_version: u32 = env
            .storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)?;

        // Compute needs_migration without requiring check_version to pass.
        let needs_migration = stored_version != STORAGE_VERSION;

        // Config version hash computation is safe even in pre-migration state
        // because load_config works across all initialized states. An unreadable
        // config (missing CONFIG_KEY, or a canonical severity absent from the
        // map) is propagated as an error rather than masked with a sentinel 0:
        // a legitimate hash is never 0, so 0 would be indistinguishable from a
        // corrupt config (see #494).
        let config_version_hash: u64 = Self::compute_config_version_hash(&env)?;

        // Pause and freeze state default to false if keys are absent.
        let is_paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
        let is_config_frozen: bool = config_freeze::is_config_frozen(&env);

        // Capture the current ledger timestamp.
        let captured_at = env.ledger().timestamp();

        Ok(ContractStateFingerprint {
            contract_name: symbol_short!("sla_calc"),
            storage_version: stored_version,
            result_schema_version: RESULT_SCHEMA_VERSION,
            config_version_hash,
            is_paused,
            needs_migration,
            is_config_frozen,
            captured_at,
        })
    }
}
