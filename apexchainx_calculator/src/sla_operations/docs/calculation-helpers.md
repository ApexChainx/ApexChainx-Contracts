# Calculation and Analysis Helpers

This guide covers the pure calculation helpers in the SLA operations
workspace. They turn validated metrics, durations, counters, and policy
snapshots into deterministic values that reporting and fixture modules can
compose.

Everything in this workspace is additive and isolated from the deployed
contract:

- no storage reads or writes,
- no contract entrypoints,
- no events or event topics,
- no ABI changes.

The helpers take plain values and slices. They never receive an `Env`, so they
cannot observe ledger state, wall-clock time, or randomness.

## Location and registration

All modules live under:

```text
apexchainx_calculator/src/sla_operations/
```

They are declared in `sla_operations/mod.rs` (or the relevant `mod.rs` for a
subdirectory). A new `.rs` file is not compiled until it is declared from its
parent module. Keep the declaration in the workspace; do not wire helpers into
`lib.rs` or the contract root.

## Shared conventions

- **Basis points:** `1 bp = 0.01%`; `10_000 bps = 100%`. Helpers that can
  exceed the whole document it explicitly.
- **Truncation:** integer division truncates toward zero. Helpers do not round
  unless their documentation says otherwise.
- **Undefined values:** a zero denominator or a non-positive total returns
  `None`; callers choose how to display the absence of data.
- **Extremes:** arithmetic uses checked or saturating operations. A helper
  documents whether a saturated total is still an exact sum.
- **No panics:** caller-controlled inputs are rejected with `Option` or a small
  `Result` error enum rather than `panic!`, indexing, or unwrap.
- **`no_std`:** use `core` and `alloc` only. No `std`, no floating point in the
  contract path, and no platform-dependent behavior.

## Module reference

### `duration_utils`

Path: `duration_utils.rs`

Normalizes and bounds outage intervals.

- `DurationWindow::new(start, end)` rejects `end < start` with
  `DurationError::EndBeforeStart`; `duration()` returns `end - start` and
  `overlaps(other)` treats touching endpoints as non-overlapping.
- `normalize_duration(start, end)` returns `Some(end - start)` or `None` for an
  inverted range.
- `clamp_duration(duration, maximum)` caps a duration at `maximum`.
- `total_duration(&[u64])` sums with saturation at `u64::MAX`.

```rust
use apexchainx_calculator::sla_operations::duration_utils::{
    clamp_duration, normalize_duration, total_duration, DurationWindow,
};

let window = DurationWindow::new(10, 40).unwrap();
assert_eq!(window.duration(), 30);
assert_eq!(normalize_duration(20, 10), None);
assert_eq!(total_duration(&[10, 20, 30]), 60);
assert_eq!(clamp_duration(100, 60), 60);
```

### `violation_rate`

Path: `violation_rate.rs`

Converts violation counts into rates in basis points.

- `violation_rate_bps(violated, total)` returns `None` when `total == 0` or
  `violated > total`.
- `compliance_rate_bps(violated, total)` returns the complement
  (`10_000 - rate`) under the same validity rules.
- `violation_rate(violated, total)` returns a `ViolationRate` carrying the
  counts and the rate.
- `within_violation_budget(violated, total, maximum_bps)` compares a valid rate
  with a budget and returns `None` for invalid counts.

```rust
use apexchainx_calculator::sla_operations::violation_rate::{
    compliance_rate_bps, violation_rate_bps, within_violation_budget,
};

assert_eq!(violation_rate_bps(2, 8), Some(2_500));
assert_eq!(compliance_rate_bps(2, 8), Some(7_500));
assert_eq!(violation_rate_bps(0, 0), None);
assert_eq!(violation_rate_bps(9, 8), None);
assert_eq!(within_violation_budget(2, 8, 2_500), Some(true));
```

### `compliance_score`

Path: `compliance_score.rs`

Converts met/violated counts into a compliance score. This is the
complement-oriented counterpart to `violation_rate`:

- `compliance_score_bps(met, violated)` returns `None` when both counts are
  zero, because there is nothing to be compliant with.
- `non_compliance_score_bps(met, violated)` returns the complement.
- `compliance_score(met, violated)` returns a `ComplianceScore` with counts.
- `meets_compliance_target(met, violated, target_bps)` compares a valid score
  with a target.

```rust
use apexchainx_calculator::sla_operations::compliance_score::{
    compliance_score_bps, meets_compliance_target, non_compliance_score_bps,
};

assert_eq!(compliance_score_bps(95, 5), Some(9_500));
assert_eq!(non_compliance_score_bps(95, 5), Some(500));
assert_eq!(compliance_score_bps(0, 0), None);
assert_eq!(meets_compliance_target(95, 5, 9_500), Some(true));
```

