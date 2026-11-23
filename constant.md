// src/metadata.rs
pub fn unpause(env: &Env, caller: &Address) -> Result<(), SLAError> {
    ...
    env.storage().instance().set(&PAUSED_KEY, &false);
    env.storage().instance().remove(&PAUSE_INFO_KEY);
    env.events().publish((EVENT_UNPAUSED, EVENT_VERSION, caller.clone()), (false,));
    Ok(())
}


The documented startup check is a no-op: backends that implement the recommended deprecated_symbols scan receive an empty list forever; when a symbol is actually deprecated later, the schema endpoint will not signal it unless someone remembers to populate the list — the protocol has no teeth.
Historical interpretation is unsupported: SeverityAliasMapping exists to translate old severity symbols in historical SLAResult entries, but no code path ever emits a mapping; a backend following the docs has no way to interpret pre-rename history.
The two mechanisms are pure scaffolding: DeprecatedSymbol and SeverityAliasMapping structs, their schema fields, and the docs exist, but the contract never produces a single entry, so the features cannot be exercised end-to-end.

