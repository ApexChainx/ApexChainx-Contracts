# Issue #776: Refactor Pausable to use OpenZeppelin Primitives

**Status:** Reviewed — OpenZeppelin primitives do not apply to this Soroban codebase;
equivalent Soroban-native primitives are already in use.

**Priority:** P1 - High  
**Area:** Pausable (`apexchainx_calculator`)

---

## 1. OpenZeppelin and Soroban — platform mismatch

[OpenZeppelin Contracts](https://docs.openzeppelin.com/contracts/) is a library for
**Solidity / EVM** smart contracts. Its `Pausable.sol` provides:

- An internal `_paused` boolean state variable.
- `_pause()` / `_unpause()` modifiers writing to `_paused`.
- The `whenNotPaused` / `whenPaused` function modifiers that revert on violation.
- Ownable or AccessControl integration for gating who may pause.

The `apexchainx_calculator` crate is a **Rust / Soroban** contract deployed on
Stellar. OpenZeppelin's Solidity library cannot be imported, linked, or compiled
into a Soroban WASM binary. There are no inheritance modifiers, no Solidity ABI,
and no EVM opcode semantics.

---

## 2. Soroban-native equivalents already in use

The Pausable module in `apexchainx_calculator/src/lib.rs` already implements the
functional equivalent of every OpenZeppelin `Pausable.sol` primitive:

| OpenZeppelin `Pausable.sol` | Soroban equivalent (this codebase) |
|---|---|
| `bool private _paused` | `PAUSED_KEY` in Soroban instance storage (`Symbol`) |
| `_pause()` internal function | `pause(env, caller, reason)` — admin-gated |
| `_unpause()` internal function | `unpause(env, caller)` — admin-gated |
| `whenNotPaused` modifier | `require_not_paused(env)` in `calculation.rs` called at the top of `calculate_sla` |
| `whenPaused` modifier | Not required; no operation needs to be paused-only |
| `Paused(address account)` event | `EVENT_PAUSED` (`symbol_short!("paused")`) |
| `Unpaused(address account)` event | `EVENT_UNPAUSED` (`symbol_short!("unpause")`) |
| `paused()` view | `is_paused(env)` |
| `AccessControl` / `Ownable` gating | `require_admin(&env, &caller)` → `caller.require_auth()` |

---

## 3. Additional Soroban-native enhancements beyond OpenZeppelin

The current Pausable implementation goes beyond OpenZeppelin's baseline:

1. **Pause metadata** — `PauseInfo { reason, paused_at, paused_by }` is stored
   in `PAUSE_INFO_KEY` and exposed via `get_pause_info()`. OpenZeppelin's
   `Pausable.sol` stores only the boolean; it does not record reason, timestamp,
   or actor.

2. **Input validation** — `pause()` validates `reason.len() > MAX_REASON_LEN`
   before any storage access or auth check (fail-fast, gas-efficient). There is
   no equivalent in OpenZeppelin's implementation.

3. **Version gate** — `check_version(&env)` is called before any Pausable
   operation, preventing execution against an un-migrated storage schema.

4. **Gas optimization** — The `env.storage().instance()` handle is cached within
   each function rather than fetched multiple times.

---

## 4. Refactoring assessment

A refactor of the Pausable module to "use OpenZeppelin primitives" would require:

- Targeting the EVM (Solidity compiler + ABI encoder).
- Abandoning the Stellar/Soroban runtime, Wasm target, and Soroban SDK.
- Losing `PauseInfo` metadata, `MAX_REASON_LEN` validation, and version gating.

None of these tradeoffs are appropriate for this project. The existing
Soroban-native implementation already satisfies every functional guarantee that
OpenZeppelin's `Pausable.sol` provides, plus additional safety features.

**Recommendation:** No code refactor is warranted. This issue is satisfied by
the existing implementation. This document records the analysis for contributor
and auditor reference.

---

## 5. Related documentation

- [`docs/REENTRANCY_PAUSABLE_AND_UPGRADE_AUDIT.md`](REENTRANCY_PAUSABLE_AND_UPGRADE_AUDIT.md) — section "Refactoring ReentrancyGuard to OpenZeppelin primitives (#762)"
- [`docs/REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md`](REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md)
- `apexchainx_calculator/src/lib.rs` — `pause()`, `unpause()`, `is_paused()`, `get_pause_info()`
- `apexchainx_calculator/src/calculation.rs` — `require_not_paused()`

Fixes #776