### `counter_deltas`

Path: `counter_deltas.rs`

Compares monotonically increasing counters and summarizes a batch.

- `counter_delta(current, previous)` returns a `CounterDelta` with movement,
  absolute magnitude, and signed `net`.
- `saturating_growth(current, previous)` and
  `saturating_decline(current, previous)` return a non-negative change.
- `summarize_counters(&[(previous, current)])` counts increases, decreases, and
  unchanged readings and saturates the aggregate growth and decline totals.

Note the argument order: the single comparison takes `(current, previous)`,
while the summary takes `(previous, current)` pairs.

```rust
use apexchainx_calculator::sla_operations::counter_deltas::{
    counter_delta, summarize_counters, CounterMovement,
};

let delta = counter_delta(120, 100);
assert_eq!(delta.movement, CounterMovement::Increased);
assert_eq!((delta.magnitude, delta.net), (20, 20));

let summary = summarize_counters(&[(100, 120), (50, 40), (7, 7)]);
assert_eq!((summary.increases, summary.decreases, summary.unchanged), (1, 1, 1));
assert_eq!((summary.growth_total, summary.decline_total), (20, 10));
```

### `mttr_classification`

Path: `mttr_classification.rs`

Classifies an MTTR against ascending warning and breach thresholds.

- `classify_mttr(mttr_secs, warning_secs, breach_secs)` returns `None` when
  `warning_secs > breach_secs` or `breach_secs == 0`.
- The band is `Healthy` at or below the warning threshold, `Warning` above the
  warning but at or below the breach threshold, and `Breached` above the breach
  threshold. The `exceeded_*` flags use strict comparisons.
- `meets_mttr_threshold(mttr, threshold)` uses the inclusive rule.
- `band_label(band)` returns a stable lowercase label.

```rust
use apexchainx_calculator::sla_operations::mttr_classification::{
    classify_mttr, meets_mttr_threshold, MttrBand,
};

let healthy = classify_mttr(30, 30, 60).unwrap();
assert_eq!(healthy.band, MttrBand::Healthy);

let warning = classify_mttr(45, 30, 60).unwrap();
assert_eq!(warning.band, MttrBand::Warning);
assert!(warning.exceeded_warning);
assert!(!warning.exceeded_breach);

assert!(meets_mttr_threshold(60, 60));
assert!(!meets_mttr_threshold(61, 60));
assert_eq!(classify_mttr(10, 60, 30), None);
```

### `policy_delta`

Path: `policy_delta.rs`

Compares two policy snapshots.

- `compare_policy(previous, current)` returns a `PolicyDeltaSummary` with one
  flag per field and the helpers `change_count()` / `is_unchanged()`.
- `policy_changes(previous, current, out)` writes at most `out.len()` changed
  fields in a fixed order (threshold, payout, enabled) and returns the number
  written. Booleans are encoded as `0` or `1`.

```rust
use apexchainx_calculator::sla_operations::policy_delta::{
    compare_policy, policy_changes, PolicyDelta, PolicyField, PolicySnapshot,
};

let previous = PolicySnapshot { threshold_secs: 3_600, payout_bps: 500, enabled: true };
let current = PolicySnapshot { threshold_secs: 3_000, payout_bps: 500, enabled: false };

let summary = compare_policy(previous, current);
assert_eq!(summary.change_count(), 2);
assert!(summary.threshold_changed);
assert!(summary.enabled_changed);

let mut changes = [PolicyDelta {
    field: PolicyField::ThresholdSecs,
    previous: 0,
    current: 0,
}];
assert_eq!(policy_changes(previous, current, &mut changes), 1);
assert_eq!(changes[0].current, 3_000);
```

### `rolling_windows`

Path: `rolling_windows.rs`

Computes aggregates over an ordered slice of samples.

- `window_sum`, `window_average`, `window_max`, and `window_min` take
  `(samples, start, count)`.
- Every window helper returns `None` when `count == 0`, `start + count`
  overflows, or the window extends past the slice.
- `aggregate_window` returns a `WindowAggregate` with `sum`, `count`, and
  integer `average`.

```rust
use apexchainx_calculator::sla_operations::rolling_windows::{
    aggregate_window, window_average, window_max, window_sum,
};

let samples = [10_u64, 20, 30, 40];
assert_eq!(window_sum(&samples, 1, 2), Some(50));
assert_eq!(window_average(&samples, 1, 2), Some(25));
assert_eq!(window_max(&samples, 0, 3), Some(30));
assert_eq!(window_sum(&samples, 3, 2), None);

let aggregate = aggregate_window(&samples, 0, 4).unwrap();
assert_eq!((aggregate.sum, aggregate.count, aggregate.average), (100, 4, Some(25)));
```

