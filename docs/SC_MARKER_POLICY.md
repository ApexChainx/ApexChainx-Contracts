# SC- Marker Policy

> **Audience:** Contributors and maintainers of `apexchainx-contracts`.
> This document explains what SC- markers are, how to add them, and when to retire them.

---

## What Are SC- Markers?

SC- markers are short inline references that tie a piece of code to a design decision
or tracked requirement. Two families are in use:

| Family | Pattern | Meaning |
|--------|---------|---------|
| Issue reference | `(SC-NNN)` or `// SC-NNN` | Links to a GitHub issue or PR in this repo |
| Wave-5 sub-issue | `(SC-W5-NNN)` or `// SC-W5-NNN` | Links to a Wave-5 scoped sub-issue |

They appear in doc comments, inline comments, and test module headers — for example:

```rust
/// Hard upper bound on retained history entries. (SC-062)
pub(crate) const MAX_HISTORY_SIZE: u32 = 1000;

//! SC-W5-041 – Canonical event schema for SLA calculation outputs.
```

Markers are **not** a substitute for a real doc comment. They supplement it.

---

## Adding a Marker

1. **Open or identify the issue** the code change tracks.
2. **Pick the right family** — use `SC-W5-NNN` only for Wave-5 sub-issues; use `SC-NNN` for everything else.
3. **Place it inline** at the most specific site:
   - Leading the module doc comment for a whole-module concern (`//! SC-W5-077 – …`)
   - In the item doc comment for a const, type, or function (`/// … (SC-062)`)
   - In an inline comment for a logic block (`// SC-013: use configurable retention limit`)
4. **One marker per concern.** If the same line is relevant to two issues, pick the primary one.
5. **Include a short label** after the dash when it aids skimmability:
   ```rust
   //! SC-W5-079 – Shared event correlation ids for cross-contract tracing.
   ```

---

## Updating a Marker

If an issue is superseded, renumbered, or split:

- Update the marker at the call site.
- Leave a one-line note in the PR description: `SC-042 superseded by SC-W5-042`.
- Do not remove the old marker silently — reviewers need to be able to trace the history.

---

## Retiring a Marker

A marker can be removed when:

- The referenced issue is **closed and the design is considered stable** (no further changes expected).
- The code it annotates is **deleted**.

When retiring:

1. Remove the marker in the same commit that closes or supersedes the issue.
2. Note the retirement in the PR description: `Removed SC-013 marker — feature stable, issue closed`.

Do **not** bulk-remove markers in a cleanup sweep without issue context — that erases design history.

---

## Review Discipline

| When | What to check |
|------|---------------|
| PR adds a new constant, type, or module | Does it have a marker if it was tracked by an issue? |
| PR closes an issue | Are the corresponding markers retired or stable? |
| PR modifies code with an existing marker | Is the marker still accurate, or does it need updating? |

Reviewers are not expected to audit every marker on every PR. The check above is
a lightweight pass to catch obvious drift.

---

## Examples

```rust
// Whole-module header (SC-W5 sub-issue)
//! SC-W5-077 – Cross-contract call safety model with failure rollback semantics.

// Constant with trailing reference
/// Hard upper bound on retained history entries. (SC-062)
pub(crate) const MAX_HISTORY_SIZE: u32 = 1000;

// Inline logic comment
// SC-013: use configurable retention limit (falls back to MAX_HISTORY_SIZE)
let retention_limit: u32 = env.storage()...

// Error variant
/// Duplicate `outage_id` with conflicting inputs detected. (SC-W5-046)
DuplicateOutageInput = 13,
```

The canonical source for existing markers is
[`apexchainx_calculator/src/lib.rs`](../apexchainx_calculator/src/lib.rs) and
the specialized test modules under `apexchainx_calculator/src/`.
