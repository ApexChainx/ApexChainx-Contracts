#![no_main]

//! Fuzz target: `validate_config` against its documented semantics.
//!
//! # What this target asserts
//!
//! `validate_config` cannot panic — it is a chain of comparisons over two
//! `i128`s and a `u32` — so "did not crash" says nothing about whether it
//! still enforces the policy. Every iteration instead compares it against
//! `apexchainx_calculator::spec::expected_validate_config`, the independent
//! restatement of the documented rules, and fails on any disagreement:
//!
//! * **It accepts exactly the documented set.** Not a superset (a config the
//!   rules reject reaching storage) and not a subset (a legitimate config
//!   refused). Both directions are checked, so neither widening nor narrowing
//!   a bound can pass unnoticed.
//! * **It returns the documented error, not merely an error.** The checks run
//!   in a fixed precedence — severity, then general bounds
//!   (threshold → penalty → reward), then severity-specific bounds, then
//!   cross-parameter consistency — and the first failure wins. Asserting the
//!   exact variant makes that ordering a tested property: a config that breaks
//!   several rules at once must still report the earliest one, so the error
//!   code stays a deterministic function of the inputs that backends can
//!   branch on.
//! * **Severity-specific bounds.** `critical` ≤ 60 min and ≥ 50/min,
//!   `high` ≤ 120 min and ≥ 25/min, `medium` ≤ 240 min and ≥ 10/min, `low`
//!   capped at 100/min from above.
//! * **Cross-parameter consistency.** `penalty * 3 < reward * 2` — meeting the
//!   SLA must stay financially better than absorbing penalties.
//! * **Non-canonical severities are always rejected** with `InvalidSeverity`,
//!   so a custom severity can never take the canonical validation path.
//! * **Validation is sufficient, not just necessary.** Anything it accepts is
//!   then run through `compute_result` and must produce a usable settlement —
//!   the composition the bounds exist to guarantee.
//!
//! The full list, with rationale, is on
//! [`apexchainx_calculator::fuzz_spec::assert_validate_config_matches_spec`].
//!
//! # What this target does NOT assert
//!
//! Cross-*severity* penalty ordering (critical ≥ high ≥ medium) is enforced by
//! `validate_cross_severity_penalty_ordering` against stored state, not by
//! `validate_config`, and is covered by `config_mutation_sequences`. Freeze
//! state, admin auth and event emission are likewise out of scope here. See
//! `docs/FUZZING_GUARANTEES.md`.
//!
//! # If this target fails
//!
//! See `docs/FUZZING_GUARANTEES.md` § "Which statement is authoritative".

use apexchainx_calculator::{fuzz_spec, spec, SLAConfig};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::symbol_short;

/// Severities that must never validate. Included so the `InvalidSeverity`
/// branch — and the guarantee that a custom severity cannot borrow the
/// canonical bounds — is exercised, not just the four canonical names.
const NON_CANONICAL: [soroban_sdk::Symbol; 4] = [
    symbol_short!("urgent"),
    symbol_short!("cust0"),
    symbol_short!("CRITICAL"),
    symbol_short!("crit"),
];

fuzz_target!(|data: (u32, u32, i128, i128)| {
    let (severity_idx, threshold_minutes, penalty_per_minute, reward_base) = data;

    // Every eighth input probes a non-canonical severity; the rest sweep the
    // canonical four so the severity-specific bounds get the bulk of the
    // budget.
    let severity = if severity_idx % 8 == 7 {
        NON_CANONICAL[((severity_idx / 8) % 4) as usize].clone()
    } else {
        spec::CANONICAL_SEVERITIES[(severity_idx % 4) as usize].clone()
    };

    let accepted = fuzz_spec::assert_validate_config_matches_spec(
        &severity,
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    );

    if accepted {
        // Validation is sufficient: an accepted config computes for any MTTR.
        let cfg = SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        };
        for mttr in [0u32, threshold_minutes, threshold_minutes.saturating_add(1), u32::MAX] {
            fuzz_spec::assert_validated_config_computes(symbol_short!("test"), mttr, &cfg);
        }
    }
});
