# Coding Style Guide

This document defines the coding style conventions for the ApexChainx Contracts repository, including Soroban `Symbol` short name conventions (#112) and the canonical code comment policy (#260).

---

## Part 1 — Soroban Symbol Short Names (Issue #112)

This section defines the naming convention for Soroban `Symbol` short names used throughout this repository.

### Scope

Apply these rules whenever you define a short `Symbol` in Rust code, especially for:

- contract storage keys
- event names and topics
- status or state identifiers
- other compact symbolic values

### Conventions

1. Keep every short name at 9 characters or fewer.
2. Use lowercase letters only.
3. Use `_` as the only separator when needed.
4. Follow the `<domain>_<sub>` pattern whenever the name has more than one meaningful part.
5. Prefer compact, descriptive names over long or overly abstract ones.

### Recommended Pattern

Use a short domain prefix followed by a concise sub-name:

- `admin`
- `cfg_upd`
- `sla_calc`
- `pruned_a`

### Non-Compliant Examples

These should be avoided:

- `SLA_CALC` (uppercase)
- `cfg-update` (hyphen is not allowed)
- `settlementintent` (too long)
- `outage_status_code` (too long and overly verbose)

### Symbol Review Checklist

Before merging a new short-name symbol, confirm that it:

- [ ] is 9 characters or fewer
- [ ] uses lowercase letters
- [ ] uses `_` only when needed
- [ ] follows the `<domain>_<sub>` pattern where appropriate

### Rationale

Soroban `Symbol` values are compact and must remain short for compatibility and readability. Consistent naming reduces confusion and makes storage keys, event topics, and contract identifiers easier to scan and maintain.

---

## Part 2 — Canonical Code Comment Policy (Issue #260)

This section defines **what must be commented**, **what should not be commented**, and how to distinguish between invariants, public API notes, and implementation details. The goal is a lightweight, enforceable policy that keeps every reader oriented without drowning the code in noise.

### Guiding Principle

> Comment the *why* and the *contract*, not the *what* or the *how*.

Rust's type system and tests communicate *what* and *how* better than prose can. Comments exist to communicate decisions, constraints, and compatibility guarantees that cannot be expressed in types or tests alone.

---

### Comment Categories

The policy recognises three categories of comments. Each has distinct rules for when it is **required**, **optional**, or **prohibited**.

#### Category 1 — Invariants (`// INVARIANT:`)

An *invariant* is a property that must always be true for the contract to behave correctly. Invariants that cannot be enforced by the type system (e.g., ordering requirements, storage-slot uniqueness, mathematical properties) must be documented.

**Required when:**

- A storage key must remain unique and must never be reused for a different semantic purpose across versions.
- A field ordering must be preserved (e.g., event payload tuples, canonical severity order).
- A numeric relationship must hold for correctness (e.g., `penalty * 3 < reward_base * 2`).
- An idempotency property is load-bearing for safety (e.g., `calculate_sla` replay semantics).

**Format:**

```rust
// INVARIANT: storage key "VER" is the storage schema version.
// Once written, this key must never be reused for a different purpose —
// migrate() depends on reading it to determine which steps to apply.
pub(crate) const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");
```

**Prohibited when:**

- The invariant is already enforced by a type constraint (e.g., `u32` can't be negative — don't write `// INVARIANT: always non-negative`).
- The invariant is covered by a test assertion — the test is the canonical source of truth; a comment is redundant.

---

#### Category 2 — Public API Notes (doc comments `///`)

A *public API note* documents a function, type, or constant that is part of the contract's observable interface. Backend consumers, maintainers, and reviewers rely on these to understand the contract surface without reading the implementation.

**Required for every `pub` or `pub(crate)` item:**

- `///` doc comment with a one-sentence summary.
- Arguments listed with `# Arguments` if the function has non-obvious parameters.
- Return value documented with `# Returns` if the return shape is not self-evident.
- Error conditions documented with `# Errors` for every `pub fn` that returns `Result`.
- Issue number reference in the doc comment when the item was introduced to satisfy a tracked issue (e.g., `/// #28 – ...` or `/// SC-021 – ...`).

**Required for `#[contracttype]` structs:**

- Every field must have a `///` doc comment describing what it holds and its unit (e.g., `/// Ledger timestamp in seconds`).

**Optional (not required but encouraged):**

- `# Examples` blocks for public helpers that are non-trivial to use correctly.
- `# Panics` section when a function may panic in unexpected conditions.

**Prohibited:**

- Restating the function name or type name in the comment (`/// Returns the admin address` on a function named `get_admin` adds nothing — write *why* this is safe to call, or what happens if the admin is absent).
- TODOs without an issue reference — every TODO must link to a tracking issue.

---

#### Category 3 — Implementation Detail Notes (`//` inline)

An *implementation detail note* explains a non-obvious algorithm choice, a workaround for a platform limitation, or a performance trade-off. These are the most common inline comments.

**Required when:**

- A numeric formula is non-obvious (e.g., polynomial rolling hash constants, multiplier derivation).
- A Soroban SDK limitation forces an unexpected pattern (e.g., packing four `u32` lanes into one `u128` to avoid `Vec<u32>` serialisation overhead).
- A guard or early-return is present for a non-obvious safety reason (e.g., `// Bypass check_version so callers can reach this in a pre-migration state`).
- A migration step is present — every migration arm must include a comment describing the transformation applied.

**Optional (encouraged for complex logic):**

- Explaining why a particular checked arithmetic method was chosen over a simpler form.
- Describing edge-case handling (e.g., what happens when `threshold_minutes == 0` in a boundary test).

**Prohibited:**

- Commenting obvious code (`// increment counter` above `count += 1`).
- Restating types (`// this is a u32` above a `u32` variable).
- Commented-out code without a `// TODO(#issue):` marker — remove it or track it.

---

### Storage Key Comment Requirements

Every `pub(crate) const *_KEY: Symbol` definition in `lib.rs` must carry:

1. A `///` doc comment summarising what the key stores and its semantic domain.
2. An issue reference if the key was introduced in a tracked issue.
3. An `// INVARIANT:` comment immediately below if the key has a stability or uniqueness constraint that cannot be expressed as a type.

**Example (compliant):**

```rust
/// Admin address — set during initialize, governs config and roles.
pub(crate) const ADMIN_KEY: Symbol = symbol_short!("ADMIN");

/// Current on-chain storage schema version number.
// INVARIANT: Once written, the value stored at STORAGE_VERSION_KEY
// must only be incremented by migrate(). Reading it at any other path
// is a read-only diagnostic; no other code must write to this key.
pub(crate) const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");
```

**Non-compliant (missing domain description):**

```rust
// key for version
pub(crate) const STORAGE_VERSION_KEY: Symbol = symbol_short!("VER");
```

---

### Versioned Event Decision Comments

Every event constant definition must include a comment block that documents the **compatibility decision** for that event. This is distinct from the payload schema (documented in `event_schema.rs`) — it captures *why* the event exists and *what breaking means* for it.

**Required format for every `pub(crate) const EVENT_*: Symbol`:**

```rust
/// Emitted on successful SLA calculation. Primary event for backend consumers.
///
/// Compatibility decision: field additions are appended to the end of the payload
/// tuple and are NOT breaking. Field reordering, removal, or type changes require
/// bumping EVENT_VERSION from "v1" to "v2". See event_schema.rs for the full
/// payload schema and the SC-099 checklist in CONTRIBUTING.md for the review
/// process.
pub(crate) const EVENT_SLA_CALC: Symbol = symbol_short!("sla_calc");
```

The compatibility decision line is mandatory for every event that has a payload. Events that emit only a unit payload `()` need only a short note confirming that the empty payload is intentional.

---

### Migration Boundary Comments

Every arm in the `migrate()` function must carry a comment describing:

1. Which version transition it handles (`// v0 → v1`).
2. What state transformation it applies, or why it applies none (e.g., `// Stamps the version key; all other keys were set by initialize`).
3. What new storage keys the arm introduces (if any) and how defaults are chosen.

**Example (compliant):**

```rust
// v0 → v1: Stamp the storage schema version.
// All storage keys that pre-date versioning were initialised by initialize(),
// so no field-level migration is needed. New installs go through initialize()
// and skip this arm entirely.
if current == 0 {
    Self::init_missing_storage_defaults(&env);
    env.storage().instance().set(&STORAGE_VERSION_KEY, &1u32);
    current = 1;
}
```

---

### Comment Policy Review Checklist

Use this checklist when reviewing any PR that touches Rust source files:

**Invariants**

- [ ] Every storage key constant that has a uniqueness or ordering constraint carries an `// INVARIANT:` comment.
- [ ] Invariants that are already enforced by types or tests are NOT duplicated in comments.

**Public API Notes**

- [ ] Every new `pub fn` has a `///` doc comment with a one-sentence summary.
- [ ] Every `Result`-returning `pub fn` documents its error conditions under `# Errors`.
- [ ] Every new `#[contracttype]` field has a `///` doc comment.
- [ ] Issue references (`#N` or `SC-NNN`) are present for items introduced in tracked issues.

**Implementation Detail Notes**

- [ ] Non-obvious algorithms carry an inline `//` comment explaining the choice.
- [ ] Soroban platform workarounds are explained (e.g., packed `u128` counters).
- [ ] No commented-out code is present without a `// TODO(#issue):` marker.

**Event Compatibility Decisions**

- [ ] Every event constant with a non-empty payload has a compatibility decision comment.
- [ ] Empty-payload events have a short note confirming the intentional absence of payload data.

**Migration Arms**

- [ ] Every migration arm in `migrate()` has a version-transition header comment (`// vN → vN+1`).
- [ ] Each arm describes the transformation applied and any new storage keys it introduces.

---

### What to Express in Tests Instead of Comments

The following concerns belong in test assertions, not in comments:

| Concern | Use instead |
|---|---|
| Numeric boundary values | `#[test]` with explicit boundary assertions |
| Error code semantics | A test that triggers the error and checks the code |
| Idempotency properties | A test that calls the function twice and checks no side effects |
| Symbol value correctness | A distinctness test (see `event_schema.rs`) |
| Config validation ranges | Parameterised tests over boundary inputs |

If a property is expressed in a test and in a comment, the comment is likely to drift. Prefer the test.

---

### Enforcement

This policy is enforced through the PR review checklist above. Reviewers must verify compliance with each category before approving. Automated enforcement via `cargo clippy` lints (`missing_docs`, `clippy::missing_errors_doc`) is enabled for public items; violations block CI.

For questions or proposed policy amendments, open an issue referencing this document.
