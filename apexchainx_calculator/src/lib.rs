#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol,
};

#[contract]
pub struct SLACalculatorContract;

// -----------------------------------------------------------------------
// Storage keys
// -----------------------------------------------------------------------
const ADMIN_KEY:           Symbol = symbol_short!("ADMIN");
const OPERATOR_KEY:        Symbol = symbol_short!("OPERATOR"); // #28
const CONFIG_KEY:          Symbol = symbol_short!("CONFIG");
const PAUSED_KEY:          Symbol = symbol_short!("PAUSED");   // #27
const STATS_KEY:           Symbol = symbol_short!("STATS");    // #29
const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");
const STORAGE_VERSION:     u32    = 1;

// -----------------------------------------------------------------------
// Events
// -----------------------------------------------------------------------
const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");
const EVENT_CONFIG_UPD: Symbol = symbol_short!("cfg_upd");
const EVENT_PAUSED:   Symbol = symbol_short!("paused");    // #27
const EVENT_UNPAUSED: Symbol = symbol_short!("unpause");   // #27
const EVENT_OP_SET:   Symbol = symbol_short!("op_set");    // #28

// -----------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SLAError {
    AlreadyInitialized = 1,
    NotInitialized     = 2,
    Unauthorized       = 3,
    ConfigNotFound     = 4,
    VersionMismatch    = 5,
    ContractPaused     = 6, // #27
}

// -----------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfig {
    pub threshold_minutes:  u32,
    pub penalty_per_minute: i128,
    pub reward_base:        i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAResult {
    pub outage_id:         Symbol,
    pub status:            Symbol, // "met" | "viol"
    pub mttr_minutes:      u32,
    pub threshold_minutes: u32,
    pub amount:            i128,   // negative = penalty, positive = reward
    pub payment_type:      Symbol, // "rew" | "pen"
    pub rating:            Symbol, // "top" | "excel" | "good" | "poor"
}

