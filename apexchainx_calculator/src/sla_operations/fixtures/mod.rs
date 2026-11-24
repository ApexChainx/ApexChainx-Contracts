//! Workspace-local test fixtures for SLA operations.
//!
//! Each sub-module provides deterministic, pre-constructed data values used by
//! the workspace test suite. Fixtures are pure data: they carry no side effects
//! and do not interact with on-chain storage.
//!
//! # Available fixtures
//!
//! | Module                | Contents                                        |
//! |-----------------------|-------------------------------------------------|
//! | `policy_snapshots`    | [`SLAConfigSnapshot`] instances for test cases  |
//! | `report_expectations` | [`SLAResult`] expectation structs for test cases|

pub mod policy_snapshots;
pub mod report_expectations;
