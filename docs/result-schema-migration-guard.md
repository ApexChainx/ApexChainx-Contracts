# Result Schema Migration Guard

> **Safe migration guardrail for `get_result_schema()` when the result layout changes.**
>
> Resolves #255.  This document describes the mechanism used to prevent
> `SLAResult` layout changes from silently breaking backend consumers, the
> process for making a deliberate schema change, and what CI checks to expect.

---

## Table of Contents

1. [Problem](#problem)
2. [Mechanism](#mechanism)
3. [Constants](#constants)
4. [The `result_field_count` field in `SLAResultSchema`](#the-result_field_count-field-in-slaresultschema)
5. [CI-backed tests (`schema_migration_tests.rs`)](#ci-backed-tests-schema_migration_testsrs)
6. [Process for changing `SLAResult`](#process-for-changing-slaresult)
7. [Process for deprecating a symbol](#process-for-deprecating-a-symbol)
8. [Backend consumer guidance](#backend-consumer-guidance)
9. [Release process requirements](#release-process-requirements)

---

## Problem

`get_result_schema()` is the primary compatibility boundary between the contract
and backend consumers.  Before this guardrail existed:

- A contributor could add or rename a field in `SLAResult` without bumping
  `RESULT_SCHEMA_VERSION`.
- The contract would continue to compile and pass its existing tests.
- Backend consumers would deserialize the wrong fields or miss new ones silently —
  no compile error, no test failure, only corrupt data at runtime.

---

## Mechanism

The guardrail operates on two levels:

### Level 1 — Compile-time

`schema_migration_tests.rs` contains a fully destructured `SLAResult` pattern
match:

```rust
let SLAResult {
    outage_id: _,
    status: _,
    mttr_minutes: _,
    threshold_minutes: _,
    amount: _,
    payment_type: _,
    rating: _,
    config_version_hash: _,
    recorded_at: _,
} = sample;
```

If a field is added or removed from `SLAResult`, this destructure will **fail to
compile** — surfacing the change before any runtime tests run.

### Level 2 — Runtime (CI)

Four tests in `schema_migration_tests.rs` assert:

| Test | What it checks |
|---|---|
| `test_result_schema_field_count_sentinel` | `RESULT_SCHEMA_FIELD_COUNT == 9` (current field count) |
| `test_get_result_schema_version_matches_constant` | `get_result_schema()` returns `schema_version == RESULT_SCHEMA_VERSION` and `result_field_count == RESULT_SCHEMA_FIELD_COUNT` |
| `test_result_schema_symbols_are_stable` | Every symbol in `get_result_schema()` matches the canonical value baked into `compute_result` |
| `test_result_schema_no_deprecated_symbols_at_v1` | `deprecated_symbols` list is empty for schema v1 |
| `test_config_bundle_schema_version_consistent` | `get_config_bundle()` embeds the same schema version and field count |

Any layout change that is not reflected in the constants will cause at least one
of these tests to fail in CI.

---

## Constants

| Constant | Location | Value | Meaning |
|---|---|---|---|
| `RESULT_SCHEMA_VERSION` | `lib.rs` | `1` | Breaking-change counter for `SLAResult` layout |
| `RESULT_SCHEMA_FIELD_COUNT` | `lib.rs` | `9` | Number of named fields in `SLAResult` |

Both constants are exposed through `get_result_schema()` so backend consumers
can detect drift at runtime without hardcoding field lists.

---

## The `result_field_count` field in `SLAResultSchema`

`SLAResultSchema` now includes:

```rust
/// Number of named fields in `SLAResult` at this schema version.
pub result_field_count: u32,
```

Backend consumers can call `get_result_schema()` at startup and compare
`result_field_count` against their own deserialization code.  A mismatch signals
that the contract binary is newer or older than the backend expects.

---

## CI-backed tests (`schema_migration_tests.rs`)

The test file lives at:

```
apexchainx_calculator/src/schema_migration_tests.rs
```

It is declared as a `#[cfg(test)]` module in `lib.rs` and runs as part of the
standard `cargo test --lib` step in CI.  No special flags are needed.

To run the guardrail tests locally:

```bash
cd apexchainx_calculator
cargo test schema_migration_tests
```

---

## Process for changing `SLAResult`

> Follow these steps in order.  All steps must be in the same PR.

### Step 1 — Modify `SLAResult`

Add, remove, or rename a field in the `SLAResult` struct in `lib.rs`.

### Step 2 — Fix the compile-time destructure in `schema_migration_tests.rs`

Update the exhaustive destructure in `test_result_schema_field_count_sentinel`
to include the new field.  The test will not compile until you do this.

### Step 3 — Update `RESULT_SCHEMA_FIELD_COUNT`

Change the value in `lib.rs` to match the new field count.

### Step 4 — Increment `RESULT_SCHEMA_VERSION`

Every field-level change to `SLAResult` is a **breaking change** for backend
consumers.  Increment the constant in `lib.rs`:

```rust
pub(crate) const RESULT_SCHEMA_VERSION: u32 = 2; // was 1
```

### Step 5 — Update `get_result_schema()` if needed

If a new symbol descriptor is required (e.g. for a new status or payment type),
add it to `SLAResultSchema` and populate it in `get_result_schema()`.

### Step 6 — Update the symbol stability test

In `test_result_schema_symbols_are_stable`, add assertions for any new symbol
fields.  If an existing symbol value changes, update the expected string.

### Step 7 — Update `CHANGELOG.md`

Under `[Unreleased]` → `Changed`:

```markdown
- `SLAResult` — added `<field_name>: <Type>` (breaking); `RESULT_SCHEMA_VERSION`
  bumped from 1 to 2 (closes #NNN)
```

### Step 8 — Notify backend consumers

The `apexchainx-be` team must be notified before the PR is merged.  Provide:
- The new `RESULT_SCHEMA_VERSION` value
- The added/removed/changed field name and type
- A migration note in the PR description explaining how the backend should adapt

### Step 9 — Run CI locally

```bash
cd apexchainx_calculator
cargo fmt
cargo clippy -- -D warnings
cargo test --lib
cargo check --target wasm32-unknown-unknown --lib
```

---

## Process for deprecating a symbol

When a result symbol (e.g. the value of `status_met`) is being replaced:

1. Continue emitting the old symbol (backward-compatible coexistence period).
2. Add a `DeprecatedSymbol` entry to `deprecated_symbols` in `get_result_schema()`.
3. Update `test_result_schema_no_deprecated_symbols_at_v1` to assert the entry
   is present rather than asserting the list is empty.
4. Announce the removal version in `deprecated_at` and `removal_version`.
5. In the release after `removal_version`, remove the old symbol and update all
   tests.

---

## Backend consumer guidance

| Signal | Action |
|---|---|
| `schema_version` unchanged, `result_field_count` unchanged | No change needed; proceed normally |
| `schema_version` bumped | Schema breaking change; redeploy backend adapter before or at the same release |
| `result_field_count` larger | New field present; old deserialization code may miss it — update and test |
| `result_field_count` smaller | Field removed; old deserialization code may error — update and test |
| `deprecated_symbols` non-empty | Start migration off the old symbol before `removal_version` is reached |

Backends should call `get_result_schema()` (or `get_config_bundle()`) once at
startup and compare `schema_version` and `result_field_count` against compiled-in
expectations.  A mismatch should block operations and alert on-call until the
backend and contract versions are aligned.

---

## Release process requirements

Before any release that includes a schema change:

- [ ] `RESULT_SCHEMA_VERSION` incremented
- [ ] `RESULT_SCHEMA_FIELD_COUNT` updated
- [ ] All four schema migration tests pass (`cargo test schema_migration_tests`)
- [ ] `CHANGELOG.md` updated with the breaking change note
- [ ] `apexchainx-be` team notified and adapter PR is merged or in flight
- [ ] PR description contains a **Schema Migration Note** section:

```markdown
## Schema Migration Note

`RESULT_SCHEMA_VERSION` bumped from X to Y.

**Changed field(s):**
- Added `<field_name>: <Type>` — <reason>

**Backend action required:**
- Update deserialization to handle the new field
- Re-run backend parity tests against the new WASM binary
```
