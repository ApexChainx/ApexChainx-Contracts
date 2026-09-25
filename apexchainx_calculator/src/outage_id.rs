//! Outage ID normalization (Issue #662).
//!
//! Previously, callers passed `outage_id` as a raw `Symbol` with no central
//! normalization rule. Different producers could submit `SF-001`, `SF001`, or
//! `sf001` for the same incident, splitting history entries, wasting the
//! recalc budget, and corrupting backend ID joins.
//!
//! # Canonical form
//!
//! An outage ID is canonical when it matches the pattern accepted by
//! [`validate_outage_id`]. The rule is deliberately simple to stay inside
//! Soroban's `Symbol` constraints (max 32 ASCII chars, a-z 0-9 _ only after
//! the Soroban Symbol encoding):
//!
//! - Length: 1–32 characters.
//! - Allowed characters: ASCII alphanumeric (`A-Z`, `a-z`, `0-9`) and
//!   hyphens (`-`). Underscores and other punctuation are **not** allowed.
//! - The value is treated as **case-sensitive** by the contract — `SF-001`
//!   and `sf-001` are distinct IDs. Callers are responsible for normalizing
//!   case before submission. The recommended convention is **upper-case**
//!   (e.g. `SF-001`, `INC-2024-042`).
//!
//! # Enforcement
//!
//! [`validate_outage_id`] is called by the write path (`calculate_sla`) and
//! returns `Err(SLAError::InvalidInput)` for non-canonical IDs. Read paths
//! (`get_history_by_outage`, `get_latest_by_outage`) do not validate because
//! IDs stored in history are already canonical by construction.
//!
//! # Accepted formats (fixtures)
//!
//! | Input | Valid? | Notes |
//! |---|---|---|
//! | `SF-001` | ✅ | Recommended format |
//! | `INC-2024-042` | ✅ | Date-scoped format |
//! | `OUTAGE1` | ✅ | No separator |
//! | `sf-001` | ✅ | Lower-case; valid but discouraged |
//! | `` (empty) | ❌ | Too short |
//! | `SF 001` | ❌ | Space not allowed |
//! | `SF_001` | ❌ | Underscore not allowed |
//! | 33+ chars | ❌ | Exceeds Symbol length limit |

use soroban_sdk::{Env, String, Symbol};

use crate::SLAError;

/// Maximum byte length of a canonical outage ID.
pub const MAX_OUTAGE_ID_LEN: u32 = 32;

/// Validate that `outage_id` is in canonical form.
///
/// Returns `Ok(())` for a valid ID, or `Err(SLAError::InvalidInput)` if the
/// ID is empty, too long, or contains disallowed characters.
///
/// This is a guard called at the write path boundary; it does not modify
/// the symbol. Case normalization (upper-casing) is the caller's responsibility.
pub fn validate_outage_id(env: &Env, outage_id: &Symbol) -> Result<(), SLAError> {
    // Convert Symbol to String for length and character inspection.
    // Symbol::to_string() produces the raw ASCII representation.
    let s: String = outage_id.to_string();
    let len = s.len();

    if len == 0 || len > MAX_OUTAGE_ID_LEN {
        return Err(SLAError::InvalidInput);
    }

    // Validate each byte: allow A-Z, a-z, 0-9, hyphen.
    let mut i = 0u32;
    while i < len {
        let byte = s.get(i).unwrap_or(0);
        let valid = matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-');
        if !valid {
            return Err(SLAError::InvalidInput);
        }
        i += 1;
    }

    Ok(())
}