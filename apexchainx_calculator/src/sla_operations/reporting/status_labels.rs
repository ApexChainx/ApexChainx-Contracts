//! Operator-facing labels and numeric codes for SLA states.
//!
//! Maps the short symbols used by the calculator (`"met"`, `"viol"`, `"rew"`,
//! `"pen"`, `"top"`, ...) to stable display labels. Unknown input never panics;
//! it maps to an explicit "Unknown"/"Unrated" label so reports stay total.

/// Outcome of an SLA calculation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlaState {
    Met,
    Violated,
    Unknown,
}

impl SlaState {
    /// Parse the calculator's status symbol.
    pub fn from_symbol(symbol: &str) -> SlaState {
        match symbol {
            "met" => SlaState::Met,
            "viol" => SlaState::Violated,
            _ => SlaState::Unknown,
        }
    }

    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            SlaState::Met => "SLA Met",
            SlaState::Violated => "SLA Violated",
            SlaState::Unknown => "Unknown",
        }
    }

    /// Stable numeric code: `0` met, `1` violated, `255` unknown.
    pub fn code(self) -> u8 {
        match self {
            SlaState::Met => 0,
            SlaState::Violated => 1,
            SlaState::Unknown => 255,
        }
    }
}

/// Label for a payment type symbol (`"rew"` / `"pen"`).
pub fn payment_label(symbol: &str) -> &'static str {
    match symbol {
        "rew" => "Reward",
        "pen" => "Penalty",
        _ => "Unknown",
    }
}

/// Label for a performance rating symbol.
pub fn rating_label(symbol: &str) -> &'static str {
    match symbol {
        "top" => "Top performer",
        "excel" => "Excellent",
        "good" => "Good",
        "poor" => "Poor",
        _ => "Unrated",
    }
}

/// Label for a canonical severity symbol.
pub fn severity_label(symbol: &str) -> &'static str {
    match symbol {
        "critical" => "Critical",
        "high" => "High",
        "medium" => "Medium",
        "low" => "Low",
        _ => "Unknown severity",
    }
}

#[cfg(test)]
mod label_tests {
    use super::*;

    #[test]
    fn states_map_to_labels_and_codes() {
        assert_eq!(SlaState::from_symbol("met").label(), "SLA Met");
        assert_eq!(SlaState::from_symbol("viol").code(), 1);
        assert_eq!(SlaState::from_symbol("???"), SlaState::Unknown);
        assert_eq!(SlaState::Unknown.code(), 255);
    }

    #[test]
    fn symbols_map_to_labels() {
        assert_eq!(payment_label("rew"), "Reward");
        assert_eq!(payment_label("x"), "Unknown");
        assert_eq!(rating_label("excel"), "Excellent");
        assert_eq!(rating_label(""), "Unrated");
        assert_eq!(severity_label("critical"), "Critical");
        assert_eq!(severity_label("nope"), "Unknown severity");
    }
}
