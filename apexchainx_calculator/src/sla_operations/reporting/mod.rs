//! Reporting helpers for SLA operations outputs.
//!
//! Each submodule is independent and deterministic; see its module docs for the
//! data expectations and output format.

pub mod outage_analytics;
pub mod settlement_summary;
pub mod sla_summary;
pub mod telemetry_snapshot;

pub mod compliance_report;
pub mod history_export;
pub mod severity_distribution;
pub mod time_series;

pub mod csv_rows;
pub mod json_models;
pub mod metadata;
pub mod status_labels;

pub mod deltas;
pub mod health_indicators;
pub use deltas::{compute_delta, compute_deltas, MetricDelta, Trend};
pub use health_indicators::{HealthIndicators, HealthLevel, SlaMetrics};
