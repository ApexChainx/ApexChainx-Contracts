# Tier-Boundary Scenario Fuzzing for `compute_result`

> **Issue:** [#681](https://github.com/ApexChainx/ApexChainx-Contracts/issues/681)
> **Status:** Findings recorded; scenario mode not yet implemented
> **Applies to:** `compute_result`, `apexchainx_calculator/fuzz/fuzz_targets/compute_result.rs`

This document records the branch structure of `compute_result`, the tier
boundaries that random fuzzing reaches unreliably, and the scenario mode that
[#681](https://github.com/ApexChainx/ApexChainx-Contracts/issues/681) asks for. It
is the reference for that work.

**Related documents:**
- [`FUZZING_GUARANTEES.md`](FUZZING_GUARANTEES.md) - what each fuzz target does and does not assert
- [`FUZZING_TRIAGE.md`](FUZZING_TRIAGE.md) - how to triage a failure
- [`SEVERITY_CURVES.md`](SEVERITY_CURVES.md) - severity configuration reference

---

## 1. Branch structure of `compute_result`

`compute_result` is a pure function, transcribed twice in this crate: the on-chain
path at `SLACalculatorContract::compute_result` in
`apexchainx_calculator/src/lib.rs:3011`, and `calculation::compute_result` in
`apexchainx_calculator/src/calculation.rs:291`. The parity baseline checks the two
against each other. It derives a status, an amount, and a rating from
`mttr_minutes` and a `SLAConfig`:

**Violation branch**, entered when `mttr_minutes > threshold` (`lib.rs:3021`):

- `overtime = mttr_minutes - threshold`
- `penalty = overtime.checked_mul(penalty_per_minute)`, then negated
- Guards: `InvalidPenaltyAmount` on overflow or on negation failure, and an
  explicit `amount >= 0` rejection
- `status = viol`, `payment_type = pen`, `rating = poor`

**Met branch**, entered when `mttr_minutes <= threshold` (`lib.rs:3048`):

- `performance_ratio = (mttr_minutes * 100) / threshold` (integer division,
  `unwrap_or(0)`)
- Three rating tiers on that ratio, with reward multipliers:

| `performance_ratio` | `rating` | Multiplier |
| --- | --- | --- |
| `< 50` | `top` | 200 |
| `50..75` | `excel` | 150 |
| `>= 75` | `good` | 100 |

- `reward = reward_base.checked_mul(multiplier)`, then `div_euclid(100)`
- Guard: `InvalidRewardAmount` on overflow

Note the boundary is **strict**: `mttr == threshold` is *met*, not violated. A
regression that flipped this would not panic. It would silently invert the
settlement direction for every outage repaired exactly on time.

## 2. The boundaries that matter

Canonical tier defaults, from the `initialize` seeding at `lib.rs:1390`-`1421`:

| Severity | `threshold_minutes` | `penalty_per_minute` | `reward_base` |
| --- | --- | --- | --- |
| `critical` | 15 | 100 | 750 |
| `high` | 30 | 50 | 750 |
| `medium` | 60 | 25 | 750 |
| `low` | 120 | 10 | 600 |

Two independent boundary families follow from the code above.

**Violation boundary, per tier.** `mttr == threshold` is met; `mttr == threshold + 1`
is the first violation. For `critical` that is the pair `(15, 16)`; for `low`,
`(120, 121)`. The penalty at the first violating MTTR is exactly one unit of
`penalty_per_minute`, so these are also the smallest-magnitude penalty paths and
the most likely to be masked by an off-by-one that happens to be absorbed
downstream.

**Rating-tier boundaries, per tier.** The rating flips at `performance_ratio` 50
and 75, which land on different MTTR values for each threshold because the ratio
is `mttr * 100 / threshold` under integer division. For `critical`
(`threshold = 15`) the `top` / `excel` edge is at MTTR 7 or 8, and the
`excel` / `good` edge at MTTR 11 or 12. For `low` (`threshold = 120`) the same
edges are near MTTR 60 and near MTTR 90. Integer division makes these edges
uneven, and the exact crossover MTTR differs per tier, so a single hard-coded
sweep does not generalise.

The multipliers are also tier-sensitive through `reward_base`: `low` uses 600 while
the other three use 750, so the `good` tier pays 600 rather than 750 for `low` and a
scenario that assumes a uniform base will mispredict it.

## 3. Why random fuzzing under-covers this

`fuzz/fuzz_targets/compute_result.rs` draws `(mttr, severity_idx, threshold,
penalty, reward)` from the fuzzer and runs three arms: an unconstrained pass
against `fuzz_spec::assert_compute_result_matches_spec`, a validated-config pass
that also probes `u32::MAX`, and a max-MTTR boundary pass at
`spec::MAX_MTTR_MINUTES` +/- 1.

That is good coverage of the *error* and *overflow* paths, and the target's own
doc comment is explicit that panic-freedom is the floor rather than the goal. But
the arms pin only one family of edges: the MTTR ceiling. Nothing walks the
per-tier `threshold` and the per-tier rating crossovers together.

Because the edges are a small set of exact integers inside a wide input domain, a
random campaign can miss a given tier boundary for its entire budget while still
reporting success. Missing one is a silent correctness gap, not a crash: a wrong
`rating` or a wrong multiplier at the crossover produces a plausible-looking
settlement.

Existing parity fixtures (`test_backend_parity_threshold_boundary_cases`,
`test_backend_parity_reward_tier_cases`) do cover some of these edges, and the
monotonicity tests cover direction. Neither systematically pivots every tier
across every boundary.

## 4. Scenario mode required to close #681

Acceptance criteria from the issue, and what each still needs:

| Criterion | State |
| --- | --- |
| Tier-boundary scenario fuzzed | **Not met.** No per-tier pivot exists in any fuzz target. |
| No panic at boundary | **Met** for the paths the current targets already drive, by the `checked_mul` / `checked_neg` guards. Not established per tier. |
| Amounts match parity fixture expectations | **Not met** for the per-tier crossovers. |

A conforming scenario mode should, for each of the four canonical severities:

1. Pin the config to that tier's default `threshold_minutes`, `penalty_per_minute`,
   and `reward_base` from the table in section 2, rather than drawing them.
2. Vary `mttr` across the exact crossovers for that tier: `threshold`,
   `threshold + 1`, and the MTTR values where `performance_ratio` crosses 50 and
   75 under integer division, plus one either side of each.
3. Assert the branch taken (`viol` vs `met`) is the expected one, that `rating`
   and `payment_type` match the resulting tier, and that the amount equals the
   parity-fixture expectation rather than merely "did not panic".
4. Reuse `fuzz_spec::assert_compute_result_matches_spec` so the scenario mode
   stays consistent with the existing spec oracle and the two-transcription
   parity check, instead of asserting against a fourth restatement of the rules.

Deriving the crossover MTTRs from the threshold rather than hard-coding them is
what makes the mode survive a change to the tier defaults; a hard-coded list would
need updating in the same commit as any default change, which is the coupling
this issue is trying to remove.
