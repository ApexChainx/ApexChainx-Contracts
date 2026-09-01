//! SC-W5-037 – Event payload size optimization without semantic loss.
//!
//! This module provides event-payload helpers that keep on-chain events
//! compact while preserving full semantic meaning.
//!
//! # Optimization Strategies
//!
//! 1. **Deterministic `payment_type` derivation** — "viol" → "pen", "met" → "rew"
//!    gives consumers a self-consistent way to cross-check the payment type
//!    against the SLA status. The canonical event payload (see
//!    [`crate::event_schema`]) still carries `payment_type` explicitly so a
//!    single event is fully self-contained for reconciliation (per the
//!    self-contained-payload decision), so consumers never need to derive it.
//! 2. **Compact field ordering** — minimises Soroban encoding overhead by
//!    grouping fields by type (Symbols together, integers together).
//!
//! # Validations
//!
//! | Function | Purpose |
//! |----------|---------|
//! | `derive_payment_type` | Deterministic payment type from SLA status |
//! | `is_valid_status` | Check if status is "met" or "viol" |
//! | `is_consistent_payment` | Verify payment_type matches status |
//! | `is_valid_rating` | Check if rating is "top"/"excel"/"good"/"poor" |
//!
//! # Backend Guidance
//!
//! Backend consumers can deterministically cross-check `payment_type` against
//! `status` using the derivation rule below. No derivation *round-trip* is
//! required to consume events — the canonical payload carries both fields
//! explicitly — but the two must agree:
//! - `status == "met"` → `payment_type == "rew"` (reward)
//! - `status == "viol"` → `payment_type == "pen"` (penalty)

use soroban_sdk::{symbol_short, Symbol};

/// Derive payment type from SLA status.
///
/// Given an SLA status symbol, returns the corresponding payment type:
/// - `"met"` → `"rew"` (reward)
/// - `"viol"` → `"pen"` (penalty)
///
/// This is a pure function with no side effects.
pub fn derive_payment_type(status: &Symbol) -> Symbol {
    if *status == symbol_short!("viol") {
        symbol_short!("pen")
    } else {
        symbol_short!("rew")
    }
}

/// Returns true if the status is a valid SLA outcome symbol.
pub fn is_valid_status(status: &Symbol) -> bool {
    *status == symbol_short!("met") || *status == symbol_short!("viol")
}

/// Returns true if the payment type is consistent with the given status.
pub fn is_consistent_payment(status: &Symbol, payment_type: &Symbol) -> bool {
    derive_payment_type(status) == *payment_type
}

/// Returns true if the rating is a valid tier symbol.
pub fn is_valid_rating(rating: &Symbol) -> bool {
    *rating == symbol_short!("top")
        || *rating == symbol_short!("excel")
        || *rating == symbol_short!("good")
        || *rating == symbol_short!("poor")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_payment_from_met() {
        assert_eq!(derive_payment_type(&symbol_short!("met")), symbol_short!("rew"));
    }

    #[test]
    fn test_derive_payment_from_viol() {
        assert_eq!(derive_payment_type(&symbol_short!("viol")), symbol_short!("pen"));
    }

    #[test]
    fn test_valid_statuses() {
        assert!(is_valid_status(&symbol_short!("met")));
        assert!(is_valid_status(&symbol_short!("viol")));
        assert!(!is_valid_status(&symbol_short!("unknown")));
    }

    #[test]
    fn test_consistent_payment() {
        assert!(is_consistent_payment(
            &symbol_short!("met"),
            &symbol_short!("rew")
        ));
        assert!(is_consistent_payment(
            &symbol_short!("viol"),
            &symbol_short!("pen")
        ));
        assert!(!is_consistent_payment(
            &symbol_short!("met"),
            &symbol_short!("pen")
        ));
    }

    #[test]
    fn test_valid_ratings() {
        assert!(is_valid_rating(&symbol_short!("top")));
        assert!(is_valid_rating(&symbol_short!("excel")));
        assert!(is_valid_rating(&symbol_short!("good")));
        assert!(is_valid_rating(&symbol_short!("poor")));
        assert!(!is_valid_rating(&symbol_short!("unknown")));
    }
}
