# SC-102 – Public Function Documentation Policy

> **Status:** Enforced — all PRs that add or modify a `pub` item MUST include
> a doc comment and, where applicable, a schema note.

## Table of Contents

- [Scope](#scope)
- [Policy](#policy)
- [Doc Comment Template](#doc-comment-template)
- [Schema Notes](#schema-notes)
- [Enforcement](#enforcement)
- [Review Checklist](#review-checklist)
- [Rationale](#rationale)

---

## Scope

This policy applies to **every public item** in the `apexchainx_calculator`
crate and any future crates added to this repository:

| Item Kind | Requirement | Example |
|-----------|-------------|---------|
| `pub fn` (free function) | Doc comment required | `/// Returns the current admin address.` |
| `pub fn` (method on `#[contractimpl]`) | Doc comment + schema note if it exposes a new data shape | `/// Calculate SLA result…` |
| `pub struct` / `pub enum` | Doc comment for the type + every public field | `/// Configuration parameters for a single severity level.` |
| `pub const` | Doc comment describing purpose and stability guarantee | `/// Admin address — set during initialize…` |
| `pub type` | Doc comment with use-case | `/// Deterministic correlation ID for cross-contract workflows.` |
| `pub mod` | Module-level doc comment (`//!`) in the module file | `//! SC-W5-041 – Canonical event schema…` |
| `pub trait` | Doc comment for the trait + every method | `/// Trait for…` |

**Private items** (`pub(crate)`, `fn` without `pub`) are encouraged but not
required to have doc comments. Reviewers should still ask for them when the
logic is non-obvious.

---

## Policy

> **Every new `pub` item merged into `main` MUST carry a doc comment that
> explains what the item does, its inputs/outputs, and any stability
> guarantees it makes to consumers.**

1. **Doc comments are mandatory.** A PR that adds a `pub` item without a
   doc comment must be rejected in review.

2. **Stale doc comments are bugs.** If a PR changes a function's signature,
   return type, or behaviour, its doc comment must be updated in the same
   commit. A doc comment that describes old behaviour is treated as a
   documentation bug.

3. **Doc comments are tested.** Rustdoc tests (` ``` ` blocks inside doc
   comments) run in CI. They must pass — use ` ```ignore` or ` ```text`
   for illustrative examples that shouldn't compile.

4. **No blanket `#[allow(missing_docs)]`.** Suppressing the lint on a
   per-item basis (`#[allow(missing_docs)]`) is allowed only with an
   explicit reviewer-approved comment explaining why. Blanket allows on
   modules are banned.

---

## Doc Comment Template

Use this template for contract methods exposed through `#[contractimpl]`:

```rust
/// Brief one-line summary of what this function does.
///
/// # Arguments
/// * `arg_name` - Description of the argument, including units and valid
///   range.
///
/// # Returns
/// Description of the return value and what `Ok` / `Err` variants mean.
///
/// # Errors
/// * `SLAError::VariantName` - When this error is returned.
///
/// # Events
/// * `event_name` - Emitted when [condition]. Payload: `(field: Type, ...)`.
///
/// # Auth
/// * Caller must be the contract admin.
///
/// # Pause behaviour
/// * Blocked when the contract is paused (returns `ContractPaused`).
///
/// # Schema stability
/// * Return type is covered by `get_result_schema()`.
/// * This function's output shape is stable across minor versions.
pub fn example_function(
    env: Env,
    caller: Address,
    arg_name: Symbol,
) -> Result<ReturnType, SLAError> {
    // …
}
```

For simple types and constants, a one-line doc comment is sufficient:

```rust
/// Maximum length (in bytes) for the pause reason string. (#68)
pub const MAX_REASON_LEN: usize = 256;

/// Configuration parameters for a single severity level.
/// Each severity (critical, high, medium, low) has its own SLAConfig.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAConfig {
    /// Maximum allowed repair time in minutes before SLA is violated.
    pub threshold_minutes: u32,
    /// Penalty amount charged per minute of overtime (positive integer).
    pub penalty_per_minute: i128,
    /// Base reward amount for meeting SLA targets (positive integer).
    pub reward_base: i128,
}
```

---

## Schema Notes

When a public function **returns or emits** a data shape that a backend
consumer (the `apexchainx-be` bridge) depends on, the doc comment must
include a **schema note**:

```rust
/// # Schema stability
/// * Return type is covered by `get_result_schema()`.
/// * The `deprecated_symbols` entry documents any pending removals.
```

Schema notes serve as audit trail for the backend team — they are the
contract-level signal that a return shape change affects the bridge.

**When to add a schema note:**

| Situation | Schema note required? |
|-----------|----------------------|
| Function returns a `#[contracttype]` struct | ✅ Yes |
| Function emits an event consumed by the backend | ✅ Yes |
| Function returns a primitive (`u32`, `Symbol`, `Address`) | ❌ No |
| Function returns a Soroban container (`Vec`, `Map`) | ✅ Yes |
| Function is a pure utility (no backend dependency) | ❌ No |

---

## Enforcement

This policy is enforced at multiple levels:

### 1. Compile-time (`#![warn(missing_docs)]`)

The crate root (`apexchainx_calculator/src/lib.rs`) carries
`#![warn(missing_docs)]`. This causes `cargo check` / `cargo build` to
emit warnings for any `pub` item in the crate that lacks a doc comment.

> **Note:** Soroban SDK proc macros (`#[contracttype]`, `#[contract]`,
> `#[contracterror]`) generate public associated functions and statics
> that cannot carry doc comments. To keep the warning output actionable,
> `#[allow(missing_docs)]` is applied to `#[contracttype]` structs and
> `#[contracterror]` enums so only **human-written** public items trigger
> warnings. A `warn`-level lint is used instead of `deny` to prevent
> macro-generated items from blocking compilation.

### 2. CI (`cargo doc`)

The `ci.yml` workflow runs `cargo doc --no-deps --document-private-items`
and fails if `RUSTDOCFLAGS="-D warnings"` produces any warnings. This
catches broken intra-doc links (`[`NotInitialized`]` referencing a
nonexistent item). It also runs `RUSTFLAGS="-D warnings" cargo check` to
ensure `missing_docs` warnings fail CI.

### 3. Code review

PR reviewers MUST check the "Public Function Doc Policy" checklist
(see below) before approving. The checklist is embedded in the PR
template.

---

## Review Checklist

Before approving a PR that adds or modifies a `pub` item, confirm:

- [ ] **Every new `pub` item has a doc comment.**
- [ ] **Modified `pub` items have updated doc comments** reflecting the
  new signature, behaviour, or return type.
- [ ] **Schema notes are present** on any function whose output shape
  is consumed by the backend bridge.
- [ ] **Storage-key references in doc comments are accurate** — storage
  key names match the `const` definition.
- [ ] **Event payload schemas in doc comments match the actual emission
  site** — field count, types, and order are correct.
- [ ] **No blanket `#[allow(missing_docs)]`** on a module unless
  accompanied by a reviewer-approved comment.
- [ ] **Rustdoc examples compile** (or use ` ```ignore` if
  intentionally non-compiling).
- [ ] **CI passes:** `cargo doc --no-deps` and `cargo check` produce
  no `missing_docs` warnings.

---

## Rationale

1. **Backend bridge depends on doc comments.** The `apexchainx-be` team
   uses `cargo doc` output as their primary API reference. Missing doc
   comments become missing API documentation, which leads to integration
   bugs.

2. **Storage keys are a shared namespace.** Doc comments trace each
   storage key to the feature that introduced it, making collision
   detection possible during review.

3. **Event schemas are a contract.** The `EVENT_*` constant block and
   the `event_schema.rs` catalogue are the source of truth for backend
   indexers. Doc comments are the first line of defence against silent
   drift between the emission site and the documented schema.

4. **Audit trail.** For security reviewers and auditors, doc comments
   on public items serve as the high-level map of the contract's
   capabilities. A missing doc comment is a missing entry in that map.

---

## References

- [CODING_STYLE.md](../CODING_STYLE.md)
- [CONTRIBUTING.md § Smart Contracts (Rust/Soroban)](../CONTRIBUTING.md#smart-contracts-rustsoroban)
- [EVENT_COMPATIBILITY_POLICY.md](EVENT_COMPATIBILITY_POLICY.md)
- [AUDIT_TRAIL.md](AUDIT_TRAIL.md)
