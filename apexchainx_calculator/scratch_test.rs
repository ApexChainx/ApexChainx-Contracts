use soroban_sdk::{Env, Symbol};

pub fn test_sym(env: &Env) -> Symbol {
    Symbol::new(env, "migrate_done")
}
