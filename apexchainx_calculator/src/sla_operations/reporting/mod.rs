//! Reporting helpers for SLA operations outputs.
//!
//! Each submodule is independent and deterministic; see its module docs for the
//! data expectations and output format.

pub mod history_export;
pub mod outage_analytics;
pub mod settlement_summary;
pub mod telemetry_snapshot;
