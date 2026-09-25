//! Named constants for the four canonical severity symbols (Issue #658).
//!
//! Previously every site repeated `symbol_short!("critical")` etc. inline.
//! These constants are the single source of truth; all validation, telemetry,
//! hash computation, and fixture code should reference them instead of
//! spelling the literals out.

use soroban_sdk::{symbol_short, Symbol};

/// Canonical severity: critical (highest urgency, strictest SLA).
pub const SEV_CRITICAL: Symbol = symbol_short!("critical");
/// Canonical severity: high.
pub const SEV_HIGH: Symbol = symbol_short!("high");
/// Canonical severity: medium.
pub const SEV_MEDIUM: Symbol = symbol_short!("medium");
/// Canonical severity: low (most lenient SLA).
pub const SEV_LOW: Symbol = symbol_short!("low");

/// The four canonical severities in priority order (critical → low).
/// Use this slice wherever the ordered set is needed instead of rebuilding it inline.
pub const CANONICAL_SEVERITIES: [Symbol; 4] = [SEV_CRITICAL, SEV_HIGH, SEV_MEDIUM, SEV_LOW];