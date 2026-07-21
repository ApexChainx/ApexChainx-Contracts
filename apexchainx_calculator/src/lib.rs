#![no_std]
extern crate alloc;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, String,
    Symbol, Vec,
};

#[contract]
pub struct SLACalculatorContract;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fuzz_tests;

pub mod audit_state;
pub mod config_bundle;
pub mod config_freeze;
pub mod config_metadata;
pub mod coordination_harness;
pub mod cross_contract_safety;
pub mod event_correlation;
mod event_schema;
pub mod history_snapshot;
pub mod version_negotiation;
pub mod error_responses;

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
// References: Issue numbers track the original feature requirements.

/// Admin address — set during initialize, governs config and roles.
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

/// Operator address — authorized to call calculate_sla. (#28)
const OPERATOR_KEY: Symbol = symbol_short!("OPERATOR");

/// Pending admin for two-step transfer. (#63)
const PENDING_ADMIN_KEY: Symbol = symbol_short!("PADMIN");
/// Pending operator for two-step handoff. (#64)
const PENDING_OP_KEY: Symbol = symbol_short!("POP");

/// Ledger timestamp recorded the last time the admin slot was written.
/// Set on initialize and on accept_admin; read by renounce_admin for the
/// enriched lifecycle_terminal event payload.
const ADMIN_SET_AT_KEY: Symbol = symbol_short!("ADM_SETAT");

/// Map of severity -> SLAConfig for all configured severity levels.
const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

/// Map of severity -> SLAConfig for admin-defined custom severity levels,
/// distinct from the four canonical entries (critical/high/medium/low). (#93)
const CUSTOM_CONFIG_KEY: Symbol = symbol_short!("CUSTCFG");

/// Boolean flag: true when contract is paused. (#27)
const PAUSED_KEY: Symbol = symbol_short!("PAUSED");

/// Pause metadata (reason, timestamp, caller). (#66)
const PAUSE_INFO_KEY: Symbol = symbol_short!("PAUSEINF");

/// Maximum length (in bytes) for the pause reason string. (#68)
const MAX_REASON_LEN: usize = 256;

/// Cumulative SLA statistics (SLAStats struct). (#29)
const STATS_KEY: Symbol = symbol_short!("STATS");

/// Per-severity weekly calculation counters for telemetry. (#101)
const SEVERITY_CALC_COUNTS_KEY: Symbol = symbol_short!("CALCCNT");

/// Per-severity weekly violation counters for telemetry. (#101)
const SEVERITY_VIOL_COUNTS_KEY: Symbol = symbol_short!("VIOLCNT");

/// Per-severity last calculation ledger snapshot for weekly windowing. (#101)
const LAST_CALCULATION_LEDGER_KEY: Symbol = symbol_short!("CALCLDG");

/// Per-severity last violation ledger snapshot for weekly windowing. (#101)
const LAST_VIOLATION_LEDGER_KEY: Symbol = symbol_short!("VIOLLDG");

/// Ordered list of historical SLAResult entries.
const HISTORY_KEY: Symbol = symbol_short!("HIST");

/// Current on-chain storage schema version number.
const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");

/// The storage schema version this contract binary expects.
/// Incremented when breaking state changes are introduced.
const STORAGE_VERSION: u32 = 1;

/// Version of the SLAResult schema exposed via get_result_schema().
/// Incremented when result encoding changes in a breaking way.
const RESULT_SCHEMA_VERSION: u32 = 1;

/// Hard upper bound on retained history entries. (SC-062)
/// Configurable down to 1 via set_retention_limit().
const MAX_HISTORY_SIZE: u32 = 1000;

/// Optional configurable retention limit override. (SC-013)
/// When set, overrides MAX_HISTORY_SIZE for history trimming.
const RETENTION_LIMIT_KEY: Symbol = symbol_short!("RETLIM");

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
// sla_calc  → (outage_id: Symbol, status: Symbol, payment_type: Symbol,
//              rating: Symbol, mttr_minutes: u32, threshold_minutes: u32,
//              amount: i128)
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
// adm_ren   → (caller: Address, ledger_seq: u32, last_admin_set_at: u64)
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
// set_int   → (outage_id: Symbol, status: Symbol, payment_type: Symbol,
//              amount: i128, config_version_hash: u64, recorded_at: u64)
//   context: severity Symbol
// -----------------------------------------------------------------------

