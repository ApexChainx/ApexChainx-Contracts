//! SLA operations workspace: additive reporting, fixture and validation helpers.
//!
//! Everything here is pure and isolated from the calculator contract: no storage,
//! no events, no contract entrypoints.

pub mod compliance_score;
pub mod counter_deltas;
pub mod duration_utils;
pub mod mttr_classification;
pub mod policy_delta;
pub mod rolling_windows;
pub mod severity_weighting;
pub mod validation;
pub mod violation_rate;

pub mod reporting;

pub mod fixtures;

#[cfg(test)]
mod tests;
