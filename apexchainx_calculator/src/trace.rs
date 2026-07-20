//! Issue #98 — Structured debug-trace logging for the test harness.
//!
//! Compiled only when the `debug-trace` cargo feature is enabled AND the
//! crate is built for a non-wasm target. The module:
//!
//! * Records a single JSON-lines entry for each meaningful state transition
//!   in the contract (`initialize`, `set_config`, `calculate_sla`, `pause`,
//!   `unpause`, `freeze_config`, `unfreeze_config`, `set_operator`).
//! * Exposes a small assertion API so tests can do
//!   `assert!(trace::find_event("set_config"))` after the action.
//!
//! The trace is stored in a [`std::thread_local`] `RefCell` so parallel
//! `cargo test` execution does not interleave events between scenarios.
//! Tests should call [`reset`] at the start of each scenario to start
//! from a clean slate on the current thread.
//!
//! This module deliberately avoids `serde` / `serde_json` so the feature
//! does not pull any extra dependency into the contract crate. JSON
//! fragments are hand-crafted with [`alloc::format!`] since all recorded
//! fields are statically known, machine-controlled types or short Symbols
//! (which are guaranteed to be alphanumeric by the Soroban SDK).
//!
//! Example wiring (in `lib.rs`):
//!
//! ```ignore
//! #[cfg(all(feature = "debug-trace", not(target_family = "wasm")))]
//! {
//!     trace::record_set_config(
//!         "critical",
//!         threshold_minutes,
//!         penalty_per_minute,
//!         reward_base,
//!     );
//! }
//! ```

#![cfg(all(feature = "debug-trace", not(target_family = "wasm")))]

extern crate alloc;
extern crate std;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::cell::RefCell;
use std::thread_local;
use std::vec::Vec as StdVec;

/// One captured state-transition event.
#[derive(Clone, Debug, PartialEq)]
pub struct TraceEvent {
    /// Canonical transition name (e.g. `"set_config"`).
    pub name: String,
    /// JSON fragments describing the transition (one-line JSON object).
    pub json: String,
}

impl TraceEvent {
    pub fn new(name: &str, json: &str) -> Self {
        Self {
            name: name.to_string(),
            json: json.to_string(),
        }
    }

