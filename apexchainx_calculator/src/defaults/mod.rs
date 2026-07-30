//! Default value policies for optional storage.
//!
//! Public query methods should never panic because an optional
//! storage value has not yet been initialized.
//!
//! This module provides deterministic fallback values while keeping
//! write operations unchanged.

pub mod default_policy;
pub mod query_defaults;
pub mod storage_fallback;