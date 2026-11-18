use crate::sla_operations::validation::severity_names::{
    normalize_severity_name, parse_severity_name, validate_severity_name, SeverityName,
    SeverityNameError,
};

#[test]
fn canonical_names_parse_with_any_ascii_case() {
    assert_eq!(parse_severity_name("low"), Some(SeverityName::Low));
    assert_eq!(parse_severity_name("LOW"), Some(SeverityName::Low));
    assert_eq!(parse_severity_name("Medium"), Some(SeverityName::Medium));
    assert_eq!(parse_severity_name("HIGH"), Some(SeverityName::High));
    assert_eq!(parse_severity_name("Critical"), Some(SeverityName::Critical));
}

#[test]
fn aliases_resolve_to_canonical_names() {
    assert_eq!(parse_severity_name("med"), Some(SeverityName::Medium));
    assert_eq!(parse_severity_name("crit"), Some(SeverityName::Critical));
    assert_eq!(normalize_severity_name("MED"), Some("medium"));
    assert_eq!(normalize_severity_name("CRIT"), Some("critical"));
}

#[test]
fn validation_rejects_empty_and_unknown_names() {
    assert_eq!(validate_severity_name(""), Err(SeverityNameError::Empty));
    assert_eq!(
        validate_severity_name("urgent"),
        Err(SeverityNameError::Unknown)
    );
    assert_eq!(
        validate_severity_name(" medium "),
        Err(SeverityNameError::Unknown)
    );
}

#[test]
fn validation_returns_the_canonical_variant() {
    assert_eq!(validate_severity_name("LOW"), Ok(SeverityName::Low));
    assert_eq!(validate_severity_name("med"), Ok(SeverityName::Medium));
    assert_eq!(validate_severity_name("CRIT"), Ok(SeverityName::Critical));
}

#[test]
fn canonical_labels_are_stable() {
    assert_eq!(SeverityName::Low.canonical(), "low");
    assert_eq!(SeverityName::Medium.canonical(), "medium");
    assert_eq!(SeverityName::High.canonical(), "high");
    assert_eq!(SeverityName::Critical.canonical(), "critical");
}
