//! Workspace-local test suite for the SLA operations namespace.
//!
//! All test modules in this directory are gated behind `#[cfg(test)]` and are
//! never compiled into the production WASM binary.
//!
//! # Test modules
//!
//! | Module                              | Covers                             |
//! |-------------------------------------|------------------------------------|
//! | `policy_snapshots_fixtures_tests`   | `fixtures::policy_snapshots`       |
//! | `report_expectations_fixtures_tests`| `fixtures::report_expectations`    |

#[cfg(test)]
pub mod policy_snapshots_fixtures_tests;

#[cfg(test)]
pub mod report_expectations_fixtures_tests;
