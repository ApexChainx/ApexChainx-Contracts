//! # Parity Checker — Historical Canonical Baseline Comparison
//!
//! This module compares the contract's current computation against the
//! locked-in canonical vectors recorded in `test_snapshots/tests/parity_baseline.json`.
//!
//! ## Purpose
//!
//! Release regressions in SLA calculation can be subtle — a one-off error in
//! the rating boundary, an overflow guard change, or a reward-multiplier tweak
//! may not be caught by unit tests that were written against the *new* code.
//! This checker anchors behaviour to a set of historically verified golden
//! outputs so that any deviation is an explicit, reviewed break rather than a
//! silent drift.
//!
//! ## Design
//!
//! * Uses [`crate::calculation::compute_result`] directly, the same pure
//!   function exercised by `calculate_sla` and `calculate_sla_view`.
//! * Inline constant vectors mirror the JSON baseline exactly; the JSON file
//!   is the human-readable authority for auditors and the downstream backend.
//! * `config_version_hash` and `recorded_at` are pinned to deterministic
//!   sentinel values (`0`) so they never cause spurious failures.
//! * All vectors are checked in a single `#[test]` sweep so that a single
//!   `cargo test parity_tests::` invocation is the complete release gate.
//!
//! ## Updating the baseline
//!
//! When a *deliberate* change to the SLA calculation logic is made:
//!
//! 1. Verify the new outputs are correct by code review.
//! 2. Update `test_snapshots/tests/parity_baseline.json` to match.
//! 3. Update the inline `PARITY_VECTORS` table below to match.
//! 4. Bump the `"baseline_version"` field in the JSON file.
//! 5. Record the change in `CHANGELOG.md` under the release section.
//!
//! **Never update the baseline to make a failing test pass without first
//! confirming the new output is intentional and correct.**

#![cfg(test)]

use crate::{
    calculation::compute_result,
    SLAConfig,
};
use soroban_sdk::{Env, Symbol};

// ─── Sentinel values ────────────────────────────────────────────────────────
//
// These constants are pinned so that config_version_hash changes (driven by
// clock or config state) cannot cause parity failures.  The pure calculation
// path does not use them for outcome determination.
const PINNED_CONFIG_HASH: u64 = 0;
const PINNED_RECORDED_AT: u64 = 0;

// ─── Inline golden vectors ───────────────────────────────────────────────────
//
// Each entry represents one row from parity_baseline.json.  Keep this table
// and the JSON file in sync; the JSON is the human-readable authority.
//
// Columns: (case_id, severity, threshold_minutes, penalty_per_minute,
//            reward_base, mttr_minutes, expected_status, expected_payment_type,
//            expected_rating, expected_amount)
//
// Derivation (for reviewers):
//   met  : ratio = mttr * 100 / threshold
//           ratio < 50  → multiplier 200 → reward = reward_base * 200 / 100
//           ratio < 75  → multiplier 150 → reward = reward_base * 150 / 100
//           otherwise   → multiplier 100 → reward = reward_base * 100 / 100
//   viol : overtime = mttr - threshold; amount = -(overtime * penalty_per_minute)

struct ParityVector {
    case_id: &'static str,
    /// Informational label matching the JSON baseline; the config fields carry
    /// the actual numerical parameters used in computation.
    #[allow(dead_code)]
    severity: &'static str,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
    mttr_minutes: u32,
    expected_status: &'static str,
    expected_payment_type: &'static str,
    expected_rating: &'static str,
    expected_amount: i128,
}

