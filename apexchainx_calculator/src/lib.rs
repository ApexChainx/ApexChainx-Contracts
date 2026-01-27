#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol,
};

#[contract]
pub struct SLACalculatorContract;

const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

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
    pub status: Symbol,       // "met" or "violated"
    pub mttr_minutes: u32,
    pub threshold_minutes: u32,
    pub amount: i128,         // negative = penalty, positive = reward
    pub payment_type: Symbol, // "reward" or "penalty"
    pub rating: Symbol,       // "exceptional", "excellent", "good", "poor"
}

// --------------------
// Contract impl
// --------------------

#[contractimpl]
impl SLACalculatorContract {
    // --------------------
    // Init & Admin
    // --------------------

    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN_KEY) {
            panic!("Already initialized");
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
    }

    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("Not initialized")
    }

    // --------------------
    // Internal helper
    // --------------------

    fn require_admin(env: &Env, caller: &Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("Not initialized");

        if caller != &admin {
            panic!("Unauthorized: admin only");
        }
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
    ) {
        Self::require_admin(&env, &caller);

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

    // --------------------
    // SLA calculation
    // --------------------

    pub fn calculate_sla(
        env: Env,
        outage_id: Symbol,
        severity: Symbol,
        mttr_minutes: u32,
    ) -> SLAResult {
        let cfg = Self::get_config(env.clone(), severity.clone());
        let threshold = cfg.threshold_minutes;

        // --------------------
        // Case 1: violated → penalty
        // --------------------
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

        // --------------------
        // Case 2: met → reward
        // --------------------
        let performance_ratio = (mttr_minutes * 100) / threshold;

        let (multiplier, rating) = if performance_ratio < 50 {
            (200, symbol_short!("exceptional"))
        } else if performance_ratio < 75 {
            (150, symbol_short!("excellent"))
        } else {
            (100, symbol_short!("good"))
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
