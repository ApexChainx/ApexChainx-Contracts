# Issue #783: Document Pausable with NatSpec Comments

**Status:** Reviewed — Rust doc comments (equivalent to NatSpec) are already present;
this document records the inventory and fills any gaps.

**Priority:** P0 - Critical  
**Area:** Pausable (`apexchainx_calculator`)

---

## 1. NatSpec vs. Rust doc comments

[NatSpec](https://docs.soliditylang.org/en/latest/natspec-format.html) is the
Solidity documentation standard using `///` and `@param`, `@return`, `@notice`,
`@dev`, and `@custom` tags.

The Soroban/Rust equivalent is idiomatic **Rust doc comments** (`///` line docs or
`/** */` block docs), which are:

- Extracted by `rustdoc` into HTML documentation.
- Enforced by the project's `PUBLIC_FUNCTION_DOC_POLICY.md` (SC-102).
- Structurally identical to NatSpec `@notice` + `@dev` comments in their
  purpose and placement — one `///` block per public item.

This document provides the full NatSpec-style documentation inventory for every
Pausable-related public item, for auditor and integrator reference.

---

## 2. Pausable public API — NatSpec-style documentation

### `PauseInfo` struct

```rust
/// Pause metadata stored when the contract is paused.
///
/// @dev Written to instance storage at `PAUSE_INFO_KEY` by `pause()`.
///      Removed from instance storage by `unpause()`.
///      Exposed read-only via `get_pause_info()`.
///
/// @field reason    Human-readable reason string supplied by the admin caller.
///                  Bounded by `MAX_REASON_LEN` bytes; `pause()` rejects longer values.
/// @field paused_at Ledger timestamp (seconds since Unix epoch) recorded at pause time.
/// @field paused_by Address of the admin that invoked `pause()`.
pub struct PauseInfo {
    pub reason: String,
    pub paused_at: u64,
    pub paused_by: Address,
}
```

### `pause()`

```rust
/// Pause the contract with a human-readable reason and ledger timestamp.
///
/// @notice Once paused, `calculate_sla` is blocked until `unpause()` is called.
///         All state-changing operations guarded by `require_not_paused` will
///         return `SLAError::ContractPaused`.
///
/// @dev    Validates `reason.len() <= MAX_REASON_LEN` before any storage access
///         or auth check (fail-fast). Writes `PAUSED_KEY = true` and
///         `PAUSE_INFO_KEY = PauseInfo { reason, paused_at, paused_by }` to
///         instance storage. Emits an `EVENT_PAUSED` event.
///
/// @param  env    Soroban execution environment.
/// @param  caller Address that must satisfy `require_admin`; auth checked on-chain.
/// @param  reason Operator-supplied pause reason (max `MAX_REASON_LEN` bytes).
///
/// @return Ok(()) on success.
///         Err(SLAError::InvalidInput) if `reason` exceeds `MAX_REASON_LEN`.
///         Err(SLAError::VersionMismatch) if storage schema is not at expected version.
///         Err(SLAError::Unauthorized) if `caller` is not the admin.
pub fn pause(env: Env, caller: Address, reason: String) -> Result<(), SLAError>
```

### `unpause()`

```rust
/// Unpause the contract, clearing all pause metadata.
///
/// @notice Restores `calculate_sla` and all operations gated by `require_not_paused`.
///
/// @dev    Sets `PAUSED_KEY = false` and removes `PAUSE_INFO_KEY` from instance
///         storage. Emits an `EVENT_UNPAUSED` event. Safe to call when already
///         unpaused (idempotent, no error).
///
/// @param  env    Soroban execution environment.
/// @param  caller Address that must satisfy `require_admin`; auth checked on-chain.
///
/// @return Ok(()) on success.
///         Err(SLAError::VersionMismatch) if storage schema is not at expected version.
///         Err(SLAError::Unauthorized) if `caller` is not the admin.
pub fn unpause(env: Env, caller: Address) -> Result<(), SLAError>
```

### `is_paused()`

```rust
/// Returns `true` when the contract is currently paused.
///
/// @notice Read-only; does not require authentication.
///
/// @dev    Reads `PAUSED_KEY` from instance storage. Returns `false` when the key
///         is absent (i.e., the contract has never been paused).
///
/// @param  env Soroban execution environment.
///
/// @return Ok(true) if the contract is paused, Ok(false) if it is not.
///         Err(SLAError::VersionMismatch) if storage schema is not at expected version.
pub fn is_paused(env: Env) -> Result<bool, SLAError>
```

### `get_pause_info()`

```rust
/// Returns pause metadata if the contract is currently paused, or `None` if unpaused.
///
/// @notice Read-only; does not require authentication.
///         Returns `None` when the contract is not paused OR when it has never
///         been paused (the key is absent in both cases).
///
/// @dev    Reads `PAUSE_INFO_KEY` from instance storage. The value is `Some(PauseInfo)`
///         only while `PAUSED_KEY == true`; `unpause()` removes the key unconditionally.
///
/// @param  env Soroban execution environment.
///
/// @return Ok(Some(PauseInfo)) while paused (includes reason, timestamp, caller address).
///         Ok(None) when not paused.
///         Err(SLAError::VersionMismatch) if storage schema is not at expected version.
pub fn get_pause_info(env: Env) -> Result<Option<PauseInfo>, SLAError>
```

### `require_not_paused()` (internal guard)

```rust
/// Guard function called at the top of `calculate_sla` to block execution while paused.
///
/// @dev Internal only (`pub(crate)` equivalent — private to `calculation.rs`).
///      Reads `PAUSED_KEY`; returns `Err(SLAError::ContractPaused)` when `true`.
///      Positioned before any state mutation to satisfy Checks-Effects-Interactions.
///
/// @param  env Reference to the Soroban execution environment.
///
/// @return Ok(()) if the contract is not paused.
///         Err(SLAError::ContractPaused) if the contract is paused.
fn require_not_paused(env: &Env) -> Result<(), SLAError>
```

---

## 3. Storage keys related to Pausable

| Key constant | Value | Type stored | Description |
|---|---|---|---|
| `PAUSED_KEY` | `symbol_short!("paused")` | `bool` | Master pause flag; `true` = paused |
| `PAUSE_INFO_KEY` | `symbol_short!("pinfo")` | `PauseInfo` | Metadata: reason, timestamp, admin address |

---

## 4. Events emitted by Pausable

| Constant | Symbol value | Payload | Emitted by |
|---|---|---|---|
| `EVENT_PAUSED` | `"paused"` | `(true,)` | `pause()` |
| `EVENT_UNPAUSED` | `"unpause"` | `(false,)` | `unpause()` |

Event topics include `(event_name, event_version, caller_address)`.

---

## 5. Error codes relevant to Pausable

| `SLAError` variant | Code | Meaning in Pausable context |
|---|---|---|
| `ContractPaused` | 6 | Returned by `require_not_paused` when `PAUSED_KEY == true` |
| `Unauthorized` | (auth) | Returned when a non-admin calls `pause` or `unpause` |
| `InvalidInput` | (input) | Returned when `reason.len() > MAX_REASON_LEN` |
| `VersionMismatch` | (version) | Returned when storage schema version does not match |

---

## 6. Related documentation

- [`docs/PUBLIC_FUNCTION_DOC_POLICY.md`](PUBLIC_FUNCTION_DOC_POLICY.md) — doc comment enforcement policy (SC-102)
- [`docs/REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md`](REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md)
- [`docs/PAUSE_TRANSITION_POSTURE.md`](PAUSE_TRANSITION_POSTURE.md)
- `apexchainx_calculator/src/lib.rs` — implementation source
- `apexchainx_calculator/src/calculation.rs` — `require_not_paused` guard

Fixes #783
