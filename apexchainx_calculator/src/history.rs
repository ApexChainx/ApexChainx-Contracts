use soroban_sdk::{Address, Env, Symbol, Vec};

use crate::{
    SLAError, SLAResult, PrunePolicy,
    HISTORY_KEY, RETENTION_LIMIT_KEY, MAX_HISTORY_SIZE, MAX_CRON_EXPR_LEN, PRUNE_POLICY_KEY, EVENT_VERSION, EVENT_PRUNED, EVENT_PRUNED_AGE, EVENT_PRUNED_POLICY,
};

pub fn get_history(env: &Env) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env)))
}

pub fn prune_history(env: &Env, caller: &Address, keep_latest: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let len = history.len();

    if len > keep_latest {
        let remove_count = len - keep_latest;
        let mut new_history = Vec::new(env);

        for i in remove_count..len {
            new_history.push_back(history.get(i).unwrap());
        }

        env.storage().instance().set(&HISTORY_KEY, &new_history);
        env.events()
            .publish((EVENT_PRUNED, EVENT_VERSION, caller.clone()), (remove_count, keep_latest));
    }

    Ok(())
}

pub fn prune_history_by_age(env: &Env, caller: &Address, min_age_seconds: u64) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let now = env.ledger().timestamp();
    let cutoff = now.saturating_sub(min_age_seconds);

    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));

    let mut new_history = Vec::new(env);
    let mut removed: u32 = 0;

    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.recorded_at >= cutoff {
            new_history.push_back(entry);
        } else {
            removed += 1;
        }
    }

    if removed > 0 {
        let kept = new_history.len();
        env.storage().instance().set(&HISTORY_KEY, &new_history);
        env.events()
            .publish((EVENT_PRUNED_AGE, EVENT_VERSION, caller.clone()), (removed, kept));
    }

    Ok(())
}

pub fn get_history_page(env: &Env, offset: u32, limit: u32) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let len = history.len();
    let mut page = Vec::new(env);
    if offset >= len || limit == 0 {
        return Ok(page);
    }
    let end = (offset + limit).min(len);
    for i in offset..end {
        page.push_back(history.get(i).unwrap());
    }
    Ok(page)
}

pub fn get_history_by_outage(env: &Env, outage_id: Symbol) -> Result<Vec<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let mut matches = Vec::new(env);
    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.outage_id == outage_id {
            matches.push_back(entry);
        }
    }
    Ok(matches)
}

pub fn get_latest_by_outage(env: &Env, outage_id: Symbol) -> Result<Option<SLAResult>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));
    let mut latest: Option<SLAResult> = None;
    for i in 0..history.len() {
        let entry = history.get(i).unwrap();
        if entry.outage_id == outage_id {
            latest = Some(entry);
        }
    }
    Ok(latest)
}

pub fn get_config_count(env: &Env) -> Result<u32, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    let configs: soroban_sdk::Map<Symbol, crate::SLAConfig> = env
        .storage()
        .instance()
        .get(&crate::CONFIG_KEY)
        .ok_or(SLAError::NotInitialized)?;
    Ok(configs.len())
}

pub fn set_retention_limit(env: &Env, caller: &Address, limit: u32) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;
    if limit == 0 || limit > MAX_HISTORY_SIZE {
        return Err(SLAError::RetentionLimitOutOfRange);
    }
    env.storage().instance().set(&RETENTION_LIMIT_KEY, &limit);
    Ok(())
}

pub fn get_retention_limit(env: &Env) -> Result<u32, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env
        .storage()
        .instance()
        .get(&RETENTION_LIMIT_KEY)
        .unwrap_or(MAX_HISTORY_SIZE))
}

pub fn set_prune_policy(env: &Env, caller: &Address, policy: &PrunePolicy) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    if policy.cron_expr.len() > MAX_CRON_EXPR_LEN as u32 {
        return Err(SLAError::InvalidInput);
    }

    env.storage().instance().set(&PRUNE_POLICY_KEY, policy);
    Ok(())
}

pub fn get_prune_policy(env: &Env) -> Result<Option<PrunePolicy>, SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    Ok(env.storage().instance().get(&PRUNE_POLICY_KEY))
}

pub fn apply_prune_policy(env: &Env, caller: &Address) -> Result<(), SLAError> {
    crate::SLACalculatorContract::check_version(env)?;
    crate::SLACalculatorContract::require_admin(env, caller)?;

    let policy: PrunePolicy = env
        .storage()
        .instance()
        .get(&PRUNE_POLICY_KEY)
        .ok_or(SLAError::NoPrunePolicy)?;

    let mut history: Vec<SLAResult> = env
        .storage()
        .instance()
        .get(&HISTORY_KEY)
        .unwrap_or_else(|| Vec::new(env));

    let mut total_removed: u32 = 0;

    if policy.keep_latest > 0 && history.len() > policy.keep_latest {
        let remove_count = history.len() - policy.keep_latest;
        total_removed += remove_count;
        let mut new_history = Vec::new(env);
        for i in remove_count..history.len() {
            new_history.push_back(history.get(i).unwrap());
        }
        history = new_history;
    }

    if policy.max_age_seconds > 0 {
        let now = env.ledger().timestamp();
        let cutoff = now.saturating_sub(policy.max_age_seconds);
        let mut new_history = Vec::new(env);
        for i in 0..history.len() {
            let entry = history.get(i).unwrap();
            if entry.recorded_at >= cutoff {
                new_history.push_back(entry);
            } else {
                total_removed += 1;
            }
        }
        history = new_history;
    }

    if total_removed > 0 {
        let kept = history.len();
        env.storage().instance().set(&HISTORY_KEY, &history);
        env.events()
            .publish((EVENT_PRUNED_POLICY, EVENT_VERSION, caller.clone()), (total_removed, kept));
    }

    Ok(())
}
