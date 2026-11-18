//! Reporting helpers for SLA operations outputs.
//!
//! Each submodule is independent and deterministic; see its module docs for the
//! data expectations and output format.

pub mod csv_rows;
pub mod json_models;
pub mod metadata;
pub mod status_labels;
