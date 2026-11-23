# Pause Transition Posture and Last-Transition Readability

> **Issue:** [#680](https://github.com/ApexChainx/ApexChainx-Contracts/issues/680)
> **Status:** Findings recorded; contract change not yet implemented
> **Applies to:** `PauseInfo`, `pause` / `unpause`, `get_full_audit_state`

This document records why the current pause state cannot answer the operational
question the issue asks, and the constraints any fix must respect. It is the
reference for the work described in
[#680](https://github.com/ApexChainx/ApexChainx-Contracts/issues/680).

**Related documents:**
- [`OBSERVABILITY_CONTRACT.md`](OBSERVABILITY_CONTRACT.md) - how backends derive health signals from events
- [`AUDIT_TRAIL.md`](AUDIT_TRAIL.md) - event payload schemas
- [`CONTRACT_LIFECYCLE.md`](CONTRACT_LIFECYCLE.md) - pause semantics
- [`STORAGE_KEY_MIGRATION_CHECKLIST.md`](STORAGE_KEY_MIGRATION_CHECKLIST.md) - the storage-version discipline referenced below

---

## 1. Current shape

`PauseInfo` is defined in `apexchainx_calculator/src/lib.rs:1121` and holds three
fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `reason` | `String` | Operator-supplied reason, bounded by `MAX_REASON_LEN` |
| `paused_at` | `u64` | Ledger timestamp of the pause |
| `paused_by` | `Address` | Admin that performed the pause |

The relevant storage keys are `PAUSED_KEY` (the boolean posture) and
`PAUSE_INFO_KEY` (the metadata), both instance storage.

## 2. The gap: unpause destroys the only record

The pause path writes metadata and emits an event:

```rust
// lib.rs:1793-1804
let paused_at = env.ledger().timestamp();
env.storage().instance().set(&PAUSED_KEY, &true);
env.storage().instance().set(
    &PAUSE_INFO_KEY,
    &PauseInfo { reason, paused_at, paused_by: caller.clone() },
);
env.events()
    .publish((EVENT_PAUSED, EVENT_VERSION, caller), (true,));
```

The unpause path **deletes** that metadata:

```rust
// lib.rs:1814-1817
env.storage().instance().set(&PAUSED_KEY, &false);
env.storage().instance().remove(&PAUSE_INFO_KEY);
env.events()
    .publish((EVENT_UNPAUSED, EVENT_VERSION, caller), (false,));
```

`get_pause_info` returns whatever is at `PAUSE_INFO_KEY`, so it yields `None` once
the contract is unpaused:

```rust
// lib.rs:1828-1831
pub fn get_pause_info(env: Env) -> Result<Option<PauseInfo>, SLAError> {
    Self::check_version(&env)?;
    Ok(env.storage().instance().get(&PAUSE_INFO_KEY))
}
```

This is a deliberate and correct choice for the *current posture* read: while
unpaused there is no active pause, so `None` is the right answer. The cost is
that the last transition is unrecoverable from contract state.

Answering "when did it last pause, who did it, and when was it last resumed?"
requires replaying the event stream from genesis and finding the most recent
`paused` or `unpause` event. The events do carry the caller as a topic and the
ledger supplies the timestamp, so the data exists, but only off-chain.

That is the operational gap in
[#680](https://github.com/ApexChainx/ApexChainx-Contracts/issues/680): compliance
output is stream-dependent rather than self-contained.

Note also the asymmetry that makes this awkward to bolt on. `PauseInfo` is keyed on
`paused_at` and `paused_by`, so it describes a *pause*, not a *transition*. A
last-transition read has to represent the unpause case too, which the current
struct cannot express.

## 3. Constraints on a fix

**Storage-version discipline.** This crate versions its storage and gates reads on
a stored version. A new storage entry cannot simply be introduced and read
unguarded: contracts already deployed on-chain will not have it, and
`get_*` functions call `Self::check_version(&env)?` before touching storage. The
process for adding a field is documented in
[`STORAGE_KEY_MIGRATION_CHECKLIST.md`](STORAGE_KEY_MIGRATION_CHECKLIST.md) and
[`STORAGE_FOOTPRINT_POLICY.md`](STORAGE_FOOTPRINT_POLICY.md); the version constant
and `migrate` entry point live in `apexchainx_calculator/src/storage_version.rs`.
A new key must be absent-tolerant on read, or it must be backfilled by `migrate`.

**Prefer additive changes to `PauseInfo`.** Extending `PauseInfo` in place changes
its serialized shape. Several tests assert exact event payload shapes, and
`api_stability.rs` pins the field count of the public contract types. Adding a
separate last-transition key, or a new read that returns a distinct struct, avoids
breaking the `paused` / `unpause` event contracts, which are load-bearing for the
backend indexer described in
[`OBSERVABILITY_CONTRACT.md`](OBSERVABILITY_CONTRACT.md).

**Do not repurpose the `unpause` event.** Its payload is the single boolean
`(false,)` and is asserted as such by `test_pause_event_payload_is_single_bool`
and `test_unpause_event_payload_is_single_bool`. Widening it would be a breaking
wire change, not an additive one.

## 4. What a fix must satisfy

Acceptance criteria from the issue, and what each still needs:

| Criterion | State |
| --- | --- |
| Last-transition info queryable | **Not met.** `unpause` removes `PAUSE_INFO_KEY`; the transition is off-chain only. |
| Version discipline respected | **Not met.** No new key or field has been added. |
| Docs describe the posture read | **Met** by this document. |

A conforming fix needs a read that survives the unpause, carries both the last
pause and the last unpause (timestamp and actor), tolerates absence on
pre-migration state, and leaves the `paused` and `unpause` event payloads exactly
as they are.
