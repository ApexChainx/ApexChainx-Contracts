#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, String,
    Symbol, Vec,
};

#[contract]
pub struct SLACalculatorContract;

#[cfg(test)]
mod tests;

// -----------------------------------------------------------------------
// Storage keys
// -----------------------------------------------------------------------
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
const OPERATOR_KEY: Symbol = symbol_short!("OPERATOR"); // #28
const PENDING_ADMIN_KEY: Symbol = symbol_short!("PADMIN"); // #63
const PENDING_OP_KEY: Symbol = symbol_short!("POP"); // #64
const CONFIG_KEY: Symbol = symbol_short!("CONFIG");
const PAUSED_KEY: Symbol = symbol_short!("PAUSED"); // #27
const PAUSE_INFO_KEY: Symbol = symbol_short!("PAUSEINF"); // #66
const STATS_KEY: Symbol = symbol_short!("STATS"); // #29
const HISTORY_KEY: Symbol = symbol_short!("HIST");
const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");
const STORAGE_VERSION: u32 = 1;
const RESULT_SCHEMA_VERSION: u32 = 1;
const MAX_HISTORY_SIZE: u32 = 1000; // SC-062: bounded retention cap
const RETENTION_LIMIT_KEY: Symbol = symbol_short!("RETLIM"); // SC-013: configurable retention

// -----------------------------------------------------------------------
// Events
//
// All events share the same topic layout:
//   topic[0] = event name (Symbol constant below)
//   topic[1] = event version ("v1")
//   topic[2] = event-specific context (severity, caller address, etc.)
//
// Event payloads (data tuple field order):
//
//   sla_calc  → (outage_id: Symbol, status: Symbol, payment_type: Symbol,
//                rating: Symbol, mttr_minutes: u32, threshold_minutes: u32,
//                amount: i128)
//
//   cfg_upd   → (threshold_minutes: u32, penalty_per_minute: i128,
//                reward_base: i128)
//             context = severity Symbol
//
//   paused    → (true,)
//   unpause   → (false,)
//             context = caller Address
//
//   op_set    → (new_operator: Address,)
//             context = caller Address
//
//   pruned    → (removed_count: u32, kept_count: u32)
//             context = caller Address
//
// Versioning: breaking payload changes increment the version symbol (v2, …).
// Additive fields are not considered breaking.
// -----------------------------------------------------------------------
const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");
const EVENT_CONFIG_UPD: Symbol = symbol_short!("cfg_upd");
const EVENT_PAUSED: Symbol = symbol_short!("paused"); // #27
const EVENT_UNPAUSED: Symbol = symbol_short!("unpause"); // #27
const EVENT_OP_SET: Symbol = symbol_short!("op_set"); // #28
const EVENT_PRUNED: Symbol = symbol_short!("pruned");
const EVENT_PRUNED_AGE: Symbol = symbol_short!("pruned_a"); // SC-063
const EVENT_ADMIN_PROP: Symbol = symbol_short!("adm_prop"); // #63
const EVENT_ADMIN_ACC: Symbol = symbol_short!("adm_acc"); // #63
const EVENT_ADMIN_REN: Symbol = symbol_short!("adm_ren"); // #65
const EVENT_OP_PROP: Symbol = symbol_short!("op_prop"); // #64
const EVENT_OP_ACC: Symbol = symbol_short!("op_acc"); // #64
const EVENT_VERSION: Symbol = symbol_short!("v1");

// -----------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SLAError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    ConfigNotFound = 4,
    VersionMismatch = 5,
    ContractPaused = 6,    // #27
    NoPendingTransfer = 7, // #63 #64
    InvalidThreshold = 8,   // #70
    InvalidPenalty = 9,    // #70
    InvalidReward = 10,    // #70
    InvalidSeverity = 11,  // #70
    RetentionLimitOutOfRange = 12, // SC-013
}

