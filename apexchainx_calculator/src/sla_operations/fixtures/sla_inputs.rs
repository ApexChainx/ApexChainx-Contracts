//! Deterministic SLA calculation input fixtures.
//!
//! Inputs mirror what the calculator takes per outage: an id, a severity and a
//! measured MTTR. Ids are generated from the severity and position, so the same
//! call always returns the same data.

use alloc::{format, string::String, vec::Vec};

/// One calculation input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlaInput {
    pub outage_id: String,
    pub severity: String,
    pub mttr_minutes: u32,
}

/// Build one input.
pub fn input(outage_id: &str, severity: &str, mttr_minutes: u32) -> SlaInput {
    SlaInput {
        outage_id: String::from(outage_id),
        severity: String::from(severity),
        mttr_minutes,
    }
}

/// A critical outage repaired well inside its 15-minute threshold.
pub fn met_input() -> SlaInput {
    input("fx-met-1", "critical", 5)
}

/// A critical outage repaired after its 15-minute threshold.
pub fn violated_input() -> SlaInput {
    input("fx-viol-1", "critical", 45)
}

/// One input per MTTR in `mttrs`, with ids `<severity>-<index>` (index from 0).
pub fn inputs_for(severity: &str, mttrs: &[u32]) -> Vec<SlaInput> {
    mttrs
        .iter()
        .enumerate()
        .map(|(i, mttr)| input(&format!("{severity}-{i}"), severity, *mttr))
        .collect()
}

/// A small mixed batch covering all four canonical severities.
pub fn sample_inputs() -> Vec<SlaInput> {
    let mut all = Vec::new();
    all.extend(inputs_for("critical", &[5, 15, 45]));
    all.extend(inputs_for("high", &[10, 30]));
    all.extend(inputs_for("medium", &[60, 90]));
    all.extend(inputs_for("low", &[30]));
    all
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn fixtures_are_deterministic_and_well_formed() {
        assert_eq!(sample_inputs(), sample_inputs());
        assert_eq!(sample_inputs().len(), 8);
        assert_eq!(inputs_for("high", &[1, 2])[1].outage_id, "high-1");
        assert!(met_input().mttr_minutes < violated_input().mttr_minutes);
    }
}
