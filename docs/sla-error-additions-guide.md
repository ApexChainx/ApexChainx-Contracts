# SLAError Addition Guide

> **Contributor guide for introducing new error categories in `SLAError`.**
>
> This document covers the complete workflow for adding, deprecating, or
> reorganising error codes in `apexchainx_calculator/src/lib.rs` and the
> companion typed-helper layer in `apexchainx_calculator/src/error_responses.rs`.
>
> A new error code is a **contract interface change**, even when it looks like
> an internal detail.  Backend consumers that rely on the numeric discriminant
> or the `get_failure_schema()` catalogue must be able to adapt without silent
> data corruption or panics.  Follow this guide in full before opening a PR.

---

## Table of Contents

1. [Why error-code additions are interface changes](#why-error-code-additions-are-interface-changes)
2. [The SLAError enum and its discriminants](#the-slaerror-enum-and-its-discriminants)
3. [The typed helper layer (`error_responses.rs`)](#the-typed-helper-layer-error_responsesrs)
4. [Step-by-step: adding a new error category](#step-by-step-adding-a-new-error-category)
5. [Step-by-step: deprecating an error category](#step-by-step-deprecating-an-error-category)
6. [Compatibility expectations for backend consumers](#compatibility-expectations-for-backend-consumers)
7. [Testing requirements](#testing-requirements)
8. [Checklist before opening a PR](#checklist-before-opening-a-pr)
9. [Reference: current error catalogue](#reference-current-error-catalogue)

---

## Why error-code additions are interface changes

The `SLAError` enum is compiled into the contract's WASM binary and serialised
on-chain as a `u32` discriminant whenever a call fails.  Backend consumers
(the `apexchainx-be` bridge and any downstream indexers) receive this
discriminant and map it to a human-readable label via `get_failure_schema()`.

A change to the enum affects consumers in the following ways:

| Change type | Consumer impact |
|---|---|
| **Append a new variant** | Consumers that call `get_failure_schema()` see a new entry. Old consumers that do not recognise the code surface "unknown error"; they do **not** misinterpret an existing code. Safe, but must be communicated. |
| **Remove or renumber an existing variant** | Any consumer that hard-codes the numeric code or calls `get_failure_schema()` to build a lookup table will silently misidentify errors. **Breaking — never do this.** |
| **Rename a variant (same discriminant)** | The numeric wire format is unchanged. The `get_failure_schema()` label changes. Consumers using the label as a stable key are broken. Treat as **breaking** — coordinate with the `apexchainx-be` team. |
| **Change a variant's `#[doc]` comment only** | No wire format impact. Safe. |

---

## The SLAError enum and its discriminants

`SLAError` lives in `apexchainx_calculator/src/lib.rs` and is annotated with
`#[contracterror]` and `#[repr(u32)]`:

```rust
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum SLAError {
    AlreadyInitialized    = 1,
    NotInitialized        = 2,
    Unauthorized          = 3,
    // … (see reference catalogue below)
    OutageRecalcLimit     = 19,
}
```

**Rules that must never be violated:**

1. Discriminant values are permanent. Once `FooError = 7` ships in a released
   binary, `7` is reserved for `FooError` for the lifetime of the contract.
2. New variants must use the **next available integer** (currently `20` for the
   first new addition after the existing 19).  Do not use gaps or reorder.
3. Every variant needs a `/// <description>` doc comment that explains the
   error condition precisely enough that a backend engineer can write a recovery
   path without reading the contract source.

---

## The typed helper layer (`error_responses.rs`)

`apexchainx_calculator/src/error_responses.rs` exposes a `is_<variant_name>`
predicate for every variant in `SLAError`.  Backend consumers and integration
tests use these predicates instead of matching on raw discriminants, so a
rename only requires updating the predicate — not every call site.

Example (existing):

```rust
pub fn is_outage_recalc_limit(err: &SLAError) -> bool {
    matches!(err, SLAError::OutageRecalcLimit)
}
```

**Every new variant in `SLAError` must have a matching predicate added to
`error_responses.rs` in the same PR.**

The predicate naming convention is `is_<snake_case_variant_name>`.

---

## Step-by-step: adding a new error category

### 1. Confirm the error is not already covered

Search the existing catalogue (see [Reference](#reference-current-error-catalogue))
and `get_failure_schema()` in `lib.rs`.  A new variant is only warranted if no
existing code describes the failure precisely enough.

### 2. Choose the next discriminant

Inspect the current highest discriminant in `SLAError`.  As of this writing it
is `OutageRecalcLimit = 19`.  Assign `20` to the first new variant, `21` to the
second, and so on.

### 3. Add the variant to `SLAError` in `lib.rs`

Append at the **end** of the enum:

```rust
/// <One-sentence summary of the condition that triggers this error.>
///
/// # Semantics
///
/// <Explain when exactly this error is returned.  Include a decision table
/// if the condition depends on multiple input states.>
///
/// # Consumer guidance
///
/// <What should a backend caller do when it receives this error?
/// E.g. "Retry with corrected input", "Alert admin", "Surface to user".>
MyNewError = 20,
```

> **Do not** insert the variant in the middle of the enum — append only.

### 4. Add the predicate to `error_responses.rs`

```rust
pub fn is_my_new_error(err: &SLAError) -> bool {
    matches!(err, SLAError::MyNewError)
}
```

### 5. Update `get_failure_schema()` in `lib.rs`

Extend the `entries` array in `get_failure_schema()`.  The array is in
ascending numeric order; append the new entry at the end:

```rust
let entries: [(u32, &str, &str); 20] = [
    // … existing 19 entries …
    (20, "MyNewError", "Short description ≤ 32 chars"),
];
```

Both the label (`&str` for `Symbol::new`) and the description must be **≤ 32
bytes** to satisfy the Soroban `Symbol` size constraint.  Use abbreviated
wording if needed (e.g. `"Calr lacks req role"` instead of
`"Caller lacks required role"`).

Also update the array length literal in the type annotation from `; 19]` to
`; 20]` (or whatever the new count is).

### 6. Emit the new error at the appropriate contract site

Return `Err(SLAError::MyNewError)` at every point in the contract logic where
this failure condition is detected.  Include a comment referencing the issue
that motivated the new error.

### 7. Update `CHANGELOG.md`

Under `[Unreleased]` → `Changed`:

```markdown
- `SLAError` — added `MyNewError = 20` (short description of condition)
  (closes #NNN)
```

If this is the first `SLAError` change in the release, open a new sub-heading:

```markdown
### Changed
- `SLAError` — added `MyNewError = 20` …
```

### 8. Write or update tests

See [Testing requirements](#testing-requirements).

### 9. Run the full CI pipeline locally

```bash
cd apexchainx_calculator
cargo fmt
cargo clippy -- -D warnings
cargo test --lib
cargo check --target wasm32-unknown-unknown --lib
```

All four commands must exit 0 before pushing.

---

## Step-by-step: deprecating an error category

Error codes are **never removed**.  If a variant becomes unreachable due to a
refactor, mark it as deprecated in a doc comment but leave the discriminant
in place:

```rust
/// **Deprecated** — no longer returned as of schema version N.
/// Kept for wire-format stability; backend consumers may ignore this code.
/// Replacement: `SLAError::MyNewError = 20`.
OldError = 7,
```

Update the `description` in `get_failure_schema()` to include `"[deprecated]"`
so callers that load the schema at runtime are informed:

```rust
(7, "OldError", "[deprecated] Use MyNewError"),
```

Document the deprecation in `CHANGELOG.md` under `Changed`.

---

## Compatibility expectations for backend consumers

| Guarantee | Detail |
|---|---|
| **Numeric stability** | Once shipped, `SLAError::Foo = N` means `N` maps to `Foo` forever. The backend may cache this mapping indefinitely. |
| **Additive additions are non-breaking** | A new variant appended at the end does not invalidate any existing mapping. The backend should treat an unrecognised code as `"unknown_error"` and surface it as a warning, not a crash. |
| **Label stability** | `get_failure_schema()` label strings are considered stable once a version is released. Changing a label is a **breaking change** that requires coordination with the `apexchainx-be` team and a version bump in the schema's `version` field. |
| **At least one release cycle notice** | If a change touches existing variant names or discriminants, the `apexchainx-be` team must be notified at least one release cycle in advance so they can deploy an adapter before the breaking binary reaches the network. |

---

## Testing requirements

Every PR that modifies `SLAError` or `error_responses.rs` must include:

### Unit test for the new error path

Write a test in `apexchainx_calculator/src/tests.rs` (or a dedicated test
module) that triggers the exact contract function that returns the new error and
asserts the correct discriminant:

```rust
#[test]
fn test_my_new_error_returned_on_condition_x() {
    // … set up env, initialize contract …
    let result = SLACalculatorContract::some_function(env, …);
    assert!(result.is_err());
    assert!(error_responses::is_my_new_error(&result.unwrap_err()));
}
```

### Predicate smoke test

Add a quick smoke test for the new predicate in `error_responses.rs` alongside
the existing tests (or create `tests/error_responses_tests.rs`):

```rust
#[test]
fn test_is_my_new_error_predicate() {
    assert!(error_responses::is_my_new_error(&SLAError::MyNewError));
    assert!(!error_responses::is_my_new_error(&SLAError::Unauthorized));
}
```

### `get_failure_schema()` catalogue coverage test

Verify that `get_failure_schema()` returns exactly the expected number of
entries and that each new entry is present.  A test template:

```rust
#[test]
fn test_failure_schema_includes_new_error() {
    let env = Env::default();
    // … initialize contract …
    let schema = SLACalculatorContract::get_failure_schema(env).unwrap();
    // Update the expected count to match the new total
    assert_eq!(schema.codes.len(), 20);
    let my_code = schema.codes.iter().find(|c| c.code == 20);
    assert!(my_code.is_some());
}
```

---

## Checklist before opening a PR

Use this checklist for every PR that adds, renames, or deprecates an `SLAError`
variant:

- [ ] New variant is appended at the end of the enum — not inserted in the middle
- [ ] Discriminant is the next available integer (no gaps)
- [ ] Variant has a `///` doc comment with at least: a one-line summary, a
      semantics section, and a consumer guidance section
- [ ] Matching `is_<variant>` predicate added to `error_responses.rs`
- [ ] `get_failure_schema()` updated: new entry added to `entries` array and
      array length constant incremented
- [ ] New error is returned at the correct contract site(s)
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] Unit test for the contract path that returns the new error
- [ ] Predicate smoke test
- [ ] `get_failure_schema()` catalogue coverage test
- [ ] `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --lib`, and
      `cargo check --target wasm32-unknown-unknown --lib` all pass
- [ ] `apexchainx-be` team notified if any existing label or discriminant changed

---

## Reference: current error catalogue

| Discriminant | Variant | Short description |
|---:|---|---|
| 1 | `AlreadyInitialized` | Contract already initialized |
| 2 | `NotInitialized` | Contract not yet initialized |
| 3 | `Unauthorized` | Caller lacks required role |
| 4 | `ConfigNotFound` | No config for severity |
| 5 | `VersionMismatch` | Storage version mismatch |
| 6 | `ContractPaused` | Contract is paused |
| 7 | `NoPendingTransfer` | No pending transfer |
| 8 | `InvalidThreshold` | Threshold out of range |
| 9 | `InvalidPenalty` | Penalty out of range |
| 10 | `InvalidReward` | Reward out of range |
| 11 | `InvalidSeverity` | Severity not supported |
| 12 | `RetentionLimitOutOfRange` | Retention limit out of range |
| 13 | `DuplicateOutageInput` | Conflicting duplicate outage_id |
| 14 | `InvalidPenaltyAmount` | Invalid penalty amount |
| 15 | `InvalidRewardAmount` | Invalid reward amount |
| 16 | `ConfigFrozen` | Configuration is frozen |
| 17 | `InvalidInput` | Invalid input parameter |
| 18 | `SeverityNotInSet` | Custom severity not registered |
| 19 | `OutageRecalcLimit` | Outage recalc limit reached |
| 20 | `ProposalExpired` | Proposal expired |
| 21 | `AdminRenounced` | Admin authority renounced |
| 22 | `ExposureOverflow` | Exposure totals overflow i128 |
| **23+** | *(next available)* | *(reserved for future additions)* |

> This table is auto-derived from `get_failure_schema()` in `lib.rs`.  Keep
> both in sync whenever the catalogue changes.
