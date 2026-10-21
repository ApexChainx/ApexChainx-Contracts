# Fuzzing Guarantees

> **Status:** Active
> **Audience:** Contract contributors, reviewers, and auditors
> **Last updated:** 2026-08-29

## Table of Contents

- [Overview](#overview)
- [What the fuzz suite guarantees](#what-the-fuzz-suite-guarantees)
- [What the fuzz suite does not guarantee](#what-the-fuzz-suite-does-not-guarantee)
- [Where the spec lives](#where-the-spec-lives)
- [Which statement is authoritative](#which-statement-is-authoritative)
- [Running the suite](#running-the-suite)
- [Adding or changing a target](#adding-or-changing-a-target)

## Overview

The contract has two distinct fuzzing layers, and it is worth being precise
about which is which:

| Layer | Where | Runs on | Toolchain |
|---|---|---|---|
| **Property tests** | `apexchainx_calculator/src/fuzz_tests.rs` | Every commit (`just fuzz`, `just ci`) | Stable, `proptest` |
| **Coverage-guided fuzzing** | `apexchainx_calculator/fuzz/fuzz_targets/` | Nightly (`.github/workflows/fuzz.yml`) | Nightly, `cargo-fuzz` + libFuzzer |

`just fuzz` runs the *first* one. The cargo-fuzz targets are a separate crate
with a separate workspace and are not built by `cargo test`, `cargo clippy`, or
`just ci`.

### The problem this document exists to prevent

The cargo-fuzz targets were originally written in the standard first form: run
the function, assert nothing, let libFuzzer watch for a crash. On functions
whose arithmetic is already guarded by `checked_mul` / `checked_neg` — which is
all of them here — that finds essentially nothing. Worse, a semantic regression
is invisible to it: if `compute_result` started treating `mttr == threshold` as
a violation, no input would crash, the nightly job would stay green, and the
only thing standing between that and a release would be the finite set of
golden vectors in `parity_tests`.

The targets now assert the documented semantics on every input. The rest of this
document says exactly what that buys.

## What the fuzz suite guarantees

Each target's own header lists its invariants; this is the summary.

### `compute_result`

Every input is checked against `spec::expected_compute_result`, an independent
restatement of the documented rules:

- **The met/violated boundary.** `mttr == threshold` is *met*. Only
  `mttr > threshold` is a violation.
- **Reward tiers.** The performance ratio `(mttr * 100) / threshold` selects
  `top` / `excel` / `good` at exactly 50 and 75, with 200% / 150% / 100%
  multipliers.
- **Overflow handling.** An overflowing penalty or reward returns exactly
  `InvalidPenaltyAmount` / `InvalidRewardAmount` — never a saturated amount and
  never a different code. Because the target no longer filters its inputs
  through `validate_config`, these paths are actually reached.
- **Sign discipline.** A stored reward is strictly positive, a stored penalty
  strictly negative. A settlement can never be a no-op.
- **Input pass-through and determinism.** `outage_id`, `mttr_minutes`,
  `threshold_minutes`, `config_version_hash` and `recorded_at` come back
  unchanged, and two identical calls agree.
- **Agreement between the contract's two copies of the rule.**
  `SLACalculatorContract::compute_result` (the on-chain path used by
  `calculate_sla`, `calculate_sla_view` and `replay_calculate_sla`) and
  `calculation::compute_result` (the copy the parity baseline checks) are
  asserted equal, so a fix applied to one and missed on the other is a failure
  rather than a silent divergence between production and its own release gate.

### `validate_config`

Every input is checked against `spec::expected_validate_config`:

- **It accepts exactly the documented set** — not a superset, not a subset.
- **It returns the documented error, not merely an error.** The precedence
  (severity → threshold → penalty → reward → severity-specific →
  cross-parameter) is therefore a tested property, and the error code stays a
  deterministic function of the inputs that backends can branch on.
- **Severity-specific bounds** for all four canonical severities.
- **Cross-parameter consistency**: `penalty * 3 < reward * 2`.
- **Non-canonical severities are always rejected** with `InvalidSeverity`.
- **Validation is sufficient, not merely necessary.** Anything it accepts is run
  through `compute_result` for several MTTRs including `u32::MAX`, and must
  produce a usable settlement. This is the cross-check that ties the two
  documented surfaces together.

### `config_mutation_sequences`

Stateful. Generates sequences of `set_config`, `set_custom_severity`,
`remove_custom_severity`, `set_retention_limit` and `calculate_sla` and asserts
that no sequence leaves the contract inconsistent: canonical snapshot shape and
ordering, cross-severity penalty ordering, retention bounds, custom severities
never shadowing canonical names, and history staying bounded and well-formed.

## What the fuzz suite does not guarantee

Stating the gaps is as important as stating the coverage.

- **Authorization and lifecycle are not fuzzed.** `require_admin`,
  `require_operator`, pause/unpause, config freeze and the two-step governance
  transfers are covered by `auth_matrix_tests` and `tests.rs`, not here.
- **Event payloads are not asserted by the pure targets.** `compute_result` and
  `validate_config` emit nothing. Event shape and topic stability are covered by
  `event_schema.rs`, `topic_stability_tests.rs` and `event_ordering_tests.rs`.
- **Duplicate detection and replay are not fuzzed directly.** The
  `DuplicateOutageInput` / `OutageRecalcLimit` decision table is unit-tested;
  `config_mutation_sequences` exercises it only incidentally.
- **Storage cost and footprint are out of scope.** See
  `storage_footprint_tests.rs` and `docs/STORAGE_FOOTPRINT_POLICY.md`.
- **Cross-contract behaviour is covered by unit and property tests, not the pure fuzz targets.** `cross_contract_safety.rs` checks call status and depth boundaries; `fuzz_tests.rs` generates call/finalize/rollback sequences and asserts guard depth, remaining capacity, and pending-state invariants after every step.
- **The pure targets do not construct an `Env`.** They exercise pure functions
  only; nothing about storage, ledger state or the budget is asserted by them.
- **Fuzzing proves absence of counterexamples found, not absence of bugs.** A
  green nightly run means libFuzzer did not find an input that violates the
  asserted invariants within its budget. Invariants nobody wrote down are not
  checked.

## Where the spec lives

The durable problem with assertion-heavy fuzzing is that encoding the spec in
the target duplicates it — the same drift the assertions are meant to catch.
This repository resolves it as follows:

```
apexchainx_calculator/src/spec.rs        ← the executable spec (no_std, dependency-free)
apexchainx_calculator/src/fuzz_spec.rs   ← assertion bodies comparing impl to spec
apexchainx_calculator/fuzz/fuzz_targets/ ← input decoding only, a few lines each
```

- **`spec.rs`** is an independent restatement of the documented rules. It
  deliberately does *not* call the contract; `impl == spec` is only meaningful
  when the two sides are written separately. Making any function there delegate
  to the implementation would silently void every test and target built on it.
- **`fuzz_spec.rs`** holds every assertion the targets make. It lives in the
  library, so `cargo test --lib` type-checks it on every commit and its unit
  tests exercise it on fixed boundary vectors. This matters more than it may
  look: the fuzz crate needs nightly, `cargo-fuzz` and a C++ toolchain for
  `libfuzzer-sys`, so assertion logic written inside a target is invisible to
  every routine check and can rot — or fail to compile — unnoticed. It also
  settles how a target reaches contract internals: `compute_result` is private
  and `validate_config` is `pub(crate)`, both inside the `#[contractimpl]`
  block, so widening them to `pub` would add them to the deployed ABI. A
  descendant module can call them without changing a single exported symbol.
- **The targets** decode fuzzer bytes and call into `fuzz_spec`. Keeping them
  thin is what keeps the spec in one place.

The practical consequence: **the fuzz suite's contract is enforced by
`just test`.** The nightly campaign adds coverage-guided input search on top of
assertions that are already being compiled and exercised on every commit.

## Which statement is authoritative

Assertion-heavy fuzzing finds two kinds of failure, and they need different
responses. When a target reports `impl != spec`, exactly one of three things is
true, and they are resolved in this order:

1. **The implementation regressed.** Fix the implementation. This is the outcome
   the targets exist to produce.
2. **The spec restatement is wrong** — a transcription slip in `spec.rs`. Fix
   `spec.rs`. Nothing else changes.
3. **The behaviour changed deliberately** and the documented rule is now stale.
   Then, and only then, update `spec.rs` — *and* the prose docs it cites, *and*
   `test_snapshots/tests/parity_baseline.json` plus the inline table in
   `parity_tests.rs` — in the same commit, and record it in `CHANGELOG.md`.

Case 3 is a reviewed, deliberate break. **It is never correct to edit `spec.rs`
solely to turn a red build green.** If you cannot say which of the three cases
applies, the answer is not case 3.

The same ordering resolves code-versus-prose conflicts elsewhere: the contract
implementation is the canonical source of truth, and a document that disagrees
with it is a document to be corrected (this is already the stated policy in
`docs/HISTORY_PAGINATION_POLICY.md` § Canonical source of truth).

## Running the suite

```sh
just fuzz          # proptest property tests — stable toolchain, seconds
just test          # includes the fuzz_spec unit tests that pin the invariants
just fuzz-build    # compile the cargo-fuzz targets (nightly + cargo-fuzz)
just fuzz-run compute_result 60   # one coverage-guided campaign, 60s
```

`just fuzz-build` is the cheap check worth running after touching anything in
`fuzz/`, `spec.rs` or `fuzz_spec.rs`: it catches a target that no longer
compiles without waiting for the nightly job.

## Adding or changing a target

1. Write the invariant as an assertion body in `fuzz_spec.rs`, with a doc
   comment listing what it asserts *beyond panic-freedom* and why each item
   matters.
2. Add a unit test in `fuzz_spec.rs` that exercises it on the boundary vectors —
   this is what keeps it working on the stable toolchain.
3. Keep the target itself to input decoding plus a call.
4. Give the target a header covering: what it asserts, what it does not, and a
   pointer back here for the authority policy.
5. Add the target to the matrix in `.github/workflows/fuzz.yml` and to
   `fuzz/Cargo.toml`.
6. Verify the assertion actually bites — break the implementation on purpose,
   confirm the unit tests fail, then revert. An assertion that cannot fail is
   worse than none, because it reads as coverage.
