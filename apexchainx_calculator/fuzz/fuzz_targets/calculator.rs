#![no_main]

//! Fuzz target: `compute_result` against its documented semantics.
//!
//! # What this target asserts
//!
//! Panic-freedom is the floor, not the goal. `compute_result` is guarded by
//! `checked_mul` / `checked_neg` throughout, so panics were engineered out
//! before this target existed; a campaign that only watched for crashes would
//! spend its entire budget confirming that. Every iteration therefore compares
//! the implementation against `apexchainx_calculator::spec` — an independent
//! restatement of the documented rules — and fails on any disagreement:
//!
//! * **Met/violated boundary.** `mttr == threshold` is *met*. A regression that
//!   made the boundary a violation would not crash; it would silently invert
//!   the settlement direction for every outage repaired exactly on time.
//! * **Reward tiers.** The performance ratio maps to `top` / `excel` / `good`
//!   at exactly 50 and 75, with 200% / 150% / 100% multipliers.
//! * **Overflow handling.** An overflowing penalty or reward yields exactly
//!   `InvalidPenaltyAmount` / `InvalidRewardAmount` — never a saturated amount,
//!   and never a different error code.
//! * **Sign discipline.** A stored reward is strictly positive and a stored
//!   penalty strictly negative, so a settlement can never be a no-op.
//! * **Input pass-through and determinism.** The pure function returns its
//!   inputs unchanged and gives the same answer twice.
//! * **Agreement between the contract's two transcriptions of the rule** —
//!   `SLACalculatorContract::compute_result` (the on-chain path) and
//!   `calculation::compute_result` (the one the parity baseline checks).
//!
//! The full list, with rationale, is on
//! [`apexchainx_calculator::fuzz_spec::assert_compute_result_matches_spec`].
//!
//! # What this target does NOT assert
//!
//! Nothing stateful: storage, history retention, auth, pausing, duplicate
//! detection and event emission are out of scope here and belong to
//! `config_mutation_sequences`. See `docs/FUZZING_GUARANTEES.md`.
//!
//! # If this target fails
//!
//! A failure means the implementation and the documented spec disagree. Read
//! `docs/FUZZING_GUARANTEES.md` § "Which statement is authoritative" before
//! changing either side — editing `spec.rs` to match new behaviour is correct
//! only when the behaviour change was deliberate and reviewed.

use apexchainx_calculator::{fuzz_spec, spec, SLAConfig};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::symbol_short;

fuzz_target!(|data: (u32, u32, u32, i128, i128)| {
    let (mttr, severity_idx, threshold_minutes, penalty_per_minute, reward_base) = data;
    let severity = spec::CANONICAL_SEVERITIES[(severity_idx % 4) as usize].clone();

    let cfg = SLAConfig {
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    };

    // Arm 1 — unconstrained. `compute_result` is a pure function with no
    // precondition, and its overflow and degenerate-amount branches are only
    // reachable with parameters `validate_config` would reject. Fuzzing it
    // behind a validity filter (as this target originally did) left exactly
    // those documented error paths unexercised.
    //
    // `config_version_hash` and `recorded_at` are fed fuzzer-derived values
    // rather than zero so the pass-through assertions have something to catch.
    fuzz_spec::assert_compute_result_matches_spec(
        symbol_short!("outage"),
        mttr,
        &cfg,
        u64::from(severity_idx),
        u64::from(mttr),
    );

    // Arm 2 — the cross-check between the two documented surfaces: a config
    // `validate_config` accepts must compute a usable settlement for *any*
    // MTTR, including `u32::MAX` where the penalty product is largest. This is
    // the property that makes the validation bounds load-bearing rather than
    // decorative.
    if fuzz_spec::assert_validate_config_matches_spec(
        &severity,
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    ) {
        fuzz_spec::assert_validated_config_computes(symbol_short!("outage"), mttr, &cfg);
        fuzz_spec::assert_validated_config_computes(symbol_short!("outage"), u32::MAX, &cfg);
    }
});
