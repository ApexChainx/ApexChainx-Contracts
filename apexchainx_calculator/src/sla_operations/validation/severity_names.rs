//! Validation and normalization helpers for SLA severity names.

/// Canonical severity names supported by the workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeverityName {
    /// Low severity.
    Low,
    /// Medium severity.
    Medium,
    /// High severity.
    High,
    /// Critical severity.
    Critical,
}

impl SeverityName {
    /// Returns the canonical lowercase name.
    #[must_use]
    pub const fn canonical(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Errors produced while validating severity names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeverityNameError {
    /// The name is empty.
    Empty,
    /// The name does not match a known severity or alias.
    Unknown,
}

/// Parses a severity name or alias without allocating.
#[must_use]
pub fn parse_severity_name(value: &str) -> Option<SeverityName> {
    if value.eq_ignore_ascii_case("low") {
        Some(SeverityName::Low)
    } else if value.eq_ignore_ascii_case("medium") || value.eq_ignore_ascii_case("med") {
        Some(SeverityName::Medium)
    } else if value.eq_ignore_ascii_case("high") {
        Some(SeverityName::High)
    } else if value.eq_ignore_ascii_case("critical") || value.eq_ignore_ascii_case("crit") {
        Some(SeverityName::Critical)
    } else {
        None
    }
}

/// Returns the canonical name for a recognized severity input.
#[must_use]
pub fn normalize_severity_name(value: &str) -> Option<&'static str> {
    parse_severity_name(value).map(SeverityName::canonical)
}

/// Validates a severity name and returns its canonical variant.
#[must_use]
pub fn validate_severity_name(value: &str) -> Result<SeverityName, SeverityNameError> {
    if value.is_empty() {
        return Err(SeverityNameError::Empty);
    }

    parse_severity_name(value).ok_or(SeverityNameError::Unknown)
}
