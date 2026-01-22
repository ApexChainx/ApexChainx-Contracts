#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Critical = 1,
    High = 2,
    Medium = 3,
    Low = 4,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SLAConfig {
    pub threshold_minutes: u32,
    pub penalty_per_minute: i128,
    pub reward_base: i128,
}

#[contract]
pub struct SLACalculatorContract;

const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

// Storage keys for SLA configurations
const CRITICAL_CONFIG_KEY: Symbol = symbol_short!("CRIT_CFG");
const HIGH_CONFIG_KEY: Symbol = symbol_short!("HIGH_CFG");
const MEDIUM_CONFIG_KEY: Symbol = symbol_short!("MED_CFG");
const LOW_CONFIG_KEY: Symbol = symbol_short!("LOW_CFG");

#[contractimpl]
impl SLACalculatorContract {

    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN_KEY) {
            panic!("Already initialized");
        }

        env.storage().instance().set(&ADMIN_KEY, &admin);

        // Initialize default SLA configurations
        let critical_config = SLAConfig {
            threshold_minutes: 15,    // 15 minutes for critical outages
            penalty_per_minute: 10_0000000,  // $10 per minute penalty
            reward_base: 500_0000000,       // $500 base reward
        };

        let high_config = SLAConfig {
            threshold_minutes: 30,    // 30 minutes for high severity
            penalty_per_minute: 5_0000000,   // $5 per minute penalty
            reward_base: 200_0000000,        // $200 base reward
        };

        let medium_config = SLAConfig {
            threshold_minutes: 60,    // 60 minutes for medium severity
            penalty_per_minute: 2_0000000,   // $2 per minute penalty
            reward_base: 100_0000000,        // $100 base reward
        };

        let low_config = SLAConfig {
            threshold_minutes: 120,   // 120 minutes for low severity
            penalty_per_minute: 1_0000000,   // $1 per minute penalty
            reward_base: 50_0000000,         // $50 base reward
        };

        env.storage().instance().set(&CRITICAL_CONFIG_KEY, &critical_config);
        env.storage().instance().set(&HIGH_CONFIG_KEY, &high_config);
        env.storage().instance().set(&MEDIUM_CONFIG_KEY, &medium_config);
        env.storage().instance().set(&LOW_CONFIG_KEY, &low_config);
    }


    pub fn get_admin(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("Not initialized")
    }


    pub fn get_config(env: Env, severity: Severity) -> SLAConfig {
        let config_key = match severity {
            Severity::Critical => CRITICAL_CONFIG_KEY,
            Severity::High => HIGH_CONFIG_KEY,
            Severity::Medium => MEDIUM_CONFIG_KEY,
            Severity::Low => LOW_CONFIG_KEY,
        };

        env.storage()
            .instance()
            .get(&config_key)
            .expect("SLA config not found - contract may not be initialized")
    }

    pub fn update_config(
        env: Env,
        severity: Severity,
        threshold_minutes: u32,
        penalty_per_minute: i128,
        reward_base: i128,
    ) {
        // Only admin can update configurations
        let admin: Address = env.storage()
            .instance()
            .get(&ADMIN_KEY)
            .expect("Contract not initialized");

        admin.require_auth();

        let config_key = match severity {
            Severity::Critical => CRITICAL_CONFIG_KEY,
            Severity::High => HIGH_CONFIG_KEY,
            Severity::Medium => MEDIUM_CONFIG_KEY,
            Severity::Low => LOW_CONFIG_KEY,
        };

        let new_config = SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        };

        env.storage().instance().set(&config_key, &new_config);
    }


    pub fn calculate_sla(
        _env: Env,
        _outage_id: Symbol,
        _severity: Symbol,
        _mttr_minutes: u32,
    ) -> Symbol {
    
        symbol_short!("TODO")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Env, IntoVal};

    #[test]
    fn test_initialize_and_get_admin() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);

        let admin = soroban_sdk::Address::generate(&env);

        client.initialize(&admin);

        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, admin);
    }

    #[test]
    #[should_panic]
    fn test_double_initialize_panics() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);

        let admin = soroban_sdk::Address::generate(&env);

        client.initialize(&admin);
        client.initialize(&admin); // should panic
    }

    #[test]
    fn test_default_config_initialization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);

        let admin = soroban_sdk::Address::generate(&env);
        client.initialize(&admin);

        // Test Critical severity config
        let critical_config = client.get_config(&Severity::Critical);
        assert_eq!(critical_config.threshold_minutes, 15);
        assert_eq!(critical_config.penalty_per_minute, 10_0000000);
        assert_eq!(critical_config.reward_base, 500_0000000);

        // Test High severity config
        let high_config = client.get_config(&Severity::High);
        assert_eq!(high_config.threshold_minutes, 30);
        assert_eq!(high_config.penalty_per_minute, 5_0000000);
        assert_eq!(high_config.reward_base, 200_0000000);

        // Test Medium severity config
        let medium_config = client.get_config(&Severity::Medium);
        assert_eq!(medium_config.threshold_minutes, 60);
        assert_eq!(medium_config.penalty_per_minute, 2_0000000);
        assert_eq!(medium_config.reward_base, 100_0000000);

        // Test Low severity config
        let low_config = client.get_config(&Severity::Low);
        assert_eq!(low_config.threshold_minutes, 120);
        assert_eq!(low_config.penalty_per_minute, 1_0000000);
        assert_eq!(low_config.reward_base, 50_0000000);
    }

    #[test]
    fn test_admin_can_update_config() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);

        let admin = soroban_sdk::Address::generate(&env);
        client.initialize(&admin);

        // Mock auth for admin updating config
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &admin,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_config",
                args: (
                    Severity::Critical,
                    20_u32,
                    15_0000000_i128,
                    750_0000000_i128,
                ).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        // Update critical config as admin
        client.update_config(
            &Severity::Critical,
            &20,           // new threshold: 20 minutes
            &15_0000000,   // new penalty: $15 per minute
            &750_0000000,  // new reward: $750 base
        );

        // Verify the update
        let updated_config = client.get_config(&Severity::Critical);
        assert_eq!(updated_config.threshold_minutes, 20);
        assert_eq!(updated_config.penalty_per_minute, 15_0000000);
        assert_eq!(updated_config.reward_base, 750_0000000);

        // Verify other configs remain unchanged
        let high_config = client.get_config(&Severity::High);
        assert_eq!(high_config.threshold_minutes, 30);
        assert_eq!(high_config.penalty_per_minute, 5_0000000);
    }

    #[test]
    #[should_panic]
    fn test_non_admin_cannot_update_config() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SLACalculatorContract);
        let client = SLACalculatorContractClient::new(&env, &contract_id);

        let admin = soroban_sdk::Address::generate(&env);
        client.initialize(&admin);

        // Create a different address that's not the admin
        let non_admin = soroban_sdk::Address::generate(&env);

        // Mock auth for non-admin trying to update config
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &non_admin,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &contract_id,
                fn_name: "update_config",
                args: (
                    Severity::Critical,
                    25_u32,
                    20_0000000_i128,
                    1000_0000000_i128,
                ).into_val(&env),
                sub_invokes: &[],
            },
        }]);

        // This should panic because non-admin is trying to update
        client.update_config(
            &Severity::Critical,
            &25,
            &20_0000000,
            &1000_0000000,
        );
    }
}