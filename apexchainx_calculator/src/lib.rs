#![no_std]

use soroban_sdk::{
    contract, contractimpl, symbol_short, Address, Env, Map, Symbol, Vec,
};

#[contract]
pub struct SLACalculatorContract;

// Storage keys
const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

#[derive(Clone)]
pub struct SLAConfig {
    pub threshold_minutes: u32,
    pub penalty_per_minute: i128,
    pub reward_base: i128,
}

#[derive(Clone)]
pub struct SLAResult {
    pub outage_id: Symbol,
    pub status: Symbol,       // "met" or "violated"
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
    pub amount: i128,         // negative = penalty, positive = reward
    pub payment_type: Symbol, // "reward" or "penalty"
    pub rating: Symbol,       // "exceptional", "excellent", "good", "poor"
}

#[contractimpl]
impl SLACalculatorContract {
    // ------------------------
    // Init & Admin
    // ------------------------

    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN_KEY) {
            panic!("Already initialized");
        }

        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage().instance().set(&CONFIG_KEY, &Map::<Symbol, SLAConfig>::new(&env));
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("Not initialized")
    }

    // ------------------------
    // Config
    // ------------------------

    pub fn set_config(
        env: Env,
        caller: Address,
        severity: Symbol,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) {
        let admin: Address = env.storage().instance().get(&ADMIN_KEY).unwrap();
        if caller != admin {
            panic!("Only admin can update config");
        }

        let mut configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .unwrap();

        let cfg = SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        };

        configs.set(severity, cfg);
        env.storage().instance().set(&CONFIG_KEY, &configs);
    }

    pub fn get_config(env: Env, severity: Symbol) -> SLAConfig {
        let configs: Map<Symbol, SLAConfig> = env
            .storage()
            .instance()
            .get(&CONFIG_KEY)
            .unwrap();

        configs.get(severity).expect("Config not found")
    }

    // ------------------------
    // SLA Calculation
    // ------------------------

    pub fn calculate_sla(
        env: Env,
        outage_id: Symbol,
        severity: Symbol,
        mttr_minutes: u32,
    ) -> SLAResult {
        let cfg = Self::get_config(env.clone(), severity.clone());

        let threshold = cfg.threshold_minutes;

        // ------------------------
        // Case 1: Violated -> Penalty
        // ------------------------
        if mttr_minutes > threshold {
            let overtime = (mttr_minutes - threshold) as i128;
            let penalty = overtime * cfg.penalty_per_minute;

            return SLAResult {
                outage_id,
                status: symbol_short!("violated"),
                mttr_minutes,
                threshold_minutes: threshold,
                amount: -penalty,
                payment_type: symbol_short!("penalty"),
                rating: symbol_short!("poor"),
            };
        }

        // ------------------------
        // Case 2: Met -> Reward
        // ------------------------
        let performance_ratio = (mttr_minutes * 100) / threshold;

        let (multiplier, rating) = if performance_ratio < 50 {
            (200, symbol_short!("exceptional")) // 2.0x
        } else if performance_ratio < 75 {
            (150, symbol_short!("excellent")) // 1.5x
        } else {
            (100, symbol_short!("good")) // 1.0x
        };

        let reward = (cfg.reward_base * (multiplier as i128)) / 100;

        SLAResult {
            outage_id,
            status: symbol_short!("met"),
            mttr_minutes,
            threshold_minutes: threshold,
            amount: reward,
            payment_type: symbol_short!("reward"),
            rating,
        }
    }
}
