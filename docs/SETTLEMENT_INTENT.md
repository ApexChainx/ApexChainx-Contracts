# Settlement Intent vs. On-Chain Execution

> **Issue:** [#676](https://github.com/ApexChainx/ApexChainx-Contracts/issues/676)
> **Status:** Active
> **Applies to:** the `set_int` event and every backend that consumes it

## The boundary in one sentence

The `set_int` event is a **request**, not a transfer: `calculate_sla` emits the
computed reward/penalty as a *settlement intent* for off-chain processing, and
**no value moves on chain** as a result of the event.

## What the contract does and does not do

| The contract… | The contract does **not**… |
|---|---|
| Computes the SLA decision (`status`, `amount`, `payment_type`, `rating`) | Hold, escrow, or transfer any token |
| Emits `set_int` (settlement intent) carrying that decision | Authorise, sign, or submit a payment |
| Emits `sla_calc` (the decision record) and appends it to history | Guarantee the intent was ever acted on |
| Provides `correlation_id` so the intent can be joined to downstream work | Execute the downstream work itself |

There is no on-chain escrow/settlement contract in this repository. The
`set_int` event is the point where this contract's responsibility ends; whatever
pays out the amount is a separate system that must decide whether to honour the
intent.

## Why the vocabulary matters

The event name is `set_int` (**set**tlement **int**ent) and the surrounding
code/docs deliberately talk about *intent* rather than *settlement* precisely
because the amount has not been collected or paid when the event fires. Treating
the event as authoritative proof of payout is an over-trust bug in the consumer,
not a property of the contract.

> **Consumer rule:** `set_int.amount` is the *amount to settle if you choose to
> honour this intent*. It is never evidence that a settlement occurred.

## Payload

`set_int` carries the canonical 9-field decision order with the correlation id
appended (#566). Dispatch by event name and topic version: `set_int` uses
`v2` for the ledger-bound ID encoding, while `sla_calc` remains `v1` (#675/#677):

```
(outage_id: Symbol, status: Symbol, mttr_minutes: u32, threshold_minutes: u32,
 amount: i128, payment_type: Symbol, rating: Symbol, config_version_hash: u64,
 recorded_at: u64, correlation_id: u64)
```

- `status` (`met` / `viol`) selects which side of the intent applies:
  a reward for `met`, a penalty for `viol`.
- `amount` is positive for rewards and negative for penalties. It is a signed
  SLA decision, not an executed transfer or balance delta.
- `payment_type` is the contract-computed lane (`rew` / `pen`); it must stay
  consistent with `status` (`calculate_sla` rejects inconsistencies with
  `InconsistentPaymentStatus`).
- `correlation_id` is deterministic from `(outage_id, ledger_sequence)` via
  `event_correlation::generate_correlation_id`, with the ledger in the upper 32 bits and an outage fingerprint in the lower
  32 bits. Different ledgers are disjoint; same-ledger fingerprints can collide.

The field-level schema is owned by
[`apexchainx_calculator/src/event_schema.rs`](../apexchainx_calculator/src/event_schema.rs)
(see its `set_int` section) — that source is authoritative; this page explains
the semantics.

## Backend settlement contract (what a consumer must do)

1. **Decode, don't infer.** Read `set_int` (and, for the decision record,
   `sla_calc`) from the transaction's event log. Do not treat the mere existence
   of the event as a completed payout.
2. **Own the execution.** The backend (or a dedicated escrow/settlement service)
   decides whether and how to pay. Record the outcome separately from the
   contract's event stream.
3. **Join with full context.** Use network, originating contract, ledger and full
   outage ID alongside `correlation_id`. The compact ID alone is neither unique
   within a ledger nor a payment-deduplication key.
4. **Reconcile on config generation.** `config_version_hash` identifies the
   config generation that produced the decision; a stale intent from an older
   generation should be reconciled rather than blindly paid.
5. **Do not double-pay on replay.** `calculate_sla` is idempotent for exact
   replays (no new history entry, no new `set_int`), but a *config change* opens
   a new generation and can legitimately produce a new intent for the same
   outage within the per-outage recalc cap.

## Related documents

- [event-ordering-guarantees.md](event-ordering-guarantees.md) — where `set_int`
  sits relative to `sla_calc` and other events.
- [EVENT_COMPATIBILITY_POLICY.md](EVENT_COMPATIBILITY_POLICY.md) — append-only
  field rules that keep this payload stable.
- [FAILURE_TAXONOMY.md](FAILURE_TAXONOMY.md) — error paths that can precede an
  intent (e.g. duplicate input).
- [INDEX.md](INDEX.md) — documentation index.