### `severity_weighting`

Path: `severity_weighting.rs`

Maps the four canonical severities to weights in basis points: low `2_500`,
medium `5_000`, high `7_500`, critical `10_000`.

- `weighted_score_bps(severity, score_bps)` scales a score by the weight.
- `total_weight_bps(&[Severity])` saturates at `u64::MAX`.
- `average_weight_bps(&[Severity])` returns `None` for an empty slice.
- `Severity::label()` and `Severity::weight_bps()` expose stable values.

```rust
use apexchainx_calculator::sla_operations::severity_weighting::{
    average_weight_bps, weighted_score_bps, Severity,
};

assert_eq!(Severity::Critical.weight_bps(), 10_000);
assert_eq!(weighted_score_bps(Severity::Low, 7_500), 1_875);
assert_eq!(average_weight_bps(&[Severity::Low, Severity::Critical]), Some(6_250));
assert_eq!(average_weight_bps(&[]), None);
```

## Validation and reporting

The `validation/` directory contains pure input and output checks:

- `validation/threshold_bounds.rs` — inclusive threshold ranges and values,
- `validation/aggregation_inputs.rs` — record batches and counter sanity,
- `validation/invariants.rs` — basis-point ranges, count sums, share sums,
  settlement nets, and ordering,
- `validation/report_windows.rs` — report windows and bucket counts.

The `reporting/` directory builds summaries and render-ready values from plain
records: SLA summary, compliance report, outage analytics, severity
distribution, time series, history export, settlement summary, telemetry
snapshot, deltas, health indicators, metadata, CSV rows, JSON models, and
status labels. These modules are documented in their module headers.

Validation helpers return a specific error enum so callers can distinguish
`Empty`, `ZeroThreshold`, `ViolationsExceedTotal`, `BpsOutOfRange`,
`InvertedRange`, and the other documented cases. Do not flatten them into a
boolean when the caller needs to react differently.

## Fixtures and composition tests

Deterministic fixtures under `fixtures/` let tests and external examples use
the same inputs without touching contract state:

- `fixtures/sla_inputs.rs` — per-outage input batches,
- `fixtures/threshold_configs.rs` — default, strict, and lenient thresholds,
- `fixtures/boundary_cases.rs` — MTTR/threshold and ratio edge cases,
- `fixtures/pagination_cases.rs` — page requests and resolved bounds,
- `fixtures/anomaly_scenarios.rs` — anomalous SLA metric patterns,
- `fixtures/history_records.rs`, `fixtures/settlement_scenarios.rs`, and
  `fixtures/telemetry_samples.rs` — reporting inputs,
- `fixtures/invalid_inputs.rs`, `fixtures/severity_distributions.rs`,
  `fixtures/time_series.rs`, and `fixtures/golden_vectors.rs` — focused cases.

`pagination_cases::page_bounds(total_items, offset, page_size)` resolves a page
using the same semantics as
`reporting::history_export::to_export_rows_page`:

- a zero page size returns an empty page,
- an offset at or past the end returns an empty page and clamps `start` to
  `total_items`,
- `len` never exceeds `page_size`, and
- `has_more` is true only when items follow the returned page.

`anomaly_scenarios::scenarios()` returns one baseline and six anomalous
patterns. Each fixture exposes predicates such as `has_no_traffic`,
`has_complete_violation`, `has_extreme_mttr`, `has_counter_regression`,
`has_severity_concentration`, and `has_zero_threshold`. Each anomaly fixture is
intended to trip exactly one predicate; keep that isolation when adding cases.

`tests/workspace_composition_tests.rs` drives multiple fixtures and helpers
from one input set and checks that their results agree. Extend that file when a
new helper needs to be proven compatible with the existing summaries, rates,
health indicators, pagination, or severity calculations.

## Contribution boundaries

- Implement changes under `apexchainx_calculator/src/sla_operations/**` only.
- Register new modules in the nearest `mod.rs`, and register tests in
  `sla_operations/tests/mod.rs`.
- Put tests in `sla_operations/tests/` and fixtures in
  `sla_operations/fixtures/`.
- Do not modify core contract entrypoints, storage keys or layout, event
  schemas or topics, or stabilized calculation semantics to make a helper fit.
- If a helper needs a value the workspace does not provide, add a pure
  workspace-local input instead of reading contract state.
- Keep documentation and tests additive; do not renumber or rename existing
  fixtures, cases, patterns, or error variants.

## Checks

Run these from `apexchainx_calculator/` before opening a pull request:

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
cargo check --target wasm32-unknown-unknown --lib
```

The workspace unit tests are part of the crate's library test target. A test
file that is not declared in `tests/mod.rs` never runs, even if it exists on
disk.