    /// Returns the event as a single JSON line of shape
    /// `{"event":"<name>","fields":<json>}`.
    pub fn to_jsonl(&self) -> String {
        format!(r#"{{"event":"{}","fields":{}}}"#, self.name, self.json)
    }
}

thread_local! {
    static TRACE: RefCell<StdVec<TraceEvent>> = const { RefCell::new(StdVec::new()) };
}

/// Drop every recorded event on the current thread. Tests call this at
/// the start of each scenario so assertions see only events emitted by
/// that scenario.
pub fn reset() {
    TRACE.with(|t| t.borrow_mut().clear());
}

/// Append a structured event. `name` is the canonical transition id and
/// `fields_json` must be a valid JSON object literal (without the
/// surrounding braces added by this function).
pub fn record(name: &str, fields_json: &str) {
    let event = TraceEvent::new(name, fields_json);
    TRACE.with(|t| t.borrow_mut().push(event));
}

/// Returns `true` if any event with `name` has been recorded on this
/// thread since the most recent [`reset`].
pub fn find_event(name: &str) -> bool {
    TRACE.with(|t| t.borrow().iter().any(|e| e.name == name))
}

/// Returns every event matching `name` on this thread since the most
/// recent [`reset`].
pub fn find_events(name: &str) -> Vec<TraceEvent> {
    TRACE.with(|t| {
        t.borrow()
            .iter()
            .filter(|e| e.name == name)
            .cloned()
            .collect()
    })
}

/// Returns the total number of recorded events on this thread since the
/// most recent [`reset`].
pub fn event_count() -> usize {
    TRACE.with(|t| t.borrow().len())
}

/// Returns every recorded event on this thread as a JSON line (one per
/// element). Lines are newline-separated so the result can be written
/// directly to a `.jsonl` file.
pub fn dump_jsonl() -> String {
    let mut out = String::new();
    TRACE.with(|t| {
        for e in t.borrow().iter() {
            out.push_str(&e.to_jsonl());
            out.push('\n');
        }
    });
    out
}

// ============================================================
// Per-operation helpers — typed wrappers around `record` so the
// call sites in `lib.rs` cannot forget to JSON-encode fields.
// Every helper takes only values whose JSON-safe encoding is
// statically known and constant cost.
// ============================================================

/// Record an `initialize` transition.
pub fn record_initialize(admin_short: &str, operator_short: &str) {
    record(
        "initialize",
        &format!(
            r#"{{"admin":"{}","operator":"{}"}}"#,
            admin_short, operator_short
        ),
    );
}

/// Record a `set_config` transition.
pub fn record_set_config(
    severity: &str,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) {
    record(
        "set_config",
        &format!(
            r#"{{"severity":"{}","threshold_minutes":{},"penalty_per_minute":{},"reward_base":{}}}"#,
            severity, threshold_minutes, penalty_per_minute, reward_base
        ),
    );
}

/// Record a `set_operator` transition.
pub fn record_set_operator() {
    record("set_operator", "{}");
}

/// Record a `calculate_sla` transition.
pub fn record_calculate_sla(
    severity: &str,
    status: &str,
    payment_type: &str,
    rating: &str,
    amount: i128,
    mttr_minutes: u32,
    threshold_minutes: u32,
) {
    record(
        "calculate_sla",
        &format!(
            r#"{{"severity":"{}","status":"{}","payment_type":"{}","rating":"{}","amount":{},"mttr_minutes":{},"threshold_minutes":{}}}"#,
            severity, status, payment_type, rating, amount, mttr_minutes, threshold_minutes
        ),
    );
}

/// Record a `pause` transition.
pub fn record_pause(reason: &str) {
    record("pause", &format!(r#"{{"reason":"{}"}}"#, escape_json(reason)));
}

/// Record an `unpause` transition.
pub fn record_unpause() {
    record("unpause", "{}");
}

/// Record a `freeze_config` transition.
pub fn record_freeze_config() {
    record("freeze_config", "{}");
}

/// Record an `unfreeze_config` transition.
pub fn record_unfreeze_config() {
    record("unfreeze_config", "{}");
}

/// Minimal JSON string escaping — covers only the characters we expect
/// to receive from Soroban `String` values (which themselves are valid
/// UTF-8) plus control characters that could appear in user-set pause
/// reasons. Quote and backslash are escaped so the result is a safe
/// JSON string value when surrounded by double quotes.
fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use core::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_clears_buffer() {
        record("alpha", "{}");
        record("beta", "{}");
        assert_eq!(event_count(), 2);
        reset();
        assert_eq!(event_count(), 0);
    }

    #[test]
    fn find_event_matches_by_name_only() {
        reset();
        record("set_config", r#"{"k":1}"#);
        record("calculate_sla", r#"{"k":2}"#);
        assert!(find_event("set_config"));
        assert!(find_event("calculate_sla"));
        assert!(!find_event("pause"));
        assert_eq!(find_events("set_config").len(), 1);
    }

    #[test]
    fn dump_jsonl_emits_one_line_per_event() {
        reset();
        record("set_config", r#"{"severity":"critical"}"#);
        record("calculate_sla", r#"{"amount":100}"#);
        let text = dump_jsonl();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(r#""event":"set_config""#));
        assert!(lines[0].contains(r#""severity":"critical""#));
        assert!(lines[1].contains(r#""event":"calculate_sla""#));
        assert!(lines[1].contains(r#""amount":100"#));
    }

    #[test]
    fn record_pause_escapes_reason() {
        reset();
        record_pause("hello \"world\"\n");
        let events = find_events("pause");
        assert_eq!(events.len(), 1);
        assert!(events[0].json.contains(r#"\"world\""#));
        assert!(events[0].json.contains(r#"\n"#));
    }

    #[test]
    fn typed_helpers_emit_expected_event_names() {
        reset();
        record_initialize("admin", "op");
        record_set_config("critical", 15, 100, 750);
        record_set_operator();
        record_calculate_sla("critical", "met", "rew", "top", 1500, 5, 15);
        record_pause("x");
        record_unpause();
        record_freeze_config();
        record_unfreeze_config();
        assert!(find_event("initialize"));
        assert!(find_event("set_config"));
        assert!(find_event("set_operator"));
        assert!(find_event("calculate_sla"));
        assert!(find_event("pause"));
        assert!(find_event("unpause"));
        assert!(find_event("freeze_config"));
        assert!(find_event("unfreeze_config"));
        assert_eq!(event_count(), 8);
    }
}
