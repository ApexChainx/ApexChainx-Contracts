#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol,
};

#[contract]
pub struct SLACalculatorContract;

// --------------------
// Storage keys
// --------------------
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

// --------------------
// Events
// --------------------
const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");
const EVENT_CONFIG_UPD: Symbol = symbol_short!("cfg_upd");

// --------------------
// Errors
// --------------------
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SLAError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    ConfigNotFound = 4,
}

// --------------------
// Types
// --------------------
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
    pub status: Symbol,       // "met" or "viol"
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
    pub amount: i128,         // negative = penalty, positive = reward
    pub payment_type: Symbol, // "rew" or "pen"
    pub rating: Symbol,       // "top", "excel", "good", "poor"
}

// --------------------
// Contract impl
// --------------------
#[contractimpl]
impl SLACalculatorContract {
    // --------------------
    // Init & Admin
    // --------------------

    pub fn initialize(env: Env, admin: Address) -> Result<(), SLAError> {
        if env.storage().instance().has(&ADMIN_KEY) {
            return Err(SLAError::AlreadyInitialized);
        }

        env.storage().instance().set(&ADMIN_KEY, &admin);

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

        Ok(())
    }

    pub fn get_admin(env: Env) -> Result<Address, SLAError> {
        env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    // --------------------
    // Internal helper
    // --------------------

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

    // --------------------
    // Config management
    // --------------------

pub fn set_config(
    env: Env,
    caller: Address,
    severity: Symbol,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) -> Result<(), SLAError> {
    Self::require_admin(&env, &caller)?;

    let mut configs: Map<Symbol, SLAConfig> = env
        .storage()
        .instance()
        .get(&CONFIG_KEY)
        .ok_or(SLAError::NotInitialized)?;

    let cfg = SLAConfig {
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    };

    configs.set(severity.clone(), cfg);
    env.storage().instance().set(&CONFIG_KEY, &configs);

    // 🔔 Emit config update event
    env.events().publish(
        (EVENT_CONFIG_UPD, severity),
        (threshold_minutes, penalty_per_minute, reward_base),
    );

    Ok(())
}

    pub fn get_config(env: Env, severity: Symbol) -> Result<SLAConfig, SLAError> {
        let configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)?;

        configs.get(severity).ok_or(SLAError::ConfigNotFound)
    }

    pub fn list_configs(env: Env) -> Result<Map<Symbol, SLAConfig>, SLAError> {
        env.storage()
            .instance()
            .get(&CONFIG_KEY)
            .ok_or(SLAError::NotInitialized)
    }

    // --------------------
    // SLA calculation
    // --------------------

pub fn calculate_sla(
    env: Env,
    outage_id: Symbol,
    severity: Symbol,
    mttr_minutes: u32,
) -> Result<SLAResult, SLAError> {
    let cfg = Self::get_config(env.clone(), severity.clone())?;
    let threshold = cfg.threshold_minutes;

    // --------------------
    // Case 1: violated → penalty
    // --------------------
    if mttr_minutes > threshold {
        let overtime = (mttr_minutes - threshold) as i128;
        let penalty = overtime * cfg.penalty_per_minute;

        // 🔔 Emit SLA event
        env.events().publish(
            (EVENT_SLA_CALC, severity.clone()),
            (outage_id.clone(), symbol_short!("viol"), -penalty),
        );

        return Ok(SLAResult {
            outage_id,
            status: symbol_short!("viol"),
            mttr_minutes,
            threshold_minutes: threshold,
            amount: -penalty,
            payment_type: symbol_short!("pen"),
            rating: symbol_short!("poor"),
        });
    }

    // --------------------
    // Case 2: met → reward
    // --------------------
    let performance_ratio = (mttr_minutes * 100) / threshold;

    let (multiplier, rating) = if performance_ratio < 50 {
        (200, symbol_short!("top"))
    } else if performance_ratio < 75 {
        (150, symbol_short!("excel"))
    } else {
        (100, symbol_short!("good"))
    };

    let reward = (cfg.reward_base * (multiplier as i128)) / 100;

    // 🔔 Emit SLA event
    env.events().publish(
        (EVENT_SLA_CALC, severity.clone()),
        (outage_id.clone(), symbol_short!("met"), reward),
    );

    Ok(SLAResult {
        outage_id,
        status: symbol_short!("met"),
        mttr_minutes,
        threshold_minutes: threshold,
        amount: reward,
        payment_type: symbol_short!("rew"),
        rating,
    })
}
}

