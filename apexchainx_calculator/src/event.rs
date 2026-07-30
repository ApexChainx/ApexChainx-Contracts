use soroban_sdk::{contracttype, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalculationExecutedEventV1 {
    pub input_key: Symbol,
    pub input_value: i128,
    pub result_value: i128,
    pub timestamp: u64,
}

pub struct EventPublisher;

impl EventPublisher {
    /// Publishes calculation execution event while preserving strict field ordering
    pub fn publish_calculation_executed(
        env: &Env,
        topic: Symbol,
        input_key: Symbol,
        input_value: i128,
        result_value: i128,
        timestamp: u64,
    ) {
        let payload = CalculationExecutedEventV1 {
            input_key,
            input_value,
            result_value,
            timestamp,
        };
        
        env.events().publish((topic, input_key), payload);
    }
}

use soroban_sdk::{contracttype, Env, Symbol};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalculationExecutedEventV1 {
    pub input_key: Symbol,
    pub input_value: i128,
    pub result_value: i128,
    pub timestamp: u64,
}

pub struct EventPublisher;

impl EventPublisher {
    /// Publishes calculation execution event while preserving strict field ordering
    pub fn publish_calculation_executed(
        env: &Env,
        topic: Symbol,
        input_key: Symbol,
        input_value: i128,
        result_value: i128,
        timestamp: u64,
    ) {
        let payload = CalculationExecutedEventV1 {
            input_key,
            input_value,
            result_value,
            timestamp,
        };
        
        env.events().publish((topic, input_key), payload);
    }
}