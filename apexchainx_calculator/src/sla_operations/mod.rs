//! SLA operations workspace — isolated additive namespace.
//!
//! This module is the root of the `sla_operations` workspace under the
//! `apexchainx_calculator` crate. Everything here is additive: it does not
//! modify core contract entrypoints, storage keys, event schemas, or any
//! stabilised modules.
//!
//! # Module layout
//!
//! ```text
//! sla_operations/
//! ├── mod.rs              — this file; re-exports workspace modules
//! ├── fixtures/
//! │   ├── mod.rs          — fixture sub-module root
//! │   ├── policy_snapshots.rs    — SLAConfigSnapshot test fixtures
//! │   └── report_expectations.rs — SLAResult expectation fixtures
//! ├── tests/
//! │   ├── mod.rs          — test sub-module root (cfg(test) gated)
//! │   ├── policy_snapshots_fixtures_tests.rs
//! │   └── report_expectations_fixtures_tests.rs
//! └── docs/
//!     ├── README.md       — workspace architecture and contribution guide
//!     └── testing.md      — testing strategy for workspace modules
//! ```
//!
//! # Contribution boundaries
//!
//! Contributors working in this workspace MUST:
//! - Stay within `apexchainx_calculator/src/sla_operations/**`.
//! - Keep all code `no_std`-compatible (no `std::` imports).
//! - Write deterministic logic (no randomness, no I/O).
//! - Register new sub-modules only within this `mod.rs`.
//!
//! Contributors MUST NOT:
//! - Alter core storage keys or their types.
//! - Change existing event topic layouts or payload schemas.
//! - Modify `lib.rs` beyond adding a `pub mod sla_operations;` declaration.

pub mod fixtures;

#[cfg(test)]
pub mod tests;
