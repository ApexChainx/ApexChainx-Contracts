//! SLA operations workspace: additive reporting, fixture and validation helpers.
//!
//! Everything here is pure and isolated from the calculator contract: no storage,
//! no events, no contract entrypoints.

pub mod duration_utils;
pub mod validation;
pub mod violation_rate;
pub mod counter_deltas;
pub mod mttr_classification;

pub mod reporting;
