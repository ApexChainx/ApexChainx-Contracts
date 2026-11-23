Ok(Err(_err_val)) => SafeCallResult {
    status: CrossContractCallStatus::RecoverableError,
    raw_output: Val::from(false),
    error_symbol: Some(Symbol::new(env, "CROSS_CONTRACT_FAILURE")),
},
Err(_err) => SafeCallResult {
    status: CrossContractCallStatus::FatalError,
    raw_output: Val::from(false),
    error_symbol: Some(Symbol::new(env, "CROSS_CONTRACT_FAILURE")),
},


(result.outage_id.clone(), result.status.clone(), result.payment_type.clone(),
 result.amount, result.config_version_hash, result.recorded_at),