// NOTE: The `severity` field is only used to document the intent of each
// vector; the config struct carries the actual numerical parameters.
const PARITY_VECTORS: &[ParityVector] = &[
    // ── critical (threshold=15, penalty=100/min, reward_base=750) ─────────

    // MTTR 0 — best possible; ratio 0% (<50%) → top rating, 2× reward
    ParityVector {
        case_id: "critical_mttr_0_top",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 0,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1500, // 750 * 200 / 100
    },
    // MTTR 7 — ratio 46% (<50%) → top rating
    ParityVector {
        case_id: "critical_mttr_7_top",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 7,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1500,
    },
    // MTTR 8 — ratio 53% (≥50%, <75%) → excel rating
    ParityVector {
        case_id: "critical_mttr_8_excel",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 8,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 1125, // 750 * 150 / 100
    },
    // MTTR 11 — ratio 73% (≥50%, <75%) → excel rating (boundary just below 75%)
    ParityVector {
        case_id: "critical_mttr_11_excel",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 11,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 1125,
    },
    // MTTR 12 — ratio 80% (≥75%) → good rating
    ParityVector {
        case_id: "critical_mttr_12_good",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 12,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 750, // 750 * 100 / 100
    },
    // MTTR 15 — exact threshold → met, good rating (boundary)
    ParityVector {
        case_id: "critical_mttr_15_exact",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 15,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 750,
    },
    // MTTR 16 — one minute over threshold → viol, penalty = 1 * 100
    ParityVector {
        case_id: "critical_mttr_16_viol",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 16,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -100,
    },
    // MTTR 30 — 15 minutes over → penalty = 15 * 100
    ParityVector {
        case_id: "critical_mttr_30_viol",
        severity: "critical",
        threshold_minutes: 15,
        penalty_per_minute: 100,
        reward_base: 750,
        mttr_minutes: 30,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -1500,
    },

    // ── high (threshold=30, penalty=50/min, reward_base=750) ──────────────

    // MTTR 0 — ratio 0% → top
    ParityVector {
        case_id: "high_mttr_0_top",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 0,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1500,
    },
    // MTTR 14 — ratio 46% (<50%) → top
    ParityVector {
        case_id: "high_mttr_14_top",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 14,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1500,
    },
    // MTTR 15 — ratio 50% (≥50%, <75%) → excel
    ParityVector {
        case_id: "high_mttr_15_excel",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 15,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 1125,
    },
    // MTTR 22 — ratio 73% (≥50%, <75%) → excel
    ParityVector {
        case_id: "high_mttr_22_excel",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 22,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 1125,
    },
    // MTTR 23 — ratio 76% (≥75%) → good
    ParityVector {
        case_id: "high_mttr_23_good",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 23,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 750,
    },
    // MTTR 30 — exact threshold → met, good (boundary)
    ParityVector {
        case_id: "high_mttr_30_exact",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 30,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 750,
    },
    // MTTR 31 — one over → viol, penalty = 1 * 50
    ParityVector {
        case_id: "high_mttr_31_viol",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 31,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -50,
    },
    // MTTR 60 — 30 over → penalty = 30 * 50
    ParityVector {
        case_id: "high_mttr_60_viol",
        severity: "high",
        threshold_minutes: 30,
        penalty_per_minute: 50,
        reward_base: 750,
        mttr_minutes: 60,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -1500,
    },

    // ── medium (threshold=60, penalty=25/min, reward_base=750) ────────────

    // MTTR 0 → top
    ParityVector {
        case_id: "medium_mttr_0_top",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 0,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1500,
    },
    // MTTR 29 — ratio 48% (<50%) → top
    ParityVector {
        case_id: "medium_mttr_29_top",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 29,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1500,
    },
    // MTTR 30 — ratio 50% → excel
    ParityVector {
        case_id: "medium_mttr_30_excel",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 30,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 1125,
    },
    // MTTR 44 — ratio 73% → excel
    ParityVector {
        case_id: "medium_mttr_44_excel",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 44,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 1125,
    },
    // MTTR 45 — ratio 75% → good
    ParityVector {
        case_id: "medium_mttr_45_good",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 45,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 750,
    },
    // MTTR 60 — exact threshold → met, good
    ParityVector {
        case_id: "medium_mttr_60_exact",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 60,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 750,
    },
    // MTTR 61 — one over → viol, penalty = 1 * 25
    ParityVector {
        case_id: "medium_mttr_61_viol",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 61,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -25,
    },
    // MTTR 120 — 60 over → penalty = 60 * 25
    ParityVector {
        case_id: "medium_mttr_120_viol",
        severity: "medium",
        threshold_minutes: 60,
        penalty_per_minute: 25,
        reward_base: 750,
        mttr_minutes: 120,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -1500,
    },

    // ── low (threshold=120, penalty=10/min, reward_base=600) ─────────────

    // MTTR 0 → top
    ParityVector {
        case_id: "low_mttr_0_top",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 0,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1200, // 600 * 200 / 100
    },
    // MTTR 59 — ratio 49% (<50%) → top
    ParityVector {
        case_id: "low_mttr_59_top",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 59,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "top",
        expected_amount: 1200,
    },
    // MTTR 60 — ratio 50% → excel
    ParityVector {
        case_id: "low_mttr_60_excel",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 60,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 900, // 600 * 150 / 100
    },
    // MTTR 89 — ratio 74% → excel
    ParityVector {
        case_id: "low_mttr_89_excel",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 89,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "excel",
        expected_amount: 900,
    },
    // MTTR 90 — ratio 75% → good
    ParityVector {
        case_id: "low_mttr_90_good",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 90,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 600, // 600 * 100 / 100
    },
    // MTTR 120 — exact threshold → met, good
    ParityVector {
        case_id: "low_mttr_120_exact",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 120,
        expected_status: "met",
        expected_payment_type: "rew",
        expected_rating: "good",
        expected_amount: 600,
    },
    // MTTR 121 — one over → viol, penalty = 1 * 10
    ParityVector {
        case_id: "low_mttr_121_viol",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 121,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -10,
    },
    // MTTR 240 — 120 over → penalty = 120 * 10
    ParityVector {
        case_id: "low_mttr_240_viol",
        severity: "low",
        threshold_minutes: 120,
        penalty_per_minute: 10,
        reward_base: 600,
        mttr_minutes: 240,
        expected_status: "viol",
        expected_payment_type: "pen",
        expected_rating: "poor",
        expected_amount: -1200,
    },
];

