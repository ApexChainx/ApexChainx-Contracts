//! Reporting helpers for SLA operations outputs.
//!
//! Each submodule is independent and deterministic; see its module docs for the
//! data expectations and output format.

pub mod compliance_report;
pub mod csv_rows;
pub mod severity_distribution;
pub mod time_series;
