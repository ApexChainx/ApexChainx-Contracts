use soroban_sdk::{Env, Symbol, symbol_short};

pub struct DeploymentPolicy;

impl DeploymentPolicy {
    pub const REQUIRED_PROTOCOL_VERSION: u32 = 1;
    pub const DEPLOYMENT_TAG: Symbol = symbol_short!("v1_release");

    pub fn verify_deployment_compatibility(env: &Env) -> bool {
        // Ensure ledger environment supports required minimum protocol version
        let current_protocol = env.ledger().protocol_version();
        current_protocol >= 1
    }
}