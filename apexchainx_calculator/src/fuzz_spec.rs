//! Assertion bodies shared by the cargo-fuzz targets in `fuzz/fuzz_targets/`.
//!
//! # Why the assertions live in the library, not in the targets
//!
//! The fuzz targets are separate binaries in a separate workspace
//! (`apexchainx_calculator/fuzz/`). They are **not** built by `cargo test`,
//! `cargo clippy`, or `just ci`; they need a nightly toolchain, `cargo-fuzz`,
//! and a C++ toolchain for `libfuzzer-sys`. Assertion logic written directly
//! inside those files is therefore invisible to every routine check — it can
//! reference private items, fail to compile, or quietly rot for months, and
//! the only signal is a nightly job that has historically swallowed its own
//! exit code.
//!
//! So the targets are kept to a few lines of input decoding, and every
//! assertion they make lives here, in the library, where:
//!
//! * `cargo test --lib` type-checks it on every commit,
//! * the in-crate unit tests below exercise it on fixed vectors, and
//! * `crate::fuzz_tests` (proptest) exercises it on generated ones —
//!   so the fuzz suite's contract is enforced by `just test`, and the nightly
//!   campaign only adds coverage-guided input search on top.
//!
//! It also settles how a fuzz target reaches contract internals.
//! `SLACalculatorContract::compute_result` is private and
//! `SLACalculatorContract::validate_config` is `pub(crate)`; both are inside
//! the `#[contractimpl]` block, so widening them to `pub` would add them to
//! the deployed contract's ABI. As a descendant module of the crate root this
//! module can call them without changing a single exported symbol, and it
//! re-exports the checks as ordinary `pub fn`s the fuzz crate can link to.
//!
//! # What the assertions compare against
//!
//! Every check compares the implementation to [`crate::spec`], the independent
//! restatement of the documented rules — never to a second copy of the
//! implementation. See `docs/FUZZING_GUARANTEES.md` for the guarantees this
//! buys, the ones it does not, and the authority policy for resolving a
//! reported mismatch.

use soroban_sdk::Symbol;

use crate::{spec, SLAConfig, SLAError, SLAResult};

/// Panics unless `SLACalculatorContract::validate_config` agrees with
/// [`spec::expected_validate_config`] for these inputs.
///
/// # Invariants asserted (beyond panic-freedom)
///
/// 1. **Total agreement on acceptance.** The implementation accepts a config
///    if and only if the spec does. A config the documented rules reject can
///    never reach storage, and one they accept is never spuriously refused.
/// 2. **Exact error identity.** On rejection the implementation returns the
///    *same* error variant the documented precedence order names — not merely
///    *some* error. This is what makes the ordering of the validation steps a
///    tested property rather than an implementation detail.
///
/// Returns whether the config was accepted, so callers can chain the
/// downstream [`assert_compute_result_matches_spec`] check.
pub fn assert_validate_config_matches_spec(
    severity: &Symbol,
    threshold_minutes: u32,
    penalty_per_minute: i128,
    reward_base: i128,
) -> bool {
    let actual = crate::SLACalculatorContract::validate_config(
        severity,
        threshold_minutes,
        penalty_per_minute,
        reward_base,
    );
    let expected =
        spec::expected_validate_config(severity, threshold_minutes, penalty_per_minute, reward_base);

    match (&actual, &expected) {
        (Ok(()), Ok(())) => true,
        (Err(a), Err(e)) => {
            assert_eq!(
                *a as u32, *e as u32,
                "validate_config error mismatch for (threshold={}, penalty={}, reward={}): \
                 implementation returned {}, spec requires {}",
                threshold_minutes, penalty_per_minute, reward_base, *a as u32, *e as u32
            );
            false
        }
        (Ok(()), Err(e)) => panic!(
            "validate_config ACCEPTED a config the spec rejects with {} \
             (threshold={}, penalty={}, reward={})",
            *e as u32, threshold_minutes, penalty_per_minute, reward_base
        ),
        (Err(a), Ok(())) => panic!(
            "validate_config REJECTED with {} a config the spec accepts \
             (threshold={}, penalty={}, reward={})",
            *a as u32, threshold_minutes, penalty_per_minute, reward_base
        ),
    }
}