/// Emitted on successful SLA calculation. Primary event for backend consumers.
const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");

/// Emitted alongside sla_calc for settlement intent reconciliation.
const EVENT_SETTLE_INTENT: Symbol = symbol_short!("set_int");

/// Emitted when configuration is updated via set_config.
const EVENT_CONFIG_UPD: Symbol = symbol_short!("cfg_upd");

/// Emitted when the contract is paused by admin. (#27)
const EVENT_PAUSED: Symbol = symbol_short!("paused");

/// Emitted when the contract is unpaused by admin. (#27)
const EVENT_UNPAUSED: Symbol = symbol_short!("unpause");

/// Emitted when the operator address is changed. (#28)
const EVENT_OP_SET: Symbol = symbol_short!("op_set");

/// Emitted after a prune_history call removes entries.
const EVENT_PRUNED: Symbol = symbol_short!("pruned");

/// Emitted after a prune_history_by_age call removes entries. (SC-063)
const EVENT_PRUNED_AGE: Symbol = symbol_short!("pruned_a");

/// Emitted when a new admin is proposed. (#63)
const EVENT_ADMIN_PROP: Symbol = symbol_short!("adm_prop");

/// Emitted when a pending admin proposal is accepted. (#63)
const EVENT_ADMIN_ACC: Symbol = symbol_short!("adm_acc");

/// Emitted when a pending admin proposal is cancelled. (SC-024)
const EVENT_ADMIN_CAN: Symbol = symbol_short!("adm_can");

/// Emitted when the admin permanently renounces their role. (#65)
const EVENT_ADMIN_REN: Symbol = symbol_short!("adm_ren");

/// Emitted when a new operator is proposed. (#64)
const EVENT_OP_PROP: Symbol = symbol_short!("op_prop");

/// Emitted when a pending operator proposal is accepted. (#64)
const EVENT_OP_ACC: Symbol = symbol_short!("op_acc");

/// Emitted when a pending operator proposal is cancelled. (SC-024)
const EVENT_OP_CAN: Symbol = symbol_short!("op_can");

/// Emitted when the configuration is frozen by admin.
const EVENT_CONFIG_FREEZE: Symbol = symbol_short!("cfg_frz");

/// Emitted when the configuration is unfrozen by admin.
const EVENT_CONFIG_UNFREEZE: Symbol = symbol_short!("cfg_unfrz");

/// Canonical event version symbol used by all events.
const EVENT_VERSION: Symbol = symbol_short!("v1");

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
// -----------------------------------------------------------------------

/// Contract has already been initialized — cannot initialize twice.
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
    /// Duplicate outage_id with conflicting inputs detected. (SC-W5-046)
    DuplicateOutageInput = 13,
    /// Computed penalty amount is invalid (e.g., overflowed to zero). (SC-W5-046)
    InvalidPenaltyAmount = 14,
    /// Computed reward amount is invalid (e.g., zero or negative). (SC-W5-046)
    InvalidRewardAmount = 15,
    /// Configuration is frozen — config changes are blocked.
    ConfigFrozen = 16,
    /// Input parameter violates documented constraints (e.g., reason too long). (#68)
    InvalidInput = 17,
    /// Custom severity referenced but not registered. (#93)
    SeverityNotInSet = 18,
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

/// A single severity-to-config mapping entry in a config snapshot.
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
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAResultSchema {
    /// Schema version label.
    pub version: Symbol,
    /// Numeric schema version (incremented on breaking changes).
    pub schema_version: u32,
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
}

/// #60 – Single introspection call for backend clients.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractMetadata {
    pub contract_name: Symbol,
    pub storage_version: u32,
    pub result_schema_version: u32,
    pub supported_severities: Vec<Symbol>,
    pub features: Vec<Symbol>,
}

/// #29 – Cumulative on-chain SLA performance metrics.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAStats {
    pub total_calculations: u64,
    pub total_violations: u64,
    pub total_rewards: i128,   // sum of all reward amounts paid out
    pub total_penalties: i128, // sum of all penalty amounts (stored positive)
}

/// #101 – Per-severity weekly violation-rate telemetry snapshot.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeverityTelemetry {
    pub severity: Symbol,
    pub calculations: u32,
    pub violations: u32,
    pub violation_rate: u32,
}

