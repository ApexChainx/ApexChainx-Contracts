# Operator Rotation Guide

> **Issue:** [#651](https://github.com/ApexChainx/ApexChainx-Contracts/issues/651)

## Two paths to rotate the operator

| Path | Steps | Consent |
|---|---|---|
| **Direct set** | Admin calls `set_operator(new_op)` | New operator does NOT consent; admin unilateral |
| **Two-step handoff** | Admin calls `propose_operator(new_op)`, new op calls `accept_operator()` | New operator explicitly accepts |

## Interaction: direct set while a handoff is pending

If `propose_operator` has been called but `accept_operator` has not yet been
called, and the admin then calls `set_operator`:

1. `set_operator` **clears the pending operator proposal** (keys `PENDING_OP_KEY`
   and `PENDING_OP_TS_KEY` are removed).
2. The proposed operator's `accept_operator` call will return `NoPendingTransfer`.
3. The `op_set` event is emitted (not `op_acc`) — indexers can distinguish the paths.

**Operational implication:** Starting a two-step handoff and then doing a
direct set is a silent cancellation of the handoff from the proposed
operator's perspective. The proposed operator receives no notification other
than `NoPendingTransfer` on their next attempt.

## Event trail per path

| Action | Event emitted | Payload |
|---|---|---|
| `propose_operator` | `op_prop` | `(new_operator,)` |
| `accept_operator` | `op_acc` | `()` |
| `cancel_operator_proposal` | `op_can` | `()` |
| `set_operator` (direct) | `op_set` | `(new_operator,)` |
| Proposal lapse (on first accept attempt) | `op_xp` | `(stale_candidate,)` |

## Best practice

Prefer the two-step handoff for key rotations requiring consent. Reserve
`set_operator` for emergency replacements where the admin cannot wait for
the new operator to accept. Always monitor for `op_set` vs `op_acc` events
to distinguish consented from non-consented rotations.