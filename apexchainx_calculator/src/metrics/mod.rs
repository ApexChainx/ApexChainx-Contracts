//! Contract-level operational metrics.
//!
//! This module exposes deterministic metrics that help operators
//! understand history retention behaviour and estimate storage growth.
//!
//! Metrics are read-only and never modify contract state.

pub mod history_metrics;
pub mod retention_stats;