/// Panics unless both copies of `compute_result` agree with
/// [`spec::expected_compute_result`] for these inputs.
///
/// # Invariants asserted (beyond panic-freedom)
///
/// 1. **Outcome parity with the spec.** `status`, `payment_type`, `rating` and
///    `amount` each equal the documented value — including the
///    `mttr == threshold` boundary, which must be *met*, and the tier
///    boundaries at performance ratios 50 and 75.
/// 2. **Error parity with the spec.** An overflowing or degenerate amount
///    yields exactly `InvalidPenaltyAmount` / `InvalidRewardAmount` where the
///    spec says so, and succeeds where it does not. A target that only checked
///    "some error" would accept a silently saturating penalty.
/// 3. **Sign discipline.** A `met` result carries a strictly positive reward
///    and a `viol` result a strictly negative penalty — restated here as a
///    standalone assertion because it is the invariant settlement consumers
///    rely on, and it must hold even if the tier table is edited.
/// 4. **Input pass-through.** `outage_id`, `mttr_minutes`,
///    `threshold_minutes`, `config_version_hash` and `recorded_at` are
///    returned unchanged. The pure function must not rewrite its own inputs.
/// 5. **Purity and determinism.** Called twice with equal inputs it produces
///    equal output.
/// 6. **Agreement between the two implementations.** The contract entry point
///    (`SLACalculatorContract::compute_result`, used by `calculate_sla`,
///    `calculate_sla_view` and `replay_calculate_sla`) and the module-level
///    copy ([`crate::calculation::compute_result`], used by the parity
///    baseline) are two hand-maintained transcriptions of one rule. They are
///    asserted equal so a fix applied to one and missed on the other is a
///    fuzz failure rather than a silent divergence between the on-chain
///    result and the release gate that is supposed to be checking it.
pub fn assert_compute_result_matches_spec(
    outage_id: Symbol,
    mttr_minutes: u32,
    cfg: &SLAConfig,
    config_version_hash: u64,
    recorded_at: u64,
) {
    let actual = crate::SLACalculatorContract::compute_result(
        outage_id.clone(),
        mttr_minutes,
        cfg,
        config_version_hash,
        recorded_at,
    );

    // (6) The module-level copy must make the identical decision.
    let mirrored = crate::calculation::compute_result(
        outage_id.clone(),
        mttr_minutes,
        cfg,
        config_version_hash,
        recorded_at,
    );
    assert_results_equal(
        &actual,
        &mirrored,
        "SLACalculatorContract::compute_result and calculation::compute_result diverged",
        mttr_minutes,
        cfg,
    );

    // (5) Determinism: the same inputs must produce the same output.
    let repeated = crate::SLACalculatorContract::compute_result(
        outage_id.clone(),
        mttr_minutes,
        cfg,
        config_version_hash,
        recorded_at,
    );
    assert_results_equal(
        &actual,
        &repeated,
        "compute_result is not deterministic",
        mttr_minutes,
        cfg,
    );

    let expected = spec::expected_compute_result(mttr_minutes, cfg);

    match (actual, expected) {
        (Ok(result), Ok(want)) => {
            // (1) Outcome parity.
            assert_eq!(
                result.status, want.status,
                "status mismatch for mttr={} threshold={}",
                mttr_minutes, cfg.threshold_minutes
            );
            assert_eq!(
                result.payment_type, want.payment_type,
                "payment_type mismatch for mttr={} threshold={}",
                mttr_minutes, cfg.threshold_minutes
            );
            assert_eq!(
                result.rating, want.rating,
                "rating mismatch for mttr={} threshold={}",
                mttr_minutes, cfg.threshold_minutes
            );
            assert_eq!(
                result.amount, want.amount,
                "amount mismatch for mttr={} threshold={} penalty={} reward={}",
                mttr_minutes, cfg.threshold_minutes, cfg.penalty_per_minute, cfg.reward_base
            );

            // (3) Sign discipline, stated independently of the tier table.
            if result.status == spec::STATUS_MET {
                assert!(
                    result.amount > 0,
                    "met result must carry a strictly positive reward, got {}",
                    result.amount
                );
                assert_eq!(result.payment_type, spec::PAYMENT_REWARD);
                assert!(
                    result.rating == spec::RATING_TOP
                        || result.rating == spec::RATING_EXCELLENT
                        || result.rating == spec::RATING_GOOD,
                    "met result carries a non-met rating"
                );
                assert!(
                    mttr_minutes <= cfg.threshold_minutes,
                    "met status returned for mttr {} above threshold {}",
                    mttr_minutes,
                    cfg.threshold_minutes
                );
            } else {
                assert_eq!(result.status, spec::STATUS_VIOLATED);
                assert!(
                    result.amount < 0,
                    "violation must carry a strictly negative penalty, got {}",
                    result.amount
                );
                assert_eq!(result.payment_type, spec::PAYMENT_PENALTY);
                assert_eq!(result.rating, spec::RATING_POOR);
                assert!(
                    mttr_minutes > cfg.threshold_minutes,
                    "viol status returned for mttr {} within threshold {}",
                    mttr_minutes,
                    cfg.threshold_minutes
                );
            }

            // (4) Input pass-through.
            assert_eq!(result.outage_id, outage_id, "outage_id was not passed through");
            assert_eq!(result.mttr_minutes, mttr_minutes, "mttr_minutes was rewritten");
            assert_eq!(
                result.threshold_minutes, cfg.threshold_minutes,
                "threshold_minutes was rewritten"
            );
            assert_eq!(
                result.config_version_hash, config_version_hash,
                "config_version_hash was rewritten"
            );
            assert_eq!(result.recorded_at, recorded_at, "recorded_at was rewritten");
        }
        // (2) Error parity.
        (Err(actual_err), Err(want_err)) => assert_eq!(
            actual_err as u32, want_err as u32,
            "compute_result error mismatch for mttr={} threshold={} penalty={} reward={}",
            mttr_minutes, cfg.threshold_minutes, cfg.penalty_per_minute, cfg.reward_base
        ),
        (Ok(result), Err(want_err)) => panic!(
            "compute_result returned amount {} where the spec requires error {} \
             (mttr={} threshold={} penalty={} reward={})",
            result.amount,
            want_err as u32,
            mttr_minutes,
            cfg.threshold_minutes,
            cfg.penalty_per_minute,
            cfg.reward_base
        ),
        (Err(actual_err), Ok(want)) => panic!(
            "compute_result failed with {} where the spec requires amount {} \
             (mttr={} threshold={} penalty={} reward={})",
            actual_err as u32,
            want.amount,
            mttr_minutes,
            cfg.threshold_minutes,
            cfg.penalty_per_minute,
            cfg.reward_base
        ),
    }
}

