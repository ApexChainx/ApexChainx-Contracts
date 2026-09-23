# Governance Reference

> **Issue:** [#590](https://github.com/ApexChainx/ApexChainx-Contracts/issues/590)
> **Status:** Active
> **Applies to:** `governance.rs` — admin/operator role transfer, renounce, and proposal lifecycle

This is the authoritative reference for the two-step role-handoff governance
flow. It documents every state transition, every emitted event, and the
invalidation rules consumers must rely on. Source of truth for the storage
keys and event names: [`governance.rs`](../apexchainx_calculator/src/governance.rs).

---

## 1. Roles

| Role | Storage key | Set by |
| --- | --- | --- |
| Admin | `ADMIN_KEY` | `initialize`, `accept_admin` |
| Operator | `OPERATOR_KEY` | `accept_operator`, `set_operator` |
| Pending admin | `PENDING_ADMIN_KEY` + `PENDING_ADMIN_TS_KEY` | `propose_admin` |
| Pending operator | `PENDING_OP_KEY` + `PENDING_OP_TS_KEY` | `propose_operator` |

## 2. Proposal lifecycle

A proposal moves through at most four states:

```
            propose_*                 (re-proposal)                (within window)
  none ───────────────▶ pending ───────────────────────▶ pending (new candidate)
                        │      ◀── supersede emits *_sup ──┘
                        │
        ┌───────────────┼───────────────────┐
        ▼               ▼                   ▼
 accept_* (*_acc)   cancel_* (*_can)   lapse (first observed *_xp)
 (role set)          (keys cleared)      (keys cleared, reject accept)
```

- **Proposal window:** 90 ledger-days (see `PROPOSAL_EXPIRY_WINDOW`).
- **Expiry is lazy and observable:** a lapsed proposal is only noticed when an
  accept attempt arrives. The first such attempt emits the typed expiry event
  (`adm_xp` / `op_xp`) with the stale candidate as payload, clears the pending
  keys, and rejects the accept with `ProposalExpired`. Every later attempt is
  rejected with `NoPendingTransfer` and emits nothing, so the expiry transition
  is emitted **at most once** and is idempotent for indexers. (#589)

## 3. Renounce (`renounce_admin`) — invalidation rules

`renounce_admin` permanently removes the admin role. It is **irreversible**.

Beyond removing the admin, renounce clears **every** pending governance
proposal:

- The pending **admin** proposal is cleared (a renouncing admin must not leave
  a stale handoff pointing at a role that no longer exists).
- If a pending **operator** proposal exists, it is cleared **and** the
  cancellation event `op_can` is emitted, so an operator who had initiated the
  handoff can reconstruct *why* their accept path no longer works.

Because the pending operator keys are removed by renounce, a proposed operator
calling `accept_operator` afterwards receives `NoPendingTransfer` — never a
silent role grant. (#590)

### Emitted events (renounce)

| Event | When | Payload |
| --- | --- | --- |
| `op_can` | only if a pending operator proposal existed | `()` |
| `adm_ren` | always | `()` |

> Consumers: a governance cycle `op_prop` … `op_can` followed by `adm_ren`
> means the admin renounced and thereby invalidated the pending operator
> handoff. A bare `adm_ren` without `op_can` means no operator handoff was
> pending at renounce time.

## 4. Event table

| Event | Emitter | Auth | Payload |
| --- | --- | --- | --- |
| `adm_prop` | `propose_admin` | admin | `(new_admin: Address,)` |
| `adm_acc` | `accept_admin` | new admin | `()` |
| `adm_can` | `cancel_admin_proposal` | admin | `()` |
| `adm_sup` | `propose_admin` (re-proposal) | admin | `(superseded_admin, new_admin)` |
| `adm_xp` | `accept_admin` (lapsed) | new admin | `(expired_admin: Address,)` |
| `adm_ren` | `renounce_admin` | admin | `()` |
| `op_prop` | `propose_operator` | admin | `(new_operator: Address,)` |
| `op_acc` | `accept_operator` | new operator | `()` |
| `op_can` | `cancel_operator_proposal` / `renounce_admin` / `set_operator` | admin | `()` |
| `op_sup` | `propose_operator` (re-proposal) | admin | `(superseded_operator, new_operator)` |
| `op_xp` | `accept_operator` (lapsed) | new operator | `(expired_operator: Address,)` |
| `op_set` | `set_operator` (single-step) | admin | `(new_operator: Address,)` |

## 5. Decision rules for consumers

1. An `op_prop` cycle terminates with exactly one of `op_acc`, `op_can`, or `op_xp`.
2. `op_can` has three possible causes — explicit cancellation, admin renounce
   (preceded/followed by `adm_ren`), or single-step `set_operator`. Disambiguate
   with the adjacent events.
3. `adm_ren` is terminal and invalidates all pending proposals.
4. Expiry events (`adm_xp`/`op_xp`) are emitted **at most once** per proposal;
   a subsequent accept attempt emits nothing.