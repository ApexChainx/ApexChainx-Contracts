# SC-104 – Security Review Template for New Contract Modules

> **Purpose:** A standardised security review template to be completed before
> any new contract module (a new crate or a new top-level module within an
> existing crate) is merged into this repository.
>
> **Audience:** The developer proposing the new module AND the security
> reviewer assigned to the PR.
>
> **When to use:** Every PR that introduces a new `pub mod` file or a new
> crate under the workspace must complete this template before merge.

## Table of Contents

- [Module overview](#module-overview)
- [Threat model](#threat-model)
- [Access control](#access-control)
- [Storage & state](#storage--state)
- [Event emission](#event-emission)
- [Input validation](#input-validation)
- [Integer safety](#integer-safety)
- [Pause & emergency](#pause--emergency)
- [Versioning & migration](#versioning--migration)
- [Cross-contract calls](#cross-contract-calls)
- [Test coverage](#test-coverage)
- [Reviewer sign-off](#reviewer-sign-off)

---

## Module overview

| Field | Value |
|-------|-------|
| **Module name** | `_________` |
| **Crate** | `_________` |
| **PR number** | `#___` |
| **Developer** | `@_________` |
| **Purpose** | (One paragraph describing what this module does and why it exists.) |

---

## Threat model

Describe the threat model for this module:

1. **What are the assets?** (e.g., token balances, config values,
   admin authority, historical data)

2. **Who are the actors?** (e.g., admin, operator, any caller,
   cross-contract caller)

3. **What are the trust boundaries?** (e.g., admin is trusted,
   operator is semi-trusted, any caller is untrusted)

4. **What is the worst-case outcome** if this module has a bug?
   (e.g., funds lost, config corrupted, history permanently pruned,
   contract bricked)

---

## Access control

- [ ] **Every state-changing function checks authorisation.**
  - Admin-gated: `require_admin()`
  - Operator-gated: `require_operator()`
  - Caller-gated: `caller.require_auth()`
- [ ] **No function silently skips auth.** Every code path that writes
  to storage must have `require_auth()` or a role check.
- [ ] **Auth checks happen before any state mutation.** If a function
  validates input, reads state, and then checks auth — the auth check
  must come first.
- [ ] **No role escalation path exists.** An operator cannot become
  admin through any sequence of calls.
- [ ] **`renounce_admin` consequences are documented.** If renounce
  affects this module, explain how.

---

## Storage & state

- [ ] **Every new storage key has a unique `Symbol` name.** No two
  keys share the same name (verify with the `test_storage_key_distinctness`
  test in `lib.rs`).
- [ ] **Storage keys follow the `<domain>_<sub>` convention**
  (see [CODING_STYLE.md](../CODING_STYLE.md)).
- [ ] **Storage keys are ≤ 9 characters** (Soroban `Symbol` limit for
  `symbol_short!`).
- [ ] **No storage key collision with existing keys.** List all existing
  storage keys this module reads or writes:

  | Key | Read | Written | Purpose |
  |-----|------|---------|---------|
  | `_________` | ✅ / ❌ | ✅ / ❌ | |
  | `_________` | ✅ / ❌ | ✅ / ❌ | |

- [ ] **Default values are deterministic.** If a key may be absent,
  the `unwrap_or` / `unwrap_or_else` path produces a deterministic
  default.
- [ ] **Storage migration path is defined.** If this module requires a
  new storage version, the `migrate()` function is updated with a
  corresponding arm.

---

## Event emission

If this module emits events:

- [ ] **Every event follows the 3-topic layout:**
  `(EVENT_*, EVENT_VERSION, context)`.
- [ ] **Event name constants are added to `event_schema.rs`.**
- [ ] **Event payload schemas are documented** in both `event_schema.rs`
  and the `EVENT_*` comment block in `lib.rs`.
- [ ] **No event reuses an existing name.** Verify with
  `test_event_names_are_distinct`.
- [ ] **Backward compatibility:** new events are additive — no existing
  event payload is modified in a breaking way.

List all new events:

| Event name | Topic[2] context | Payload fields | Emission site |
|------------|------------------|----------------|---------------|
| `_________` | | | |

---

## Input validation

- [ ] **All function parameters are validated before use.** At minimum:
  - Numeric ranges are checked (no zero denominators, no negative
    values where positive is required).
  - Symbol/string lengths are bounded.
  - Addresses are non-null (Soroban enforces this at the host layer,
    but document the assumption).
- [ ] **Re-entrancy is considered.** Soroban does not have re-entrancy
  in the EVM sense, but cross-contract calls can create call cycles.
  If this module calls another contract, does it assume the callee
  won't call back into this contract?
- [ ] **Duplicate input detection.** If the module generates or accepts
  IDs, is there protection against duplicate submissions?
- [ ] **Error messages are actionable.** Every `Err` variant returned
  maps to a specific, documented failure mode (see
  `get_failure_schema()`).

---

## Integer safety

- [ ] **All arithmetic uses safe operations.** Prefer
  `checked_add`/`checked_mul`/`saturating_add` over bare `+`/`*`.
- [ ] **No truncation bugs.** Casts between integer types (`u64` →
  `u32`, `i128` → `i64`) use explicit `.try_into()` or are guarded
  by a range check.
- [ ] **Overflow/underflow cannot produce valid-but-wrong results.**
  If an overflow saturates (e.g., stats counter at `u64::MAX`), an
  event (`stats_sat`) is emitted to signal the cap.
- [ ] **Division by zero is impossible.** Every division/remainder
  operation is guarded by a non-zero check.

---

## Pause & emergency

- [ ] **State-changing functions check pause status.** All write
  functions call `require_not_paused()` (or equivalent) at the top.
- [ ] **Read-only functions work while paused.** Views and queries
  must not be blocked by the pause flag.
- [ ] **Pause cannot be bypassed.** There is no code path that writes
  state without first checking the pause flag.
- [ ] **Emergency stop is documented.** If this module has an
  emergency-only function (e.g., force-unlock, emergency-withdraw),
  its behaviour and auth requirements are clearly documented.

---

## Versioning & migration

- [ ] **`STORAGE_VERSION` is bumped** if this module adds persistent
  storage keys or changes the schema of existing keys.
- [ ] **`RESULT_SCHEMA_VERSION` is bumped** if any public return type
  changes shape.
- [ ] **`migrate()` includes a transition arm** for the new version.
- [ ] **Migration is tested** — there is a test that deploys the old
  version, calls migrate, and verifies the new module's state is
  correctly initialised.
- [ ] **`get_contract_metadata().features` lists the new module** if
  it adds a user-visible capability.

---

## Cross-contract calls

If this module calls another Soroban contract:

- [ ] **The callee contract ID is configurable or well-known.**
- [ ] **The call result is checked** — `Ok`/`Err` is handled, not
  ignored.
- [ ] **A timeout or failure mode is defined.** What happens if the
  callee panics or is unavailable?
- [ ] **No unbounded recursion or call loops.**

N/A if this module makes no cross-contract calls: ☐

---

## Test coverage

- [ ] **Happy-path tests exist** for every public function.
- [ ] **Error-path tests exist** for every `Err` variant this module
  can return.
- [ ] **Boundary tests exist** for numeric ranges, empty collections,
  and edge-case inputs.
- [ ] **Pause tests exist** — every write function is tested with the
  contract paused.
- [ ] **Auth tests exist** — unauthorised callers are rejected for
  every gated function.
- [ ] **Fuzz tests exist** (if the module accepts user-controlled
  numeric input). See `apexchainx_calculator/fuzz/`.
- [ ] **Idempotency tests exist** — calling the same function twice
  with identical inputs produces consistent results (no unintended
  state mutation on repeat calls).

---

## Reviewer sign-off

| Check | Reviewer | Date |
|-------|----------|------|
| Threat model reviewed | | |
| Access control verified | | |
| Storage keys audited | | |
| Event schema confirmed | | |
| Input validation checked | | |
| Integer safety verified | | |
| Pause compliance confirmed | | |
| Migration path validated | | |
| Tests reviewed for coverage | | |
| **Overall approval** | | |

---

## References

- [SECURITY.md](../SECURITY.md) — vulnerability reporting policy.
- [CONTRIBUTING.md § SC-098](../CONTRIBUTING.md#sc-098-security-review-checklist-for-privileged-changes) —
  PR-level security review for privileged changes.
- [CONTRIBUTING.md § SC-099](../CONTRIBUTING.md#sc-099-event-topic--payload-schema-contributor-safety-checklist) —
  event compatibility checklist.
- [PROJECT_CONTEXT.md § SC-100](PROJECT_CONTEXT.md#sc-100-future-contract-roadmap) —
  planned future crates and their dependencies.
- [PUBLIC_FUNCTION_DOC_POLICY.md](PUBLIC_FUNCTION_DOC_POLICY.md) —
  doc comment policy for all public items.
