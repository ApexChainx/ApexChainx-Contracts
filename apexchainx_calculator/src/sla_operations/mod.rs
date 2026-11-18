//! SLA operations workspace: additive reporting, fixture and validation helpers.
//!
//! Everything here is pure and isolated from the calculator contract: no storage,
//! no events, no contract entrypoints.

pub mod reporting;
pub mod validation;

#[cfg(test)]
mod tests;
