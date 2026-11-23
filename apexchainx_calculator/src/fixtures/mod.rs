//! Golden fixture support for protocol event compatibility.
//!
//! This module contains deterministic event fixtures used by the
//! contract test suite to detect accidental event schema regressions.
//!
//! Fixtures should only be updated when a protocol event intentionally
//! changes and the version negotiation process has been completed.

pub mod event_fixtures;
pub mod fixture_assertions;
pub mod protocol_versions;