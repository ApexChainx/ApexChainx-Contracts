# Issue #772: Review Pausable for Reentrancy Vulnerabilities

**Status:** Reviewed — no reentrancy vulnerabilities found in the Pausable module.

**Priority:** P1 - High  
**Area:** Pausable (`apexchainx_calculator`)

---

## 1. Scope

This review covers the Pausable module's reentrancy posture:

- `pause()` — `apexchainx_calculator/src/lib.rs`
- `unpause()` — `apexchainx_calculator/src/lib.rs`
- `is_paused()` / `get_pause_info()` — `apexchainx_calculator/src/lib.rs`
- `require_not_paused()` — `apexchainx_calculator/src/calculation.rs`

---

## 2. Reentrancy threat model on Soroban

On Soroban (Stellar's smart contract platform), reentrancy attack vectors differ
from Ethereum:

- **No fallback / receive hooks**: Soroban contracts cannot receive native tokens
  through a fallback that auto-executes arbitrary code. There is no equivalent to
  Solidity's `receive()` or `fallback()`.
- **Deterministic host execution**: The Soroban host executes each contract
  invocation atomically within a single transaction. Sub-invocations (`invoke_contract`)
  happen synchronously and do not yield back to the caller mid-execution.
- **No read-modify-write races**: All storage operations against the instance
  key-value store are serialised within the host; there is no concurrent access.

The primary reentrancy risk specific to Pausable is:
> Could an attacker call `pause` or `unpause` from within a reentrant call chain
> in a way that bypasses the `require_not_paused` guard in `calculate_sla`?

---

## 3. Analysis

### 3.1 `pause()` — Checks-Effects-Interactions compliance

```
pause(env, caller, reason):
  1. CHECKS:  reason.len() > MAX_REASON_LEN  → return Err(InvalidInput)   [early exit]
  2. CHECKS:  check_version(&env)?                                          [version gate]
  3. CHECKS:  require_admin(&env, &caller)?                                 [auth gate]
  4. EFFECTS: storage.set(PAUSED_KEY, true)
  5. EFFECTS: storage.set(PAUSE_INFO_KEY, PauseInfo { ... })
  6. INTERACTIONS: env.events().publish(EVENT_PAUSED, ...)                  [emit event]
```

All state writes (EFFECTS) occur **before** any external interaction (event
emission). No outbound `invoke_contract` call is made. The event publishing step
calls into the Soroban host event ledger — a write-only append operation with no
callback path back into the contract.

**Finding:** `pause()` is fully Checks-Effects-Interactions compliant and is not
susceptible to reentrancy.

### 3.2 `unpause()` — Checks-Effects-Interactions compliance

```
unpause(env, caller):
  1. CHECKS:  check_version(&env)?
  2. CHECKS:  require_admin(&env, &caller)?
  3. EFFECTS: storage.set(PAUSED_KEY, false)
  4. EFFECTS: storage.remove(PAUSE_INFO_KEY)
  5. INTERACTIONS: env.events().publish(EVENT_UNPAUSED, ...)
```

Same pattern as `pause()`. All state mutations precede event publication.

**Finding:** `unpause()` is Checks-Effects-Interactions compliant and is not
susceptible to reentrancy.

### 3.3 `require_not_paused()` in `calculate_sla`

```rust
fn require_not_paused(env: &Env) -> Result<(), SLAError> {
    let paused: bool = env.storage().instance().get(&PAUSED_KEY).unwrap_or(false);
    if paused {
        return Err(SLAError::ContractPaused);
    }
    Ok(())
}
```

This guard is called at the **top** of `calculate_sla` before any state
mutations or sub-invocations. Because `pause()` writes `PAUSED_KEY = true` before
emitting any event, and there is no code path that calls back into `calculate_sla`
from within `pause()`'s execution, the guard cannot be bypassed via reentrancy.

**Finding:** The `require_not_paused` guard is positioned correctly and cannot be
circumvented through reentrant calls.

### 3.4 Admin-authorization gate

Both `pause()` and `unpause()` call `require_admin(&env, &caller)`, which invokes
`caller.require_auth()` on the Soroban host. This call authenticates the invoker
via the Stellar signature mechanism; it does not invoke any user-controlled
callback. It cannot initiate a reentrant execution path.

**Finding:** No reentrancy risk introduced by the authorization gate.

### 3.5 Double-pause / double-unpause

The current implementation allows pausing an already-paused contract (overwrites
`PAUSE_INFO_KEY`) and unpausing an already-unpaused contract (sets `PAUSED_KEY`
to `false` again, removing an absent key). Neither path creates a reentrancy
vector; both remain admin-gated and state-safe.

**Finding:** Idempotency is safe; no reentrancy concern.

---

## 4. Cross-contract safety

Outbound cross-contract calls in this codebase go through
`cross_contract_safety::safe_invoke_contract`, which:

- Tracks call depth and enforces `MAX_CALL_DEPTH` (= 64).
- Performs compensation rollbacks on failures.
- Is **not** invoked by `pause()`, `unpause()`, `is_paused()`, or `get_pause_info()`.

The Pausable entrypoints make no outbound contract invocations at all, eliminating
the entire class of cross-contract reentrancy from this surface.

---

## 5. Conclusion

| Entrypoint | CEI Compliant | Outbound calls | Reentrancy risk |
|---|---|---|---|
| `pause()` | ✅ Yes | None | **None** |
| `unpause()` | ✅ Yes | None | **None** |
| `is_paused()` | ✅ Read-only | None | **None** |
| `get_pause_info()` | ✅ Read-only | None | **None** |
| `require_not_paused` | ✅ Guard-first | None | **None** |

No reentrancy vulnerabilities were identified in the Pausable module.

---

## 6. Related documentation

- [`docs/REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md`](REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md) — combined ReentrancyGuard + Pausable review
- [`docs/REENTRANCY_PAUSABLE_AND_UPGRADE_AUDIT.md`](REENTRANCY_PAUSABLE_AND_UPGRADE_AUDIT.md) — full audit record
- [`docs/UPGRADE_AND_REENTRANCY_REVIEW.md`](UPGRADE_AND_REENTRANCY_REVIEW.md) — upgrade + reentrancy context
- `apexchainx_calculator/src/cross_contract_safety.rs` — cross-contract call isolation

Fixes #772
