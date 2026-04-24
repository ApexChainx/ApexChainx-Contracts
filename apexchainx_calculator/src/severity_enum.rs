use soroban_sdk::{symbol_short, Env, Symbol, Vec};

pub fn supported_severities(env: &Env) -> Vec<Symbol> {
    let mut out = Vec::new(env);
    out.push_back(symbol_short!("critical"));
    out.push_back(symbol_short!("high"));
    out.push_back(symbol_short!("medium"));
    out.push_back(symbol_short!("low"));
    out
}

pub fn is_known_severity(env: &Env, severity: &Symbol) -> bool {
    supported_severities(env).contains(severity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{symbol_short, Env};

    #[test]
    fn test_severity_list_is_deterministic() {
        let env = Env::default();
        let first  = supported_severities(&env);
        let second = supported_severities(&env);
        assert_eq!(first, second);
    }

    #[test]
    fn test_severity_list_contains_expected_values() {
        let env = Env::default();
        let sevs = supported_severities(&env);
        assert_eq!(sevs.len(), 4);
        assert!(sevs.contains(&symbol_short!("critical")));
        assert!(sevs.contains(&symbol_short!("high")));
        assert!(sevs.contains(&symbol_short!("medium")));
        assert!(sevs.contains(&symbol_short!("low")));
    }

    #[test]
    fn test_unknown_severity_is_rejected() {
        let env = Env::default();
        assert!(!is_known_severity(&env, &symbol_short!("ultra")));
    }
}