/// Panics unless a config accepted by `validate_config` always produces a
/// usable `compute_result` for every MTTR.
///
/// # Invariant asserted (beyond panic-freedom)
///
/// **Validation is sufficient, not merely necessary.** The bounds enforced by
/// `validate_config` exist so that no accepted config can later fail
/// arithmetically. This asserts the composition directly: for a config the
/// implementation accepted, `compute_result` must succeed for the supplied
/// `mttr_minutes` — including `u32::MAX`, where the penalty product is largest
/// — and yield a settlement-shaped amount.
///
/// This is the cross-check that ties the two documented surfaces together; a
/// change that widens a validation bound without re-checking the arithmetic
/// guards surfaces here rather than in production.
pub fn assert_validated_config_computes(outage_id: Symbol, mttr_minutes: u32, cfg: &SLAConfig) {
    let result = crate::SLACalculatorContract::compute_result(outage_id, mttr_minutes, cfg, 0, 0);
    match result {
        Ok(res) => assert!(
            res.amount != 0,
            "a validated config produced a zero settlement amount \
             (mttr={} threshold={} penalty={} reward={})",
            mttr_minutes,
            cfg.threshold_minutes,
            cfg.penalty_per_minute,
            cfg.reward_base
        ),
        Err(err) => panic!(
            "validate_config accepted a config that compute_result rejects with {} \
             (mttr={} threshold={} penalty={} reward={})",
            err as u32, mttr_minutes, cfg.threshold_minutes, cfg.penalty_per_minute, cfg.reward_base
        ),
    }
}