/// #29 – Cumulative on-chain SLA performance metrics.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAStats {
    pub total_calculations: u64,
    pub total_violations:   u64,
    pub total_rewards:      i128, // sum of all reward amounts paid out
    pub total_penalties:    i128, // sum of all penalty amounts (stored positive)
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

        env.storage().instance().set(&ADMIN_KEY,    &admin);
        env.storage().instance().set(&OPERATOR_KEY, &operator); // #28
        env.storage().instance().set(&PAUSED_KEY,   &false);    // #27

        // #29 – initialise zeroed stats
        env.storage().instance().set(&STATS_KEY, &SLAStats {
            total_calculations: 0,
            total_violations:   0,
            total_rewards:      0,
            total_penalties:    0,
        });

        let mut configs = Map::<Symbol, SLAConfig>::new(&env);
        configs.set(symbol_short!("critical"), SLAConfig { threshold_minutes: 15,  penalty_per_minute: 100, reward_base: 750 });
        configs.set(symbol_short!("high"),     SLAConfig { threshold_minutes: 30,  penalty_per_minute: 50,  reward_base: 750 });
        configs.set(symbol_short!("medium"),   SLAConfig { threshold_minutes: 60,  penalty_per_minute: 25,  reward_base: 750 });
        configs.set(symbol_short!("low"),      SLAConfig { threshold_minutes: 120, penalty_per_minute: 10,  reward_base: 600 });

        env.storage().instance().set(&CONFIG_KEY, &configs);
        Self::write_version(&env);
        Ok(())
    }

    // -------------------------------------------------------------------
    // Role queries
    // -------------------------------------------------------------------

    pub fn get_admin(env: Env) -> Result<Address, SLAError> {
        Self::check_version(&env)?;
        env.storage().instance().get(&ADMIN_KEY).ok_or(SLAError::NotInitialized)
    }

    /// #28 – Returns the current operator address.
    pub fn get_operator(env: Env) -> Result<Address, SLAError> {
        Self::check_version(&env)?;
        env.storage().instance().get(&OPERATOR_KEY).ok_or(SLAError::NotInitialized)
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
            (EVENT_OP_SET, caller),
            (new_operator,),
        );

        Ok(())
    }

    // -------------------------------------------------------------------
    // #27 – Pause / Unpause (admin only)
    // -------------------------------------------------------------------

    /// Pause the contract; `calculate_sla` will be blocked until unpaused.
    /// Emits a `paused` event.
    pub fn pause(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        env.storage().instance().set(&PAUSED_KEY, &true);
        env.events().publish((EVENT_PAUSED, caller), (true,));
        Ok(())
    }

    /// Unpause the contract.
    /// Emits an `unpause` event.
    pub fn unpause(env: Env, caller: Address) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?;

        env.storage().instance().set(&PAUSED_KEY, &false);
        env.events().publish((EVENT_UNPAUSED, caller), (false,));
        Ok(())
    }

    /// Returns `true` when the contract is paused.
    pub fn is_paused(env: Env) -> Result<bool, SLAError> {
        Self::check_version(&env)?;
        Ok(env.storage().instance().get(&PAUSED_KEY).unwrap_or(false))
    }

    // -------------------------------------------------------------------
    // Config management (admin only)                                 #28
    // -------------------------------------------------------------------

    pub fn set_config(
        env:                Env,
        caller:             Address,
        severity:           Symbol,
        threshold_minutes:  u32,
        penalty_per_minute: i128,
        reward_base:        i128,
    ) -> Result<(), SLAError> {
        Self::check_version(&env)?;
        Self::require_admin(&env, &caller)?; // #28 – admin role enforced

        let mut configs: Map<Symbol, SLAConfig> = env
            .storage().instance().get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;

        configs.set(severity.clone(), SLAConfig { threshold_minutes, penalty_per_minute, reward_base });
        env.storage().instance().set(&CONFIG_KEY, &configs);

        env.events().publish(
            (EVENT_CONFIG_UPD, severity),
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
        env.storage().instance().get(&CONFIG_KEY).ok_or(SLAError::NotInitialized)
    }

    // -------------------------------------------------------------------
    // #29 – Stats view
    // -------------------------------------------------------------------

    /// Returns the cumulative SLA performance statistics.
    pub fn get_stats(env: Env) -> Result<SLAStats, SLAError> {
        Self::check_version(&env)?;
        env.storage().instance().get(&STATS_KEY).ok_or(SLAError::NotInitialized)
    }

    // -------------------------------------------------------------------
    // SLA calculation (operator only)                                #28
    // -------------------------------------------------------------------

    pub fn calculate_sla(
        env:          Env,
        caller:       Address, // #28 – operator must identify themselves
        outage_id:    Symbol,
        severity:     Symbol,
        mttr_minutes: u32,
    ) -> Result<SLAResult, SLAError> {
        Self::check_version(&env)?;
        Self::require_not_paused(&env)?;       // #27
        Self::require_operator(&env, &caller)?; // #28

        let cfg       = Self::load_config(&env, &severity)?;
        let threshold = cfg.threshold_minutes;

        // Case 1: SLA violated → penalty
        if mttr_minutes > threshold {
            let overtime = (mttr_minutes - threshold) as i128;
            let penalty  = overtime * cfg.penalty_per_minute;

            // #29 – update stats
            Self::increment_stats(&env, false, 0, penalty);

            env.events().publish(
                (EVENT_SLA_CALC, severity.clone()),
                (outage_id.clone(), symbol_short!("viol"), -penalty),
            );

            return Ok(SLAResult {
                outage_id,
                status:            symbol_short!("viol"),
                mttr_minutes,
                threshold_minutes: threshold,
                amount:            -penalty,
                payment_type:      symbol_short!("pen"),
                rating:            symbol_short!("poor"),
            });
        }

        // Case 2: SLA met → reward
        let performance_ratio = if threshold == 0 { 0 } else { (mttr_minutes * 100) / threshold };

        let (multiplier, rating) = if performance_ratio < 50 {
            (200u32, symbol_short!("top"))
        } else if performance_ratio < 75 {
            (150u32, symbol_short!("excel"))
        } else {
            (100u32, symbol_short!("good"))
        };

        let reward = (cfg.reward_base * multiplier as i128) / 100;

        // #29 – update stats
        Self::increment_stats(&env, true, reward, 0);

        env.events().publish(
            (EVENT_SLA_CALC, severity.clone()),
            (outage_id.clone(), symbol_short!("met"), reward),
        );

        Ok(SLAResult {
            outage_id,
            status:            symbol_short!("met"),
            mttr_minutes,
            threshold_minutes: threshold,
            amount:            reward,
            payment_type:      symbol_short!("rew"),
            rating,
        })
    }

    // -------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------

    fn write_version(env: &Env) {
        env.storage().instance().set(&STORAGE_VERSION_KEY, &STORAGE_VERSION);
    }

    fn check_version(env: &Env) -> Result<(), SLAError> {
        let v: u32 = env
            .storage().instance().get(&STORAGE_VERSION_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if v != STORAGE_VERSION { return Err(SLAError::VersionMismatch); }
        Ok(())
    }

    fn require_admin(env: &Env, caller: &Address) -> Result<(), SLAError> {
        let admin: Address = env
            .storage().instance().get(&ADMIN_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if caller != &admin { return Err(SLAError::Unauthorized); }
        Ok(())
    }

    /// #28 – Ensures the caller holds the operator role.
    fn require_operator(env: &Env, caller: &Address) -> Result<(), SLAError> {
        let operator: Address = env
            .storage().instance().get(&OPERATOR_KEY)
            .ok_or(SLAError::NotInitialized)?;
        if caller != &operator { return Err(SLAError::Unauthorized); }
        Ok(())
    }

    /// #27 – Blocks execution when the contract is paused.
    fn require_not_paused(env: &Env) -> Result<(), SLAError> {
        let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
        if paused { return Err(SLAError::ContractPaused); }
        Ok(())
    }

    /// Shared config lookup that borrows env (avoids consuming it).
    fn load_config(env: &Env, severity: &Symbol) -> Result<SLAConfig, SLAError> {
        let configs: Map<Symbol, SLAConfig> = env
            .storage().instance().get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;
        configs.get(severity.clone()).ok_or(SLAError::ConfigNotFound)
    }

    /// #29 – Read-modify-write the stats entry.
    /// `met`     – true when SLA was met (reward path), false for violation.
    /// `reward`  – reward amount to add (0 on violation path).
    /// `penalty` – penalty amount to add, stored positive (0 on met path).
    fn increment_stats(env: &Env, met: bool, reward: i128, penalty: i128) {
        let mut stats: SLAStats = env
            .storage().instance().get(&STATS_KEY)
            .unwrap_or(SLAStats {
                total_calculations: 0,
                total_violations:   0,
                total_rewards:      0,
                total_penalties:    0,
            });

        stats.total_calculations += 1;

        if met {
            stats.total_rewards += reward;
        } else {
            stats.total_violations   += 1;
            stats.total_penalties    += penalty;
        }

        env.storage().instance().set(&STATS_KEY, &stats);
    }
}