/// #66 – Pause metadata stored when the contract is paused.
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
#[contractimpl]
impl SLACalculatorContract {
    // -------------------------------------------------------------------
    // Initialisation
    // -------------------------------------------------------------------

    /// Deploy the contract.
    /// `admin`    – may update config, pause/unpause, and assign the operator.
    /// `operator` – may call `calculate_sla`.
    pub fn initialize(env: Env, admin: Address, operator: Address) -> Result<(), SLAError> {
        if env.storage().instance().has(&ADMIN_KEY) {
            return Err(SLAError::AlreadyInitialized);
        }

        env.storage().instance().set(&ADMIN_KEY, &admin);
        // Record the timestamp at which the admin slot was first populated so
        // renounce_admin can include it in the lifecycle_terminal event payload.
        env.storage()
            .instance()
            .set(&ADMIN_SET_AT_KEY, &env.ledger().timestamp());
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
        env.storage()
            .instance()
            .set(&SEVERITY_CALC_COUNTS_KEY, &[0u32; 4]);
        env.storage()
            .instance()
            .set(&SEVERITY_VIOL_COUNTS_KEY, &[0u32; 4]);
        env.storage()
            .instance()
            .set(&LAST_CALCULATION_LEDGER_KEY, &[0u32; 4]);
        env.storage()
            .instance()
            .set(&LAST_VIOLATION_LEDGER_KEY, &[0u32; 4]);
        env.storage()
            .instance()
            .set(&HISTORY_KEY, &Vec::<SLAResult>::new(&env));

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
            inst.set(&SEVERITY_CALC_COUNTS_KEY, &[0u32; 4]);
        }

        if !inst.has(&SEVERITY_VIOL_COUNTS_KEY) {
            inst.set(&SEVERITY_VIOL_COUNTS_KEY, &[0u32; 4]);
        }

        if !inst.has(&LAST_CALCULATION_LEDGER_KEY) {
            inst.set(&LAST_CALCULATION_LEDGER_KEY, &[0u32; 4]);
        }

        if !inst.has(&LAST_VIOLATION_LEDGER_KEY) {
            inst.set(&LAST_VIOLATION_LEDGER_KEY, &[0u32; 4]);
        }

        if !inst.has(&HISTORY_KEY) {
            inst.set(&HISTORY_KEY, &Vec::<SLAResult>::new(env));
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
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if caller != admin {
            return Err(SLAError::Unauthorized);
        }

        let stored: u32 = env
            .storage()
            .instance()
            .get(&STORAGE_VERSION_KEY)
            .unwrap_or(0);

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

        // v1 → v2 (placeholder for the next breaking state change):
        // if current == 1 {
        //     // … transform state …
        //     env.storage().instance().set(&STORAGE_VERSION_KEY, &2u32);
        //     current = 2;
        // }

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

    /// Replace the operator address (admin only).
    /// Emits an `op_set` event.
    pub fn set_operator(env: Env, caller: Address, new_operator: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        env.storage().instance().set(&OPERATOR_KEY, &new_operator);

        env.events().publish(
            (EVENT_OP_SET, EVENT_VERSION, caller),
            (new_operator.clone(),),
        );

        Ok(())
    }

    // -------------------------------------------------------------------
    // #63 – Two-step admin transfer
    // -------------------------------------------------------------------

    /// Propose a new admin. The current admin initiates; the new admin must call `accept_admin`.
    pub fn propose_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        env.storage().instance().set(&PENDING_ADMIN_KEY, &new_admin);
        env.events()
            .publish((EVENT_ADMIN_PROP, EVENT_VERSION, caller), (new_admin,));
        Ok(())
    }

    /// Accept a pending admin transfer. Must be called by the proposed new admin.
    /// On success the caller becomes admin and the pending proposal is cleared.
    pub fn accept_admin(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        let pending: Address = env
            .storage()
            .instance()
            .get(&PENDING_ADMIN_KEY)
            .ok_or(SLAError::NoPendingTransfer)?;
        if caller != pending {
            return Err(SLAError::Unauthorized);
        }
        env.storage().instance().set(&ADMIN_KEY, &caller);
        // Refresh the timestamp so renounce_admin reports when the *current*
        // admin was installed, not the original one.
        env.storage()
            .instance()
            .set(&ADMIN_SET_AT_KEY, &env.ledger().timestamp());
        env.storage().instance().remove(&PENDING_ADMIN_KEY);
        env.events()
            .publish((EVENT_ADMIN_ACC, EVENT_VERSION, caller), ());
        Ok(())
    }