// ─── Parity check ────────────────────────────────────────────────────────────

/// Release gate: verifies every canonical golden vector against the current
/// `compute_result` implementation.
///
/// A failure here means the live computation has diverged from the locked-in
/// historical baseline.  This must be investigated and either the code or the
/// baseline updated explicitly before a release is cut.
///
/// Run in isolation with:
/// ```text
/// cargo test --lib parity_tests::
/// ```
/// or via the `just parity-check` recipe.
#[test]
fn parity_check_all_canonical_vectors() {
    let env = Env::default();

    let mut failures: alloc::vec::Vec<alloc::string::String> = alloc::vec::Vec::new();

    for v in PARITY_VECTORS {
        let outage_id = Symbol::new(&env, v.case_id);
        let cfg = SLAConfig {
            threshold_minutes: v.threshold_minutes,
            penalty_per_minute: v.penalty_per_minute,
            reward_base: v.reward_base,
        };

        let result = compute_result(
            outage_id,
            v.mttr_minutes,
            &cfg,
            PINNED_CONFIG_HASH,
            PINNED_RECORDED_AT,
        )
        .unwrap_or_else(|e| {
            panic!(
                "[parity] case '{}': compute_result returned error {:?}",
                v.case_id, e
            )
        });

        let want_status = symbol_short_from_str(&env, v.expected_status);
        let want_payment = symbol_short_from_str(&env, v.expected_payment_type);
        let want_rating = symbol_short_from_str(&env, v.expected_rating);

        let mut case_ok = true;

        if result.status != want_status {
            failures.push(alloc::format!(
                "[parity] case '{}': status expected '{}', got symbol (check source)",
                v.case_id, v.expected_status
            ));
            case_ok = false;
        }
        if result.payment_type != want_payment {
            failures.push(alloc::format!(
                "[parity] case '{}': payment_type expected '{}', got symbol (check source)",
                v.case_id, v.expected_payment_type
            ));
            case_ok = false;
        }
        if result.rating != want_rating {
            failures.push(alloc::format!(
                "[parity] case '{}': rating expected '{}', got symbol (check source)",
                v.case_id, v.expected_rating
            ));
            case_ok = false;
        }
        if result.amount != v.expected_amount {
            failures.push(alloc::format!(
                "[parity] case '{}': amount expected {}, got {}",
                v.case_id, v.expected_amount, result.amount
            ));
            case_ok = false;
        }
        if result.mttr_minutes != v.mttr_minutes {
            failures.push(alloc::format!(
                "[parity] case '{}': mttr_minutes expected {}, got {}",
                v.case_id, v.mttr_minutes, result.mttr_minutes
            ));
            case_ok = false;
        }
        if result.threshold_minutes != v.threshold_minutes {
            failures.push(alloc::format!(
                "[parity] case '{}': threshold_minutes expected {}, got {}",
                v.case_id, v.threshold_minutes, result.threshold_minutes
            ));
            case_ok = false;
        }

        let _ = case_ok; // suppress unused warning; failures vec carries the state
    }

    if !failures.is_empty() {
        let msg = failures.join("\n");
        panic!(
            "\n\n=== PARITY CHECK FAILED ({} vector(s)) ===\n{}\n\n\
             To resolve: verify the change is intentional, update \
             test_snapshots/tests/parity_baseline.json and the inline \
             PARITY_VECTORS table, bump baseline_version, and record in CHANGELOG.\n",
            failures.len(),
            msg
        );
    }
}

/// Sanity-check: the PARITY_VECTORS table must not be empty and must have no
/// duplicate case_ids (which would mask a real vector being silently skipped).
#[test]
fn parity_vectors_have_unique_case_ids() {
    assert!(
        !PARITY_VECTORS.is_empty(),
        "PARITY_VECTORS must not be empty — the parity baseline has been erased"
    );

    let mut seen = alloc::collections::BTreeSet::new();
    for v in PARITY_VECTORS {
        assert!(
            seen.insert(v.case_id),
            "duplicate case_id in PARITY_VECTORS: '{}'",
            v.case_id
        );
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Convert a `&str` whose length ≤ 9 bytes to a `Symbol` via `symbol_short!`
/// equivalents at runtime.  Used to compare against the `symbol_short!(…)`
/// literals produced by `compute_result`.
fn symbol_short_from_str(env: &Env, s: &str) -> Symbol {
    // soroban_sdk::Symbol::new delegates to symbol_short for short strings.
    Symbol::new(env, s)
}