/// Panics unless the pagination oracle in [`spec`] describes the page a given
/// `(offset, limit)` selects over a history of `len` entries.
///
/// # Invariants asserted (beyond panic-freedom)
///
/// 1. The page never starts before `offset` nor ends past `len`.
/// 2. `offset + limit` is evaluated with saturating arithmetic, so no `u32`
///    pair wraps into a wrong slice.
/// 3. `limit` is clamped to `MAX_PAGE_SIZE`, so no single call can read the
///    entire retained history.
/// 4. `has_more` is `true` exactly when the page ends before the end of
///    history, which makes `offset += page_len` a terminating iteration for
///    every non-zero limit.
pub fn assert_pagination_oracle_self_consistent(offset: u32, limit: u32, len: u32) {
    let end = spec::expected_page_end(offset, limit, len);
    let page_len = spec::expected_page_len(offset, limit, len);

    assert!(end <= len, "page end {} exceeds history length {}", end, len);
    assert!(
        page_len <= crate::history::MAX_PAGE_SIZE,
        "page of {} entries exceeds MAX_PAGE_SIZE",
        page_len
    );
    assert!(
        page_len <= limit || limit > crate::history::MAX_PAGE_SIZE,
        "page of {} entries exceeds the requested limit {}",
        page_len,
        limit
    );
    if offset >= len || limit == 0 {
        assert_eq!(
            page_len, 0,
            "expected an empty page for offset={} limit={}",
            offset, limit
        );
    }
    assert_eq!(
        spec::expected_has_more(offset, limit, len),
        limit > 0 && end < len,
        "has_more disagrees with the page end for offset={} limit={} len={}",
        offset,
        limit,
        len
    );
    if limit > 0 && offset < len {
        assert!(
            page_len > 0,
            "a non-zero limit within range produced an empty page (offset={} limit={} len={})",
            offset,
            limit,
            len
        );
    }
}

