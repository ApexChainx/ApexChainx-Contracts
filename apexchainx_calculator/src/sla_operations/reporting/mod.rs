//! Reporting helpers for SLA operations outputs.
//!
//! Each submodule is independent and deterministic; see its module docs for the
//! data expectations and output format.

pub mod deltas;
pub mod health_indicators;

pub use deltas::{compute_delta, compute_deltas, MetricDelta, Trend};
pub use health_indicators::{HealthIndicators, HealthLevel, SlaMetrics};