// -----------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfig {
    pub threshold_minutes: u32,
    pub penalty_per_minute: i128,
    pub reward_base: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAResult {
    pub outage_id: Symbol,
    pub status: Symbol, // "met" | "viol"
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
    pub amount: i128,         // negative = penalty, positive = reward
    pub payment_type: Symbol, // "rew" | "pen"
    pub rating: Symbol,       // "top" | "excel" | "good" | "poor"
    pub recorded_at: u64,     // SC-063: ledger timestamp at calculation time
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfigEntry {
    pub severity: Symbol,
    pub config: SLAConfig,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfigSnapshot {
    pub version: Symbol,
    pub entries: Vec<SLAConfigEntry>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAResultSchema {
    pub version: Symbol,
    pub schema_version: u32,
    pub status_met: Symbol,
    pub status_violated: Symbol,
    pub payment_reward: Symbol,
    pub payment_penalty: Symbol,
    pub rating_exceptional: Symbol,
    pub rating_excellent: Symbol,
    pub rating_good: Symbol,
    pub rating_poor: Symbol,
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

/// #66 – Pause metadata stored when the contract is paused.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseInfo {
    pub reason: String,
    pub paused_at: u64, // ledger timestamp (seconds)
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

    // -------------------------------------------------------------------
    // #61 – Storage migration harness
    // -------------------------------------------------------------------

    /// Migrate storage from a previous version to the current one.
    /// Must be called by admin after a contract upgrade that bumps STORAGE_VERSION.
    /// Currently handles v0→v1 (first-time version stamp on legacy state).
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

        if stored == STORAGE_VERSION {
            // Already current – idempotent no-op
            return Ok(());
        }

        // v0 → v1: stamp the version; all other fields were set by initialize
        if stored == 0 {
            Self::write_version(&env);
            return Ok(());
        }

        Err(SLAError::VersionMismatch)
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
        env.storage().instance().remove(&PENDING_ADMIN_KEY);
        env.events()
            .publish((EVENT_ADMIN_ACC, EVENT_VERSION, caller), ());
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
        env.storage()
            .instance()
            .set(&PENDING_OP_KEY, &new_operator);
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
    pub fn renounce_admin(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;
        env.storage().instance().remove(&ADMIN_KEY);
        env.storage().instance().remove(&PENDING_ADMIN_KEY);
        env.events()
            .publish((EVENT_ADMIN_REN, EVENT_VERSION, caller), ());
        Ok(())
    }

    /// Pause the contract with a reason and timestamp.
    /// `calculate_sla` will be blocked until unpaused.
    /// Emits a `paused` event.
    pub fn pause(env: Env, caller: Address, reason: String) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        let paused_at = env.ledger().timestamp();
        env.storage().instance().set(&PAUSED_KEY, &true);
        env.storage()
            .instance()
            .set(&PAUSE_INFO_KEY, &PauseInfo { reason, paused_at });
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
        
        // #70 – Validate configuration parameters
        Self::validate_config(&severity, threshold_minutes, penalty_per_minute, reward_base)?;

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

    pub fn list_configs(env: Env) -> Result<Map<Symbol, SLAConfig>, SLAError> {
        Self::check_version(&env)?;
        env.storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    /// Returns a deterministic backend-friendly snapshot of all config values.
    pub fn get_config_snapshot(env: Env) -> Result<SLAConfigSnapshot, SLAError> {
        Self::check_version(&env)?;

        let mut entries = Vec::new(&env);
        let severities = [
            symbol_short!("critical"),
            symbol_short!("high"),
            symbol_short!("medium"),
            symbol_short!("low"),
        ];

        for severity in severities {
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
        let severities = [
            symbol_short!("critical"),
            symbol_short!("high"),
            symbol_short!("medium"),
            symbol_short!("low"),
        ];
        
        // Polynomial rolling hash parameters for good collision resistance
        const BASE: u64 = 91138233; // Large prime number
        const MODULUS: u64 = (1u64 << 63) - 25; // Large prime (Mersenne-like)
        
        let mut hash: u64 = 1; // Start with non-zero seed
        let mut power: u64 = 1;
        
        for sev in severities {
            let cfg = Self::load_config(&env, &sev)?;
            
            // Mix each field with position-dependent weights
            let field_hash = hash
                .wrapping_mul(BASE)
                .wrapping_add(cfg.threshold_minutes as u64)
                .wrapping_mul(power)
                % MODULUS;
            
            hash = field_hash;
            power = power.wrapping_mul(BASE) % MODULUS;
            
            // Add penalty_per_minute with different weight
            hash = hash
                .wrapping_mul(BASE)
                .wrapping_add(cfg.penalty_per_minute as u64)
                .wrapping_mul(power)
                % MODULUS;
            
            power = power.wrapping_mul(BASE) % MODULUS;
            
            // Add reward_base with different weight
            hash = hash
                .wrapping_mul(BASE)
                .wrapping_add(cfg.reward_base as u64)
                .wrapping_mul(power)
                % MODULUS;
            
            power = power.wrapping_mul(BASE) % MODULUS;
        }
        
        // Final mixing to improve distribution
        hash = hash.wrapping_mul(BASE).wrapping_add(0x9e3779b97f4a7c15u64) % MODULUS;
        Ok(hash)
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
        })
    }

    /// #60 – Returns static contract capabilities for backend introspection.
    pub fn get_contract_metadata(env: Env) -> Result<ContractMetadata, SLAError> {
        Self::check_version(&env)?;
        let mut severities = Vec::new(&env);
        severities.push_back(symbol_short!("critical"));
        severities.push_back(symbol_short!("high"));
        severities.push_back(symbol_short!("medium"));
        severities.push_back(symbol_short!("low"));

        let mut features = Vec::new(&env);
        features.push_back(symbol_short!("calc"));
        features.push_back(symbol_short!("audit"));
        features.push_back(symbol_short!("pause"));
        features.push_back(symbol_short!("stats"));
        features.push_back(symbol_short!("history"));

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

        // Delegate to pure internal math; recorded_at=0 for view-only calls
        Ok(Self::compute_result(outage_id, mttr_minutes, &cfg, 0))
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
        let result = Self::compute_result(outage_id.clone(), mttr_minutes, &cfg, env.ledger().timestamp());
        let mut history: Vec<SLAResult> = env
            .storage()
            .instance()
            .get(&HISTORY_KEY)
            .unwrap_or_else(|| Vec::new(&env));

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

        Self::publish_sla_event(&env, severity, &result);

        Ok(result)
    }

    // -------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------

    /// Pure helper to generate the SLAResult deterministically.
    /// `recorded_at` is the ledger timestamp at call time (0 in view/audit mode).
    fn compute_result(outage_id: Symbol, mttr_minutes: u32, cfg: &SLAConfig, recorded_at: u64) -> SLAResult {
        let threshold = cfg.threshold_minutes;

        // Case 1: SLA violated → penalty
        if mttr_minutes > threshold {
            let overtime = (mttr_minutes - threshold) as i128;
            let penalty = overtime * cfg.penalty_per_minute;

            SLAResult {
                outage_id,
                status: symbol_short!("viol"),
                mttr_minutes,
                threshold_minutes: threshold,
                amount: -penalty,
                payment_type: symbol_short!("pen"),
                rating: symbol_short!("poor"),
                recorded_at,
            }
        } else {
            // Case 2: SLA met → reward
            let performance_ratio = if threshold == 0 {
                0
            } else {
                (mttr_minutes * 100) / threshold
            };

            let (multiplier, rating) = if performance_ratio < 50 {
                (200u32, symbol_short!("top"))
            } else if performance_ratio < 75 {
                (150u32, symbol_short!("excel"))
            } else {
                (100u32, symbol_short!("good"))
            };

            let reward = (cfg.reward_base * multiplier as i128) / 100;

            SLAResult {
                outage_id,
                status: symbol_short!("met"),
                mttr_minutes,
                threshold_minutes: threshold,
                amount: reward,
                payment_type: symbol_short!("rew"),
                rating,
                recorded_at,
            }
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

    /// #27 – Blocks execution when the contract is paused.
    fn require_not_paused(env: &Env) -> Result<(), SLAError> {
        let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
        if paused {
            return Err(SLAError::ContractPaused);
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
        let valid_severities = [
            symbol_short!("critical"),
            symbol_short!("high"),
            symbol_short!("medium"),
            symbol_short!("low"),
        ];
        if !valid_severities.contains(severity) {
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

    /// Shared config lookup that borrows env (avoids consuming it).
    fn load_config(env: &Env, severity: &Symbol) -> Result<SLAConfig, SLAError> {
        let configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;
        configs
            .get(severity.clone())
            .ok_or(SLAError::ConfigNotFound)
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

        stats.total_calculations += 1;

        if met {
            stats.total_rewards += reward;
        } else {
            stats.total_violations += 1;
            stats.total_penalties += penalty;
        }

        env.storage().instance().set(&STATS_KEY, &stats);
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
            env.events().publish(
                (EVENT_PRUNED_AGE, EVENT_VERSION, caller),
                (removed, kept),
            );
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
    pub fn get_history_by_outage(
        env: Env,
        outage_id: Symbol,
    ) -> Result<Vec<SLAResult>, SLAError> {
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

    // -------------------------------------------------------------------
    // SC-021 – Migration state read helper
    // -------------------------------------------------------------------

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
}