/// Compares two `compute_result` outcomes for exact equality, including the
/// error variant, and panics with the offending inputs on any difference.
fn assert_results_equal(
    left: &Result<SLAResult, SLAError>,
    right: &Result<SLAResult, SLAError>,
    context: &str,
    mttr_minutes: u32,
    cfg: &SLAConfig,
) {
    match (left, right) {
        (Ok(a), Ok(b)) => assert!(
            a.status == b.status
                && a.payment_type == b.payment_type
                && a.rating == b.rating
                && a.amount == b.amount
                && a.outage_id == b.outage_id
                && a.mttr_minutes == b.mttr_minutes
                && a.threshold_minutes == b.threshold_minutes
                && a.config_version_hash == b.config_version_hash
                && a.recorded_at == b.recorded_at,
            "{} (mttr={} threshold={} penalty={} reward={}): amounts {} vs {}",
            context,
            mttr_minutes,
            cfg.threshold_minutes,
            cfg.penalty_per_minute,
            cfg.reward_base,
            a.amount,
            b.amount
        ),
        (Err(a), Err(b)) => assert_eq!(
            *a as u32, *b as u32,
            "{} (mttr={} threshold={})",
            context, mttr_minutes, cfg.threshold_minutes
        ),
        _ => panic!(
            "{}: one side succeeded and the other failed \
             (mttr={} threshold={} penalty={} reward={})",
            context, mttr_minutes, cfg.threshold_minutes, cfg.penalty_per_minute, cfg.reward_base
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::symbol_short;

    fn cfg(threshold_minutes: u32, penalty_per_minute: i128, reward_base: i128) -> SLAConfig {
        SLAConfig {
            threshold_minutes,
            penalty_per_minute,
            reward_base,
        }
    }

    /// Boundary sweep around every documented decision point: the met/viol
    /// boundary and both reward-tier ratios. These are the exact cases a
    /// panic-only fuzz target could not have caught.
    #[test]
    fn compute_result_matches_spec_at_documented_boundaries() {
        let c = cfg(100, 50, 1_000);
        // mttr == threshold is met; +1 is the first violation.
        for mttr in [0u32, 49, 50, 74, 75, 99, 100, 101, 200] {
            assert_compute_result_matches_spec(symbol_short!("out"), mttr, &c, 7, 11);
        }
    }

    #[test]
    fn compute_result_boundary_ratings_are_the_documented_tiers() {
        let c = cfg(100, 50, 1_000);
        let at = |mttr: u32| {
            crate::SLACalculatorContract::compute_result(symbol_short!("out"), mttr, &c, 0, 0)
                .expect("valid config computes")
        };
        // ratio 49 -> top, 50 -> excel, 74 -> excel, 75 -> good, 100 -> good.
        assert_eq!(at(49).rating, spec::RATING_TOP);
        assert_eq!(at(50).rating, spec::RATING_EXCELLENT);
        assert_eq!(at(74).rating, spec::RATING_EXCELLENT);
        assert_eq!(at(75).rating, spec::RATING_GOOD);
        assert_eq!(at(100).rating, spec::RATING_GOOD);
        assert_eq!(at(100).status, spec::STATUS_MET);
        assert_eq!(at(101).status, spec::STATUS_VIOLATED);
    }

    #[test]
    fn compute_result_overflow_guards_match_spec() {
        // Penalty overflow: a huge overtime against a huge per-minute penalty.
        assert_compute_result_matches_spec(symbol_short!("out"), u32::MAX, &cfg(0, i128::MAX, 1), 0, 0);
        // Reward overflow via the 200% multiplier.
        assert_compute_result_matches_spec(symbol_short!("out"), 0, &cfg(10, i128::MAX, i128::MAX), 0, 0);
        // Degenerate zero/negative amounts.
        assert_compute_result_matches_spec(symbol_short!("out"), 5, &cfg(1, 0, 1_000), 0, 0);
        assert_compute_result_matches_spec(symbol_short!("out"), 0, &cfg(10, 1, 0), 0, 0);
    }

    #[test]
    fn validate_config_matches_spec_across_severity_bounds() {
        let severities = spec::CANONICAL_SEVERITIES;
        let thresholds = [0u32, 1, 60, 61, 120, 121, 240, 241, 1440, 1441];
        let penalties = [0i128, 1, 10, 25, 50, 100, 101, 10_000, 10_001];
        let rewards = [0i128, 1, 100, 1_000, 100_000, 100_001];
        for severity in severities.iter() {
            for threshold in thresholds {
                for penalty in penalties {
                    for reward in rewards {
                        assert_validate_config_matches_spec(severity, threshold, penalty, reward);
                    }
                }
            }
        }
    }

    #[test]
    fn validate_config_rejects_non_canonical_severities() {
        for severity in [symbol_short!("cust0"), symbol_short!("urgent"), symbol_short!("")] {
            let accepted = assert_validate_config_matches_spec(&severity, 30, 60, 1_000);
            assert!(!accepted, "a non-canonical severity must never validate");
        }
    }

    #[test]
    fn accepted_configs_always_compute() {
        let severities = spec::CANONICAL_SEVERITIES;
        for severity in severities.iter() {
            for threshold in [1u32, 30, 60, 120, 240, 1440] {
                for penalty in [1i128, 10, 25, 50, 100, 10_000] {
                    for reward in [1i128, 1_000, 100_000] {
                        if !assert_validate_config_matches_spec(severity, threshold, penalty, reward) {
                            continue;
                        }
                        let c = cfg(threshold, penalty, reward);
                        for mttr in [0u32, threshold, threshold + 1, u32::MAX] {
                            assert_validated_config_computes(symbol_short!("out"), mttr, &c);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn pagination_oracle_holds_at_extremes() {
        for len in [0u32, 1, 5, 199, 200, 201, 1_000] {
            for offset in [0u32, 1, 199, 200, 1_000, u32::MAX] {
                for limit in [0u32, 1, 199, 200, 201, u32::MAX] {
                    assert_pagination_oracle_self_consistent(offset, limit, len);
                }
            }
        }
    }
}
