#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // ... existing errors
    /// Operation would cause a division by zero.
    DivisionByZero,
    /// Emitted when `calculate_sla` is called with a `payment_type` that
    /// is inconsistent with the calculated `status`.
    InconsistentPaymentStatus,
}


pub fn calculate_sla(
    env: Env,
    outage_id: Symbol,
    severity: Symbol,
    mttr_minutes: u32,
) -> Result<SLAResult, Error> {
    let operator = Self::get_operator(&env)?;
    operator.require_auth();

    if Self::is_paused(&env) {
        return Err(Error::Paused);
    }

    let config = Self::get_config(&env, &severity)?;
    let result = calculation::compute_result(&env, outage_id, &severity, mttr_minutes, &config);

    // Enforce consistency between status and payment_type.
    if !payload_optimizer::is_consistent_payment(&result.status, &result.payment_type) {
        return Err(Error::InconsistentPaymentStatus);
    }

    if let Some(new_stats) =
        metrics::increment_stats(&env, &result.status, result.amount, &severity)
    {
        new_stats.write_to(&env);
    }

    // Publish the primary 'sla_calc' event
    event::publish_sla_calc(&env, &result, &severity);

    // Publish the 'set_int' event for settlement
    event::publish_settlement_intent(&env, &result, &severity);

    history::append_and_prune(&env, result.clone());

    Ok(result)
}