    /// Cancel a pending admin transfer. Only the current admin may cancel.
    /// Clears the pending proposal without changing the active admin.
    /// Returns `NoPendingTransfer` if there is nothing to cancel.
    pub fn cancel_admin_proposal(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        if !env.storage().instance().has(&PENDING_ADMIN_KEY) {
            return Err(SLAError::NoPendingTransfer);
        }
        env.storage().instance().remove(&PENDING_ADMIN_KEY);
        env.events()
            .publish((EVENT_ADMIN_CAN, EVENT_VERSION, caller), ());
        Ok(())
    }

    /// Returns the pending admin address, if any.
    pub fn get_pending_admin(env: Env) -> Result<Option<Address>, SLAError> {
        Self::check_version(&env)?;
        Ok(env.storage().instance().get(&PENDING_ADMIN_KEY))
    }

    // -------------------------------------------------------------------
    // #64 – Two-step operator handoff
    // -------------------------------------------------------------------

    /// Propose a new operator. The current admin initiates; the new operator must call `accept_operator`.
    pub fn propose_operator(
        env: Env,
        caller: Address,
        new_operator: Address,
    ) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        env.storage().instance().set(&PENDING_OP_KEY, &new_operator);
        env.events()
            .publish((EVENT_OP_PROP, EVENT_VERSION, caller), (new_operator,));
        Ok(())
    }

    /// Accept a pending operator handoff. Must be called by the proposed new operator.
    pub fn accept_operator(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        let pending: Address = env
            .storage()
            .instance()
            .get(&PENDING_OP_KEY)
            .ok_or(SLAError::NoPendingTransfer)?;
        if caller != pending {
            return Err(SLAError::Unauthorized);
        }
        env.storage().instance().set(&OPERATOR_KEY, &caller);
        env.storage().instance().remove(&PENDING_OP_KEY);
        env.events()
            .publish((EVENT_OP_ACC, EVENT_VERSION, caller), ());
        Ok(())
    }

    /// Cancel a pending operator proposal. Only the current admin may cancel.
    /// Clears the pending proposal without changing the active operator.
    /// Returns `NoPendingTransfer` if there is nothing to cancel.
    pub fn cancel_operator_proposal(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        if !env.storage().instance().has(&PENDING_OP_KEY) {
            return Err(SLAError::NoPendingTransfer);
        }
        env.storage().instance().remove(&PENDING_OP_KEY);
        env.events()
            .publish((EVENT_OP_CAN, EVENT_VERSION, caller), ());
        Ok(())
    }

    /// Returns the pending operator address, if any.
    pub fn get_pending_operator(env: Env) -> Result<Option<Address>, SLAError> {
        Self::check_version(&env)?;
        Ok(env.storage().instance().get(&PENDING_OP_KEY))
    }

    // -------------------------------------------------------------------
    // #65 – Admin renounce
    // -------------------------------------------------------------------

    /// Permanently renounce admin authority. This is irreversible: no admin will
    /// exist after this call and admin-gated functions will be permanently locked.
    /// Any pending admin proposal is also cleared.
    ///
    /// Emits `adm_ren` with payload `(caller: Address, ledger_seq: u32, last_admin_set_at: u64)`.
    /// `last_admin_set_at` is the ledger timestamp recorded when the current admin
    /// was last installed (via `initialize` or `accept_admin`). Backends can use
    /// this to correlate config snapshots that were active during the admin's tenure.
    pub fn renounce_admin(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        // Capture lifecycle metadata before removing storage.
        let ledger_seq: u32 = env.ledger().sequence();
        let last_admin_set_at: u64 = env
            .storage()
            .instance()
            .get(&ADMIN_SET_AT_KEY)
            .unwrap_or(0u64);

        env.storage().instance().remove(&ADMIN_KEY);
        env.storage().instance().remove(&PENDING_ADMIN_KEY);
        env.storage().instance().remove(&ADMIN_SET_AT_KEY);

        env.events().publish(
            (EVENT_ADMIN_REN, EVENT_VERSION, caller.clone()),
            (caller, ledger_seq, last_admin_set_at),
        );
        Ok(())
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
        Self::validate_config(
            &severity,
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        )?;

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

        env.events().publish(
            (EVENT_CONFIG_UPD, EVENT_VERSION, severity),
            (threshold_minutes, penalty_per_minute, reward_base),
        );
        Ok(())
    }

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

        custom.set(
            severity.clone(),
            SLAConfig {
                threshold_minutes,
                penalty_per_minute,
                reward_base,
            },
        );
        env.storage().instance().set(&CUSTOM_CONFIG_KEY, &custom);

        env.events().publish(
            (EVENT_CONFIG_UPD, EVENT_VERSION, severity),
            (threshold_minutes, penalty_per_minute, reward_base),
        );
        Ok(())
    }

    /// Removes a previously registered custom severity level. Admin only.
    /// Returns `SeverityNotInSet` if the severity was never registered.
    pub fn remove_custom_severity(
        env: Env,
        caller: Address,
        severity: Symbol,
    ) -> Result<(), SLAError> {
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

        env.events().publish(
            (EVENT_CONFIG_UPD, EVENT_VERSION, severity),
            (0u32, 0i128, 0i128),
        );
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
            version: symbol_short!("v1"),
            entries,
        })
    }

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
        Ok(config_metadata::get_last_config_update(&env)
            .map(|seq| ConfigUpdateInfo { sequence: seq }))
    }

    /// Returns a deterministic backend-friendly snapshot of all config values.
    pub fn get_config_snapshot(env: Env) -> Result<SLAConfigSnapshot, SLAError> {
        Self::check_version(&env)?;

        let mut entries = Vec::new(&env);

        for severity in Self::canonical_severities(&env) {
            let config = Self::load_config(&env, &severity)?;
            entries.push_back(SLAConfigEntry { severity, config });
        }

        Ok(SLAConfigSnapshot {
            version: symbol_short!("v1"),
            entries,
        })
    }

    /// Returns a deterministic config version hash so backend sync logic can
    /// detect meaningful config changes cheaply.
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
        let entries: [(u32, &str, &str); 18] = [
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
            (
                12,
                "RetentionLimitOutOfRange",
                "Retention limit out of range",
            ),
            (13, "DuplicateOutageInput", "Duplicate outage input"),
            (14, "InvalidPenaltyAmount", "Invalid penalty amount"),
            (15, "InvalidRewardAmount", "Invalid reward amount"),
            (16, "ConfigFrozen", "Configuration is frozen"),
            (17, "InvalidInput", "Invalid input parameter"),
            (18, "SeverityNotInSet", "Custom severity not registered"),
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

    pub fn get_result_schema(env: Env) -> Result<SLAResultSchema, SLAError> {
        Self::check_version(&env)?;
        Ok(SLAResultSchema {
            version: symbol_short!("v1"),
            schema_version: RESULT_SCHEMA_VERSION,
            status_met: symbol_short!("met"),
            status_violated: symbol_short!("viol"),
            payment_reward: symbol_short!("rew"),
            payment_penalty: symbol_short!("pen"),
            rating_exceptional: symbol_short!("top"),
            rating_excellent: symbol_short!("excel"),
            rating_good: symbol_short!("good"),
            rating_poor: symbol_short!("poor"),
            includes_config_version_hash: true,
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
        let schema = Self::get_result_schema(env)?;
        Ok(Some(ConfigBundle { snapshot, schema }))
    }

    pub fn get_full_audit_state(env: Env) -> Result<AuditState, SLAError> {
        Self::check_version(&env)?;

        let admin = Self::get_admin(env.clone())?;
        let operator = Self::get_operator(env.clone())?;
        let pending_admin = Self::get_pending_admin(env.clone())?;
        let pending_operator = Self::get_pending_operator(env.clone())?;
        let paused = Self::is_paused(env.clone())?;
        let pause_info = Self::get_pause_info(env.clone())?;
        let config_snapshot = Self::get_config_snapshot(env.clone())?;
        let stats = Self::get_stats(env.clone())?;
        let result_schema = Self::get_result_schema(env.clone())?;

        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let history_len = history.len();

        Ok(AuditState {
            admin,
            operator,
            pending_admin,
            pending_operator,
            paused,
            pause_info,
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

        let mut features = Vec::new(&env);
        features.push_back(symbol_short!("calc"));
        features.push_back(symbol_short!("audit"));
        features.push_back(symbol_short!("pause"));
        features.push_back(symbol_short!("stats"));
        features.push_back(symbol_short!("history"));
        features.push_back(symbol_short!("failcode"));
        features.push_back(symbol_short!("safe_call"));
        features.push_back(symbol_short!("ver_nego"));
        features.push_back(symbol_short!("corr_id"));
        features.push_back(symbol_short!("freeze"));

        Ok(ContractMetadata {
            contract_name: symbol_short!("sla_calc"),
            storage_version: STORAGE_VERSION,
            result_schema_version: RESULT_SCHEMA_VERSION,
            supported_severities: severities,
            features,
        })
    }

    // -------------------------------------------------------------------
    // #29 – Stats view
    // -------------------------------------------------------------------

    /// Returns the cumulative SLA performance statistics.
    pub fn get_stats(env: Env) -> Result<SLAStats, SLAError> {
        Self::check_version(&env)?;
        env.storage()
            .instance()
            .get(&STATS_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    /// #101 – Returns per-severity weekly violation-rate telemetry.
    pub fn get_severity_telemetry(env: Env) -> Result<Vec<SeverityTelemetry>, SLAError> {
        Self::check_version(&env)?;
        let mut telemetry = Vec::new(&env);
        let severities = Self::canonical_severities(&env);
        let calculations: [u32; 4] = env
            .storage()
            .instance()
            .get(&SEVERITY_CALC_COUNTS_KEY)
            .unwrap_or([0u32; 4]);
        let violations: [u32; 4] = env
            .storage()
            .instance()
            .get(&SEVERITY_VIOL_COUNTS_KEY)
            .unwrap_or([0u32; 4]);

        for index in 0..severities.len() {
            let severity = severities.get(index).unwrap();
            let calc_count = calculations[index as usize];
            let violation_count = violations[index as usize];
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

        Ok(telemetry)
    }

    // -------------------------------------------------------------------
    // #31 - SLA Audit Mode (View-only calculation)
    // -------------------------------------------------------------------

    /// Recalculates SLA deterministically without mutating any state or emitting events.
    /// Can be called by anyone for verification and audit purposes.
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

        // Delegate to pure internal math without mutating state or emitting events.

        // Use the current ledger timestamp so the view result matches the mutating
        // path for the same inputs executed in the same ledger, while still avoiding
        // any state writes or event emission.
        Self::compute_result(
            outage_id,
            mttr_minutes,
            &cfg,
            config_version_hash,
            env.ledger().timestamp(),
        )
    }

    // -------------------------------------------------------------------
    // SLA calculation (operator only)                                #28
    // -------------------------------------------------------------------

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
        let met = result.status != symbol_short!("viol");
        Self::record_severity_telemetry(&env, &severity, met);
        let mut history: Vec<SLAResult> = env
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
                // Explicit duplicate policy: same outage_id is idempotent only when
                // execution inputs resolve to the same deterministic result.
                if prev.mttr_minutes != mttr_minutes
                    || prev.threshold_minutes != cfg.threshold_minutes
                {
                    return Err(SLAError::DuplicateOutageInput);
                }
                return Ok(prev);
            }
            // Config changed: treat as a fresh calculation rather than a conflict.
        }

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
            env.storage().instance().set(&HISTORY_KEY, &trimmed);
        } else {
            env.storage().instance().set(&HISTORY_KEY, &history);
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
    /// (0 in view/audit mode).
    fn compute_result(
        outage_id: Symbol,
        mttr_minutes: u32,
        cfg: &SLAConfig,
        config_version_hash: u64,
        recorded_at: u64,
    ) -> Result<SLAResult, SLAError> {
        let threshold = cfg.threshold_minutes;

        if threshold == 0 {
            return Err(SLAError::InvalidThreshold);
        }

        // Case 1: SLA violated → penalty
        if mttr_minutes > threshold {
            let overtime = (mttr_minutes - threshold) as i128;
            let penalty = overtime.saturating_mul(cfg.penalty_per_minute);
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

            let reward = cfg
                .reward_base
                .saturating_mul(multiplier as i128)
                .div_euclid(100);
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

    fn check_version(env: &Env) -> Result<(), SLAError> {
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

    fn require_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
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
    fn require_operator(env: &Env, caller: &Address) -> Result<(), SLAError> {
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
    fn validate_general_bounds(
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
    fn validate_config(
        severity: &Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) -> Result<(), SLAError> {
        // Validate severity is one of the supported values
        if !Self::is_canonical_severity(severity) {
            return Err(SLAError::InvalidSeverity);
        }

        // Threshold must be between 1 and 1440 minutes (24 hours max)
        if threshold_minutes == 0 || threshold_minutes > 1440 {
            return Err(SLAError::InvalidThreshold);
        }

        // Penalty must be positive and reasonable (1 to 10000 per minute)
        if penalty_per_minute <= 0 || penalty_per_minute > 10000 {
            return Err(SLAError::InvalidPenalty);
        }

        // Reward base must be positive and reasonable (1 to 100000)
        if reward_base <= 0 || reward_base > 100000 {
            return Err(SLAError::InvalidReward);
        }

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
        } else {
            return Err(SLAError::InvalidSeverity);
        }

        Ok(())
    }

    fn canonical_severities(env: &Env) -> Vec<Symbol> {
        let mut severities = Vec::new(env);
        severities.push_back(symbol_short!("critical"));
        severities.push_back(symbol_short!("high"));
        severities.push_back(symbol_short!("medium"));
        severities.push_back(symbol_short!("low"));
        severities
    }

    fn canonical_severity_index(severity: &Symbol) -> Option<u32> {
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

    fn is_canonical_severity(severity: &Symbol) -> bool {
        Self::canonical_severity_index(severity).is_some()
    }

    /// Shared config lookup that borrows env (avoids consuming it).
    fn compute_config_version_hash(env: &Env) -> Result<u64, SLAError> {
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
    fn load_config(env: &Env, severity: &Symbol) -> Result<SLAConfig, SLAError> {
        if Self::is_canonical_severity(severity) {
            let configs: Map<Symbol, SLAConfig> = env
                .storage()
                .instance()
                .get(&CONFIG_KEY)
                .ok_or(SLAError::NotInitialized)?;
            return configs
                .get(severity.clone())
                .ok_or(SLAError::ConfigNotFound);
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
        let mut stats: SLAStats = env
            .storage()
            .instance()
            .get(&STATS_KEY)
            .unwrap_or(SLAStats {
                total_calculations: 0,
                total_violations: 0,
                total_rewards: 0,
                total_penalties: 0,
            });

        stats.total_calculations = stats.total_calculations.saturating_add(1);

        if met {
            stats.total_rewards = stats.total_rewards.saturating_add(reward);
        } else {
            stats.total_violations = stats.total_violations.saturating_add(1);
            stats.total_penalties = stats.total_penalties.saturating_add(penalty);
        }

        env.storage().instance().set(&STATS_KEY, &stats);
    }

    fn record_severity_telemetry(env: &Env, severity: &Symbol, met: bool) {
        let index = Self::canonical_severity_index(severity).unwrap_or(0) as usize;
        let mut calculations: [u32; 4] = env
            .storage()
            .instance()
            .get(&SEVERITY_CALC_COUNTS_KEY)
            .unwrap_or([0u32; 4]);
        let mut violations: [u32; 4] = env
            .storage()
            .instance()
            .get(&SEVERITY_VIOL_COUNTS_KEY)
            .unwrap_or([0u32; 4]);
        let mut last_calculations: [u32; 4] = env
            .storage()
            .instance()
            .get(&LAST_CALCULATION_LEDGER_KEY)
            .unwrap_or([0u32; 4]);
        let mut last_violations: [u32; 4] = env
            .storage()
            .instance()
            .get(&LAST_VIOLATION_LEDGER_KEY)
            .unwrap_or([0u32; 4]);

        let now = env.ledger().timestamp();
        let week_seconds = 7u64 * 24u64 * 60u64 * 60u64;
        let last_calc = last_calculations[index] as u64;
        let last_violation = last_violations[index] as u64;
        let calc_stale = last_calc != 0 && now.saturating_sub(last_calc) >= week_seconds;
        let violation_stale = last_violation != 0 && now.saturating_sub(last_violation) >= week_seconds;
        if calc_stale || violation_stale {
            calculations[index] = 0;
            violations[index] = 0;
        }

        calculations[index] = calculations[index].saturating_add(1);
        if !met {
            violations[index] = violations[index].saturating_add(1);
        }

        let current_ledger = if now > u64::from(u32::MAX) {
            u32::MAX
        } else {
            now as u32
        };
        last_calculations[index] = current_ledger;
        if !met {
            last_violations[index] = current_ledger;
        }

        env.storage().instance().set(&SEVERITY_CALC_COUNTS_KEY, &calculations);
        env.storage().instance().set(&SEVERITY_VIOL_COUNTS_KEY, &violations);
        env.storage().instance().set(&LAST_CALCULATION_LEDGER_KEY, &last_calculations);
        env.storage().instance().set(&LAST_VIOLATION_LEDGER_KEY, &last_violations);
    }

    fn publish_sla_event(env: &Env, severity: Symbol, result: &SLAResult) {
        env.events().publish(
            (EVENT_SLA_CALC, EVENT_VERSION, severity),
            (
                result.outage_id.clone(),
                result.status.clone(),
                result.payment_type.clone(),
                result.rating.clone(),
                result.mttr_minutes,
                result.threshold_minutes,
                result.amount,
            ),
        );
    }

    fn publish_settlement_intent_event(env: &Env, severity: Symbol, result: &SLAResult) {
        env.events().publish(
            (EVENT_SETTLE_INTENT, EVENT_VERSION, severity),
            (
                result.outage_id.clone(),
                result.status.clone(),
                result.payment_type.clone(),
                result.amount,
                result.config_version_hash,
                result.recorded_at,
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

        if len > keep_latest {
            let remove_count = len - keep_latest;
            let mut new_history = Vec::new(&env);

            // Rebuild the vector keeping only the most recent entries
            for i in remove_count..len {
                new_history.push_back(history.get(i).unwrap());
            }

            env.storage().instance().set(&HISTORY_KEY, &new_history);
            env.events().publish(
                (EVENT_PRUNED, EVENT_VERSION, caller),
                (remove_count, keep_latest),
            );
        }

        Ok(())
    }

    /// SC-063 – Prune history entries older than `min_age_seconds` before the
    /// current ledger timestamp.  Entries with `recorded_at == 0` (view-mode
    /// results that were never stored with a real timestamp) are always kept.
    /// Admin-only.  Emits a `pruned_a` event.
    pub fn prune_history_by_age(
        env: Env,
        caller: Address,
        min_age_seconds: u64,
    ) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        let now = env.ledger().timestamp();
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
            let kept = new_history.len();
            env.storage().instance().set(&HISTORY_KEY, &new_history);
            env.events()
                .publish((EVENT_PRUNED_AGE, EVENT_VERSION, caller), (removed, kept));
        }

        Ok(())
    }

    // -------------------------------------------------------------------
    // SC-059: History pagination
    // -------------------------------------------------------------------

    /// Returns a bounded page of history entries.
    /// `offset` is zero-based; entries are ordered oldest-first (insertion order).
    /// Returns an empty Vec when `offset` is beyond the end of history.
    pub fn get_history_page(env: Env, offset: u32, limit: u32) -> Result<Vec<SLAResult>, SLAError> {
        Self::check_version(&env)?;
        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let len = history.len();
        let mut page = Vec::new(&env);
        if offset >= len || limit == 0 {
            return Ok(page);
        }
        let end = (offset + limit).min(len);
        for i in offset..end {
            page.push_back(history.get(i).unwrap());
        }
        Ok(page)
    }

    // -------------------------------------------------------------------
    // SC-060: History query by outage identifier
    // -------------------------------------------------------------------

    /// Returns all history entries whose `outage_id` matches the given value.
    /// Returns an empty Vec when no matching entries exist.
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
    pub fn get_latest_by_outage(
        env: Env,
        outage_id: Symbol,
    ) -> Result<Option<SLAResult>, SLAError> {
        Self::check_version(&env)?;
        let history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));
        let mut latest: Option<SLAResult> = None;
        for i in 0..history.len() {
            let entry = history.get(i).unwrap();
            if entry.outage_id == outage_id {
                latest = Some(entry);
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
        if limit == 0 || limit > MAX_HISTORY_SIZE {
            return Err(SLAError::RetentionLimitOutOfRange);
        }
        env.storage().instance().set(&RETENTION_LIMIT_KEY, &limit);
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

    /// SC-021 – Migration state read helper
    ///
    /// Returns the storage version and migration posture.
    ///
    /// Backend consumers should call this after any contract upgrade to confirm
    /// the storage version matches expectations. If `needs_migration` is true,
    /// the admin must call `migrate` before versioned endpoints will respond.
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
}
