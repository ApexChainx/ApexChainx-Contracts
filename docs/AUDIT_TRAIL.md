# SLA Calculator Event Audit Trail

> **Authoritative reference:** Every contract event that the `apexchainx_calculator`
> contract can emit, with topic layout, payload fields, emitter site, and backend
> recovery implications — sourced directly from
> [`event_schema.rs`](../apexchainx_calculator/src/event_schema.rs) and
> [`lib.rs`](../apexchainx_calculator/src/lib.rs).
>
> **Audience:** Backend indexers (apexchainx-be), dashboarding, audit, replay,
> and any integrator who needs to subscribe to the contract without reading
> Rust.
>
> **Coverage:** All 19 events currently emitted by the contract, including the
> canonical calculation/config/governance lifecycle, the storage-migration
> lifecycle, and the running-stats saturation signal.

<p align="center">
  <img src="https://img.shields.io/badge/audience-backend_integrators-blue" alt="Audience: Backend integrators" />
  <img src="https://img.shields.io/badge/event_version-v1-success" alt="Event version: v1" />
  <img src="https://img.shields.io/badge/topic_arity-3-informational" alt="Topic arity: 3" />
</p>

---

## Table of Contents

- [How to use this document](#how-to-use-this-document)
- [Conventions](#conventions)
  - [Universal topic layout](#universal-topic-layout)
  - [Canonical event version](#canonical-event-version)
  - [Where event constants live in code](#where-event-constants-live-in-code)
- [Symbol discovery endpoints](#symbol-discovery-endpoints)
- [Correlation IDs (SC-W5-079)](#correlation-ids-sc-w5-079)
- [Event ordering invariants (SC-W5-042)](#event-ordering-invariants-sc-w5-042)
- [Topic stability contract (SC-W5-043)](#topic-stability-contract-sc-w5-043)
- [Event catalog summary](#event-catalog-summary)
- [Event reference](#event-reference)
  - [SLA calculation: `sla_calc` and `set_int`](#sla-calculation-sla_calc-and-set_int)
  - [Configuration update: `cfg_upd`](#configuration-update-cfg_upd)
  - [Pause / unpause: `paused` and `unpause`](#pause--unpause-paused-and-unpause)
  - [Operator assignment (single-step): `op_set`](#operator-assignment-single-step-op_set)
  - [Operator two-step handoff: `op_prop`, `op_acc`, `op_can`](#operator-two-step-handoff-op_prop-op_acc-op_can)
  - [Admin two-step transfer: `adm_prop`, `adm_acc`, `adm_can`, `adm_ren`](#admin-two-step-transfer-adm_prop-adm_acc-adm_can-adm_ren)
  - [Configuration freeze: `cfg_frz` and `cfg_unfrz`](#configuration-freeze-cfg_frz-and-cfg_unfrz)
  - [History pruning: `pruned` and `pruned_a`](#history-pruning-pruned-and-pruned_a)
  - [Running-stats saturation: `stats_sat` (SC-W5-047)](#running-stats-saturation-stats_sat-sc-w5-047)
  - [Storage migration: `migrate_done`](#storage-migration-migrate_done)
- [Symbol deprecation protocol](#symbol-deprecation-protocol)
- [Replay & recovery playbook](#replay--recovery-playbook)
  - [Missed events](#missed-events)
  - [Out-of-order replay](#out-of-order-replay)
  - [Counter saturation](#counter-saturation)
  - [Re-bootstrap after a contract upgrade](#re-bootstrap-after-a-contract-upgrade)
- [Source of truth](#source-of-truth)

---

## How to use this document

This document is the **operational one-pager** for an integrator consuming
SLA-calculator events. It is intentionally derived from, and subordinate to,
the canonical Rust source comments at the top of
[`event_schema.rs`](../apexchainx_calculator/src/event_schema.rs) and the
`EVENT_*` constant block in [`lib.rs`](../apexchainx_calculator/src/lib.rs).
If this document ever disagrees with those files, **the source files are
correct** — this document should be updated to match.

For context on the broader system architecture and the Soroban execution
environment, see [`CODEX_CONTEXT.md`](CODEX_CONTEXT.md) and
[`PROJECT_CONTEXT.md`](PROJECT_CONTEXT.md). For detailed configuration
validation rules that gate `cfg_upd`, see [`config-validation.md`](config-validation.md).
For storage-cost and regression baselines, see
[`sc-w5-storage-and-cost-baselines.md`](sc-w5-storage-and-cost-baselines.md).

If you maintain the contract, when you add or break an event:

1. Update `event_schema.rs` (catalogue + payload schema) **and**
2. Update `lib.rs` `EVENT_*` constants **and**
3. Update this audit trail (or it will silently drift from the source of truth).

---

## Conventions

### Universal topic layout

All 19 events use **exactly three topics**, in this order:

| Index | Name | Type | Meaning |
|-------|------|------|---------|
| `topic[0]` | Event name | `Symbol` | One of the `EVENT_*` constants (`sla_calc`, `cfg_upd`, …) |
| `topic[1]` | Event version | `Symbol` | Always `"v1"` today (see [Canonical event version](#canonical-event-version)). |
| `topic[2]` | Event context | `Symbol` or `Address` | Either the severity (`critical` / `high` / `medium` / `low`) or the caller's `Address`, depending on event kind. |

Soroban enforces arity at the host — indexers can hard-code a 3-element
topic vector and never need to handle shorter or longer ones. This
invariant is asserted by
[`topic_stability_tests.rs`](../apexchainx_calculator/src/topic_stability_tests.rs).

### Canonical event version

The event-version symbol is `"v1"` today, exposed as
`event_schema::EVENT_VERSION` and re-exported publicly as
`get_contract_metadata()`. **Backward-compatible additions** (a new field
appended to the end of a payload, or a brand-new event) do **not** require
a version bump. **Breaking changes** (field removal, type change, payload
reordering) MUST bump the version to `v2` and update the
`event_schema.rs` catalogue in the same release.

### Where event constants live in code

| File | Contents |
|------|----------|
| [`event_schema.rs`](../apexchainx_calculator/src/event_schema.rs) | Canonical event **catalogue** (rustdoc with payload schemas) + `EVENT_*` `Symbol` constants + deprecation protocol doc |
| [`lib.rs`](../apexchainx_calculator/src/lib.rs) (`EVENT_*` constants) | Mirrored `pub(crate) const EVENT_*: Symbol` reused at every `env.events().publish(...)` site |
| [`lib.rs`](../apexchainx_calculator/src/lib.rs) (payload schema block) | A second, compact listing of every publish-site payload as a comment near the top of the file |
| [`calculation.rs`](../apexchainx_calculator/src/calculation.rs) | Implements `calculate_sla` / `increment_stats` (the emitters of `sla_calc`, `set_int`, `stats_sat`) |

When a backend reads a topic index, accept `Symbol` and compare against the
literal string (`"sla_calc"`, `"cfg_upd"`, …) — the `Symbol` length limit
of 32 bytes is sufficient for every current event name.

---

## Symbol discovery endpoints

The contract exposes three read-only views that backends should call once at
startup (or after an upgrade) to pin every symbol/label they rely on.
None of these views are required to interpret event *topics* — those are
defined by the `EVENT_*` constants — but they pin every **payload symbol**
so consumers can adapt across releases.

| View | Returns | Use it for |
|------|---------|-----------|
| `get_contract_metadata()` | `ContractMetadata` | Available features, supported severities, schema versions |
| `get_result_schema()` | `SLAResultSchema` | Status / payment-type / rating symbol vocabulary + `deprecated_symbols` |
| `get_failure_schema()` | `FailureSchema` | Numeric → label → description mapping for every typed `SLAError` |
| `get_config_snapshot()` | `SLAConfigSnapshot` | Canonical severity list + active configuration values |

A consumer that wants to be robust against the
[Symbol deprecation protocol](#symbol-deprecation-protocol) should compare
its locally cached symbol strings against `get_result_schema()` on
startup and log a warning if any cached symbol appears in
`deprecated_symbols` with `removal_version` populated.

---

## Correlation IDs (SC-W5-079)

Cross-contract workflows (an `sla_calc` that triggers a future
payment-escrow release) are stitched together by a `CorrelationId`, a
`u64` deterministically derived from the ledger sequence by
[`event_correlation.rs`](../apexchainx_calculator/src/event_correlation.rs).

Properties a backend can rely on:

| Property | Guarantee |
|----------|-----------|
| Determinism | Same `(outage_id, ledger_sequence)` ⇒ same `CorrelationId` |
| Uniqueness | Different ledger sequences ⇒ distinct IDs (FNV-1a hash of ledger sequence) |
| Non-zero | Always non-zero (FNV offset basis seeded) |
| Topic placement | Correlation IDs are **payload-only**, NOT a topic. They never change the 3-topic layout. |

In the current `apexchainx_calculator` emissions, the `set_int` payload
already carries the recoverable `config_version_hash` and `recorded_at`
fields that backends use for the same dedupe purpose. The
`CorrelationId` becomes relevant once the future `payment_escrow` and
`settlement` contracts ([roadmap](../docs/PROJECT_CONTEXT.md#sc-100-future-contract-roadmap))
begin emitting events of their own.

---

## Event ordering invariants (SC-W5-042)

These invariants are locked by
[`event_ordering_tests.rs`](../apexchainx_calculator/src/event_ordering_tests.rs):

1. **`cfg_upd` precedes any `sla_calc`/`set_int` that uses that config.**
   Backend consumers who maintain an in-memory cache keyed by
   `config_version_hash` should re-read `get_config_snapshot()` after every
   `cfg_upd` (or invalidate the cache).
2. **`sla_calc` and `set_int` for a single `calculate_sla` call are
   emitted in that order, immediately adjacent in the event stream** (the
   `set_int` is the *settlement intent* — see
   [SLA calculation](#sla-calculation-sla_calc-and-set_int)).
3. **Calls to `calculate_sla` produce events in call order**: the
   `sla_calc` for call *N+1* always follows the `set_int` for call *N*.
4. **`paused` is emitted before any operation is blocked; `unpause` is
   emitted before blocked operations resume.** Backends can use these as
   hard "calculation slot" delimiters.
5. **Two-step governance lifecycle events** (`adm_prop`, `adm_acc`,
   `adm_can`, `adm_ren`, `op_prop`, `op_acc`, `op_can`) are emitted
   strictly in lifecycle order — a backend may treat each cycle's first
   `*_prop` event as the start of a new governance attempt and the next
   `*_acc` / `*_can` as the terminal event.
6. **Every `calculate_sla` call emits exactly one `sla_calc` + exactly
   one `set_int`** (subject to the contract not being paused) — there is
   no half-emitted state to reconcile.

---

## Topic stability contract (SC-W5-043)

[`topic_stability_tests.rs`](../apexchainx_calculator/src/topic_stability_tests.rs)
pins the structure so indexers can rely on it:

- `topic[0]` is always the event name (`Symbol`).
- `topic[1]` is always the literal event version `Symbol` — equal to
  `event_schema::EVENT_VERSION`.
- `topic[2]` is the event's context — `Symbol` (severity) for SLA-class
  events, `Address` (caller) for governance-class events, or a
  `Symbol` counter name for `stats_sat`.
- Topic arity never changes (no future event will have 2 or 4 topics).

---

## Event catalog summary

| # | Event | Topic [2] | Emitter function | Auth required |
|---|-------|-----------|------------------|---------------|
| 1 | `sla_calc` | severity | `calculate_sla` | operator |
| 2 | `set_int` | severity | `calculate_sla` | operator (same call) |
| 3 | `cfg_upd` | severity | `set_config`, `set_custom_severity`, `remove_custom_severity` | admin |
| 4 | `paused` | caller `Address` | `pause` | admin |
| 5 | `unpause` | caller `Address` | `unpause` | admin |
| 6 | `op_set` | caller `Address` | `set_operator` | admin |
| 7 | `op_prop` | caller `Address` | `propose_operator` | admin |
| 8 | `op_acc` | caller `Address` | `accept_operator` | pending operator |
| 9 | `op_can` | caller `Address` | `cancel_operator_proposal` | admin |
| 10 | `adm_prop` | caller `Address` | `propose_admin` | admin |
| 11 | `adm_acc` | caller `Address` | `accept_admin` | pending admin |
| 12 | `adm_can` | caller `Address` | `cancel_admin_proposal` | admin |
| 13 | `adm_ren` | caller `Address` | `renounce_admin` | admin |
| 14 | `cfg_frz` | caller `Address` | `freeze_config` | admin |
| 15 | `cfg_unfrz` | caller `Address` | `unfreeze_config` | admin |
| 16 | `pruned` | caller `Address` | `prune_history` | admin |
| 17 | `pruned_a` | caller `Address` | `prune_history_by_age` | admin |
| 18 | `stats_sat` | counter-name `Symbol` | `calculate_sla` (via `increment_stats`) | implicit (operator) |
| 19 | `migrate_done` | caller `Address` | `migrate` | admin |

Group by purpose:

- **SLA calculation** — `sla_calc`, `set_int`, `stats_sat`
- **Configuration** — `cfg_upd`, `cfg_frz`, `cfg_unfrz`
- **Pause / lifecycle** — `paused`, `unpause`
- **Operator management** — `op_set`, `op_prop`, `op_acc`, `op_can`
- **Admin governance** — `adm_prop`, `adm_acc`, `adm_can`, `adm_ren`
- **History maintenance** — `pruned`, `pruned_a`
- **Storage migration** — `migrate_done`

---

## Event reference

The sections below document each event with its topic layout, payload fields,
emission conditions, and operational guidance.

### SLA calculation: `sla_calc` and `set_int`

Every successful call to `calculate_sla(operator, outage_id, severity, mttr_minutes)`
emits **two events in immediate succession**: `sla_calc` first, then
`set_int`. Backends that need to ingest a single "SLA decision" should
correlate the two by reading `outage_id`, `recorded_at`, and
`config_version_hash` from the `set_int` payload (these are identical to
the analogous `SLAResult` fields).

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `sla_calc` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Symbol` | Severity at calculation time (`critical`, `high`, `medium`, `low`, or a registered custom severity) |

**`sla_calc` payload fields** (in this exact order — do not reorder):

| # | Field | Type | Notes |
|---|-------|------|-------|
| 1 | `outage_id` | `Symbol` | Echo of the `calculate_sla` argument |
| 2 | `status` | `Symbol` | `met` or `viol` (see `get_result_schema().status_met` / `status_violated`) |
| 3 | `payment_type` | `Symbol` | `rew` or `pen` |
| 4 | `rating` | `Symbol` | `top` / `excel` / `good` / `poor` |
| 5 | `mttr_minutes` | `u32` | Echo of the argument |
| 6 | `threshold_minutes` | `u32` | Threshold that was applied for the (now possibly updated) severity |
| 7 | `amount` | `i128` | Signed financial outcome — positive = reward, negative = penalty |

**`set_int` payload fields** (settlement intent, emitted for backend
reconciliation alongside `sla_calc`):

| # | Field | Type | Notes |
|---|-------|------|-------|
| 1 | `outage_id` | `Symbol` | Same as `sla_calc` |
| 2 | `status` | `Symbol` | Same as `sla_calc` |
| 3 | `payment_type` | `Symbol` | Same as `sla_calc` |
| 4 | `amount` | `i128` | Same semantic convention as `sla_calc.amount` |
| 5 | `config_version_hash` | `u64` | Deterministic config hash at the time of calculation — **primary dedupe key** |
| 6 | `recorded_at` | `u64` | Ledger timestamp at calculation time (wall-clock seconds) |

**Recovery / replay implications:**

- **Dedupe key** — `(outage_id, config_version_hash, recorded_at)` is the
  authoritative triple. Backends should treat the (rare) case of
  identical `outage_id` + identical `config_version_hash` + identical
  `recorded_at` as a duplicate-to-skip from event-replay.
- **Partial pairs** — if a backend sees a `set_int` without a preceding
  `sla_calc` in the same ledger batch, replay the ledger for the
  matching `outage_id` rather than reconstructing from the `set_int`
  alone. The `sla_calc` is the canonical decision; `set_int` is the
  settlement handshake.
- **Configuration detachment** — if the active severity configuration
  changed *after* the `recorded_at` timestamp, the event's
  `threshold_minutes` reflects the config that was active *at
  `recorded_at`*, not the config now. Use `cfg_upd` events + the
  `config_version_hash` to reconstruct historical point-in-time configs.
- **Pause boundary** — if `calculate_sla` was attempted while the
  contract was paused, **no events are emitted and the call panics
  with `ContractPaused`**. Backends reconciling state should not
  synthesize missing `sla_calc` events during a pause window; instead,
  surface the gap as a paused-period audit marker.
- **Idempotent replay** — resubmitting an `outage_id` with an unchanged
  `config_version_hash` and identical inputs succeeds but stores nothing
  and **emits no events**; the already-stored decision is returned. A
  successful `calculate_sla` response is therefore *not* proof that a new
  `sla_calc` was emitted. Backends must key off the
  `(outage_id, config_version_hash, recorded_at)` triple above rather than
  counting responses, and should not treat a missing event after a retry as
  a dropped event.
- **Recalculation cap** — a *changed* `config_version_hash` lets the same
  outage be recorded again (a new generation, with its own `sla_calc` /
  `set_int` pair). One outage may hold at most 16 retained entries; beyond
  that the call is rejected with `OutageRecalcLimit` and, as with any error,
  no events are emitted. Admin pruning frees headroom.

### Configuration update: `cfg_upd`

Emitted by `set_config`, `set_custom_severity`, and
`remove_custom_severity`. The `cfg_upd` event always carries the
**post-write** values.

When `remove_custom_severity` succeeds, the emitted `cfg_upd` payload
uses zeros (so indexers can distinguish a deletion from a "set to
zero" — see [`CHANGELOG.md`](../CHANGELOG.md) for the explicit
contract on this).

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `cfg_upd` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Symbol` | The severity whose config was written — canonical or custom |

**Payload fields:**

| # | Field | Type | Notes |
|---|-------|------|-------|
| 1 | `threshold_minutes` | `u32` | New threshold value (0 prior to deletion; see above) |
| 2 | `penalty_per_minute` | `i128` | New penalty rate (0 prior to deletion) |
| 3 | `reward_base` | `i128` | New reward base (0 prior to deletion) |

**Recovery / replay implications:**

- **Cache invalidation** — backends that pin a server-side cache keyed
  on `(severity, config_version_hash, snapshot_ledger)` must
  invalidate the entry on every `cfg_upd`, then re-read
  `get_config_snapshot()`. The on-chain `LAST_CFG_UPDATE_KEY`
  exposed via `get_last_config_update()` is a cheap staleness check.
- **Validation gating** — `cfg_upd` is only emitted if `set_config`
  validation passes; failed updates emit no event. Backends should
  not assume "no `cfg_upd` for one block ⇒ unchanged" — the admin
  may have submitted an invalid update that was rejected silently.
  Compare `get_config_snapshot()` against the last cached value if a
  hard guarantee is needed.
- **Cross-severity ordering** — Issue #92 guarantees the contract
  validates that `critical.penalty ≥ high.penalty ≥ medium.penalty ≥
  low.penalty` *before* emitting `cfg_upd`. If a backend observes
  an order inversion in its cached snapshot after replay, that order
  inversion is a contract-level bug, not a stale index.

### Pause / unpause: `paused` and `unpause`

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `paused` or `unpause` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Address` | The admin who called `pause` / `unpause` |

**`paused` payload:** `(true,)` — a sole `bool` confirming the
post-state (true).
**`unpause` payload:** `(false,)`.

`paused` is always emitted *before* the contract begins rejecting
mutating calls. There is no separate event for the pause *reason* or
`paused_at` timestamp — those live in the `PauseInfo` struct returned
by `get_pause_info()` and cease to exist the moment `unpause` is
called.

**Recovery / replay implications:**

- **Calm-window marker** — a backend can use a `paused` event as a
  clean audit-window boundary: no further `sla_calc`, `set_int`,
  `cfg_upd`, or similar events will be observed until a matching
  `unpause`. Activity during a `paused`-without-`unpause` interval
  means the indexer is reading from a wrong (or replayed) ledger
  range.
- **Reason off-chain** — the reason string is *not* in the event
  payload. If your UI surfaces pause reasons, call
  `get_pause_info()` directly. The reason field is bounded to
  [`MAX_REASON_LEN = 256` bytes](../apexchainx_calculator/src/lib.rs)
  — refuse to write longer strings.

### Operator assignment (single-step): `op_set`

Emitted once and only once by `set_operator` after the new operator is
saved. There is no two-step ceremony in this path because
`propose_operator`/`accept_operator` cover the deliberate handoff
case.

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `op_set` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Address` | The admin who set the operator |

**Payload:** `(new_operator: Address,)` — a sole tuple field of the
new operator address.

**Recovery / replay implications:**

- Backends that mirror "current operator" off-chain should treat
  every `op_set` as a hard overwrite. There is no rollback event —
  only a fresh `op_set` or an `adm_acc` (admin change implies
  operator is still governed by the admin role, but the operator
  field is independent).

### Operator two-step handoff: `op_prop`, `op_acc`, `op_can`

| Event | Meaning | Emitter |
|-------|---------|---------|
| `op_prop` | Pending operator set | `propose_operator` |
| `op_acc` | Pending operator accepted (now current) | `accept_operator` |
| `op_can` | Pending operator proposal cleared | `cancel_operator_proposal` |

All three share the same topic layout:

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | One of `op_prop` / `op_acc` / `op_can` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Address` | The caller on the emitting function (admin for `op_prop`/`op_can`, the new operator for `op_acc`) |

**`op_prop` payload:** `(new_operator: Address,)` — the pending
candidate.
**`op_acc` payload:** *empty payload tuple* `()` — only the topics
identify the event.
**`op_can` payload:** *empty payload tuple* `()`.

`op_prop` + a matching `op_acc` is the happy-path lifecycle. `op_prop`
+ a matching `op_can` is the cancellation lifecycle. Any `op_prop`
without a matching `op_acc`/`op_can` in the same governance cycle is
treated as the **pending** state — `get_pending_operator()` returns
`Some(_)` while the proposal is open.

**Recovery / replay implications:**

- **Dedupe** — backends that surface "operator pending" banners can
  use `op_prop` as enter-pending and either `op_acc` or `op_can` as
  exit-pending. The pending state is also recoverable from
  `get_pending_operator()` if the event stream is incomplete.

### Admin two-step transfer: `adm_prop`, `adm_acc`, `adm_can`, `adm_ren`

Mirror of the operator two-step flow, plus the unique `adm_ren`
"renounce" event which leaves **no admin at all** (irreversible).

| Event | Meaning | Emitter |
|-------|---------|---------|
| `adm_prop` | Pending admin set | `propose_admin` |
| `adm_acc` | Pending admin accepted (now current) | `accept_admin` |
| `adm_can` | Pending admin proposal cleared | `cancel_admin_proposal` |
| `adm_ren` | Current admin renounced permanently | `renounce_admin` |

All four share the same topic layout:

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | One of `adm_prop` / `adm_acc` / `adm_can` / `adm_ren` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Address` | The caller on the emitting function (current admin for `adm_prop` / `adm_can` / `adm_ren`; the new admin for `adm_acc`) |

**Payloads:**

| Event | Payload |
|-------|---------|
| `adm_prop` | `(new_admin: Address,)` |
| `adm_acc` | `()` (empty tuple) |
| `adm_can` | `()` |
| `adm_ren` | `()` |

**Recovery / replay implications:**

- **Renounce is terminal** — once an `adm_ren` is observed, no
  admin-gated function will ever succeed again. A backend must
  surface this as **permanent loss of admin capability** on the
  contract — there is no path to recover short of redeploying.
- **Dedupe** — same pattern as operator transfers: pair `adm_prop`
  with its terminating `adm_acc` or `adm_can`. An unmatched `adm_prop`
  is the pending state.

### Configuration freeze: `cfg_frz` and `cfg_unfrz`

Admin-only. While the configuration is frozen, `set_config` and
`set_custom_severity`/`remove_custom_severity` will panic with
`ConfigFrozen`. `cfg_upd` is *not* the inverse of `cfg_frz` — config
can be unfrozen without any `cfg_upd` between freeze and unfreeze.

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `cfg_frz` or `cfg_unfrz` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Address` | Admin who froze / unfroze |

**Payloads:** both events emit an *empty payload tuple* `()`.

**Recovery / replay implications:**

- **Mute window** — between a `cfg_frz` and the matching
  `cfg_unfrz`, backends that pre-compute "next config" updates from
  policy files should treat the interval as a hard mute on config
  changes — assume any `cfg_upd` observed in that window is a
  contract bug, not honest state.
- **Recovery** — `get_last_config_update()` will not change during
  the freeze window (verified in tests).

### History pruning: `pruned` and `pruned_a`

Two distinct prune paths:

- `pruned` — emitted by `prune_history()` when an *explicit* history
  cap is enforced (admin-defined retention limit).
- `pruned_a` — emitted by `prune_history_by_age()` when
  *age-based* pruning removes rows older than a threshold.

Both share the same payload schema because their semantics are the
same: "we compacted N rows, and M remain".

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `pruned` or `pruned_a` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Address` | Admin who called the prune |

**Payload:** `(removed_count: u32, kept_count: u32)`.

**Recovery / replay implications:**

- **History-length delta** — backends that mirror history off-chain
  must apply the delta: drop `removed_count` rows (the *oldest*
  ones, by `recorded_at`) and re-host with the new cap = `kept_count`.
- **Replay determinism** — pruning is order-stable: older records
  are removed first regardless of insertion order ([SC-W5
  baselines](sc-w5-storage-and-cost-baselines.md#pruning-by-age-chronology)).
  A backend replaying events out of order will still arrive at the
  same final retained set, so long as it consumes every
  `pruned`/`pruned_a` in event-stream order.
- **No content payload** — pruned rows are not individually
  identified in the event. If a backend needs per-row history of
  items **before** compaction, it must consume the events before
  the matching `pruned`/`pruned_a` — there is no way to recover
  pruned records from events alone.

### Running-stats saturation: `stats_sat` (SC-W5-047)

Emitted by `calculate_sla` via `increment_stats` when one of the
four running counters (`totcalc`, `totviol`, `totrew`, `totpen`)
hits its type-bound ceiling (`u64::MAX` for counters, integer-bound
for the i128 totals).

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `stats_sat` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Symbol` | Counter name that saturated (`totcalc`, `totviol`, `totrew`, or `totpen`) |

**Payload:** `(field: Symbol, previous_value: i128, attempted_increment: i128)`.

The **counter itself remains capped** on-chain after the saturating
attempt — this event is *not* an unbounded overflow signal. It is a
**read-honesty** signal: callers that rely on the affected field as
"total economic exposure" must now understand that the on-chain
total is *less than* the true cumulative.

`totcalc` and `totviol` payloads carry their (still fitting) `prev`
and (already-at-bound) `attempted_increment` as `i128` for
uniformity — the in-storage type is `u64` for these two counters.
The semantics for those counters are: the `attempted_increment` is
logically `1` for `totcalc` and `1` for `totviol` (one increment per
`calculate_sla` call), carried over the i128 boundary as `1` so
downstream code can use a uniform type.

**Recovery / replay implications:**

- **Most-important recovery event.** Treat any `stats_sat` as a
  durability boundary for the affected counter.
- **Cumulative drift** — once `stats_sat` has fired for `totcalc`
  or `totviol`, the on-chain count of *calculations* or *violations*
  no longer equals the historical count. Backends that compute
  violation rate from `stats_sat` will silently under-report. The
  applied remediation is to maintain an *off-chain* running total
  that the backend has been keeping since the first event of the
  lifetime of the contract (so the backend must capture
  pre-saturation totals before `stats_sat` is observed).
- **Economic exposure ceiling** — backends that surface "total
  rewards paid" / "total penalties paid" via `get_stats()` must mark
  the corresponding dashboard with a "counter saturated" warning
  badge once a `stats_sat` is seen for `totrew` or `totpen`. The
  contract enforces the bound; the backend cannot ask the contract
  for a higher-precision total.
- **Per-event payload** — backends can pre-aggregate the inflation
  load by summing `attempted_increment - previous_value` across
  every `stats_sat` event. This is the most precise measurement of
  "lost" precision and is the right input for dashboards.

### Storage migration: `migrate_done`

> **Implementation note for indexers:** `EVENT_MIGRATE_DONE` is the one
> event whose name constant is declared as `&str` (not `Symbol`) in
> [`event_schema.rs`](../apexchainx_calculator/src/event_schema.rs),
> because the literal `"migrate_done"` is too long for Soroban's 9-byte
> `symbol_short!` limit. The publish site in
> [`lib.rs`](../apexchainx_calculator/src/lib.rs) wraps it with
> `Symbol::new(&env, EVENT_MIGRATE_DONE)`, so to the indexer `topic[0]`
> is still the Symbol value `"migrate_done"` — just constructed at
> emission time rather than via `symbol_short!.`

Emitted by the admin-only `migrate()` function at the end of a successful
storage migration. The migrate function applies each storage-version
step in sequence (`v0→v1`, `v1→v2`, …) so a contract that is multiple
versions behind is brought fully up to date in one call.

| Field | Type | Description |
|-------|------|-------------|
| `topic[0]` | `Symbol` | `migrate_done` |
| `topic[1]` | `Symbol` | `v1` |
| `topic[2]` | `Address` | The admin who called `migrate` |

**Payload:** `(old_version: u32, new_version: u32)`.

**Recovery / replay implications:**

- **Hard upgrade boundary** — `migrate_done` is the *only* signal a
  backend gets that the contract has changed storage shape. Treat it
  as a *mandatory* checkpoint: re-read `get_version_info()` /
  `get_contract_metadata()`, re-pin all symbol tables via
  `get_result_schema()` and `get_failure_schema()`, drop any cached
  shape assumptions inherited from the prior storage version.
- **Idempotent migrate** — calling `migrate()` when the storage
  is already current is a *no-op*: no `migrate_done` event is
  emitted. Do not assume "missing `migrate_done` ⇒ no migration
  happened" — also check `get_storage_version()`.
- **Future-proofing** — the migration function applies each step in
  sequence, so the `old_version` might be more than one version
  below `new_version`. A single `migrate_done` event captures the
  whole span.

---

## Symbol deprecation protocol

When a new symbol supersedes an old one (e.g., a future `"violated"`
replacing `"viol"`), the contract follows a three-phase lifecycle.
The protocol is documented in
[`event_schema.rs`](../apexchainx_calculator/src/event_schema.rs) and
mirrored in [`CODEX_CONTEXT.md`](CODEX_CONTEXT.md#symbol-deprecation-protocol).

1. **Introduction (minor release)** — Both the old and new symbols
   are emitted in every event. `get_result_schema()` returns a
   `deprecated_symbols` entry marking the old symbol.
2. **Coexistence (≥ 1 minor release)** — Old symbol still emitted.
   `deprecated_symbols.removal_version = None` (TBD).
3. **Removal (major release)** — Old symbol removed from emission.
   `schema_version` in `get_result_schema()` is bumped. The
   `deprecated_symbols` entry keeps its entry, now with
   `removal_version` populated.

**Backend obligations:**

- At startup, compare your locally cached symbols against
  `get_result_schema().deprecated_symbols`. **Log a warning for any
  deprecated symbol you still rely on.**
- If `removal_version` is `Some(v)` and `schema_version ≥ v`, the
  deprecated symbol **will no longer be emitted**. Stop relying on
  it.

---

## Replay & recovery playbook

### Missed events

A backend that has been offline for N ledgers must:

1. Detect the gap — compare the last processed-ledger-sequence
   against the network's current ledger.
2. Replay by event index — `getEvents` with explicit `startLedger` /
   `endLedger` works because the contract's events are deterministic
   for a deterministic input.
3. Cross-check with on-chain state — for any `sla_calc`, fetch the
   current `SLAResult` for the same `(outage_id, recorded_at)` via
   a paging read (`get_history_by_outage` / `get_latest_by_outage`)
   and assert that the event payload matches. **Any divergence is a
   double-execution-class risk and should be alerted, not silently
   reconciled.**
4. Rebuild indexes using the `set_int.config_version_hash` as the
   dedupe key — never re-process the same `(outage_id,
   config_version_hash, recorded_at)` twice.

### Out-of-order replay

The contract's events are deterministic, so reordering the event
stream is safe **only if** end-state semantics are
order-independent for the indexer's use-case.

| Use case | Order-sensitive? |
|----------|------------------|
| Config cache invalidation | **Yes** — apply every `cfg_upd` in event order |
| Running totals (`totcalc`, `totviol`, `totrew`, `totpen`) | Yes — each `sla_calc` increments one of these |
| Arbitrary receipt store keyed by `outage_id` | No |
| Per-outage audit log | Yes — apply `pruned` / `pruned_a` deltas in order |

When in doubt, replay the event stream in the exact emission order
recorded by `getEvents`. The ordering invariants in
[SC-W5-042](#event-ordering-invariants-sc-w5-042) make that
deterministic.

### Counter saturation

Whenever a `stats_sat` is observed:

1. Mark the affected counter as *under-reporting* on every dashboard
   that depends on it.
2. From that point on, treat the on-chain total as a *lower bound*.
3. Use the cumulative sum of `(attempted_increment - previous_value)`
   across every subsequent `stats_sat` event to estimate the
   additional "lost" precision; surface this on the dashboard.

There is no contract-level fix for saturation — the only remedy is
a future contract upgrade with widened counter types. Until then,
the only authoritative exposure numbers come from the off-chain
aggregator, never from `get_stats()` for the saturated field.

### Re-bootstrap after a contract upgrade

A `migrate_done` event (or any indication of a `STORAGE_VERSION`
bump in `get_version_info()`) is the cue for a full re-bootstrap:

1. Re-read `get_contract_metadata()` — note the new
   `storage_version` and `result_schema_version`.
2. Re-read `get_result_schema()` — check `schema_version` and
   `deprecated_symbols` again; pin new symbol vocabulary.
3. Re-read `get_failure_schema()` — pick up any newly added
   `SLAError` labels.
4. Re-read `get_config_snapshot()` — note any canonical-severity
   additions or removals.
5. Re-initialize any in-memory caches keyed by `config_version_hash`
   from scratch — the snapshot shape *may* have changed.

Skipping this list causes silent breakage on the next `calculate_sla`
call. It is idempotent and cheap — typically a single batch of read
RPCs.

---

## Source of truth

| Resource | File | Role |
|----------|------|------|
| Canonical event catalogue (rustdoc) | [`apexchainx_calculator/src/event_schema.rs`](../apexchainx_calculator/src/event_schema.rs) | Effective payload schema and event-version documentation |
| `EVENT_*` constants (definition) | [`apexchainx_calculator/src/lib.rs`](../apexchainx_calculator/src/lib.rs) | All `Symbol` constants referenced by `env.events().publish` |
| Event payload block (compact listing) | [`apexchainx_calculator/src/lib.rs`](../apexchainx_calculator/src/lib.rs) | Mirrored payload schemas near top of file |
| Topic stability assertions | [`apexchainx_calculator/src/topic_stability_tests.rs`](../apexchainx_calculator/src/topic_stability_tests.rs) | Enforces `topic[0]`/`topic[1]`/`topic[2]` invariants |
| Event ordering assertions | [`apexchainx_calculator/src/event_ordering_tests.rs`](../apexchainx_calculator/src/event_ordering_tests.rs) | Enforces ordering invariants enumerated above |
| Correlation IDs | [`apexchainx_calculator/src/event_correlation.rs`](../apexchainx_calculator/src/event_correlation.rs) | Deterministic trace ID generation (SC-W5-079) |
| Maturity changelog | [`CHANGELOG.md`](../CHANGELOG.md) | Records new functions and breaking changes per release |

If you spot a mismatch, **the source files win**. Open a PR that
fixes *this document* to match — every contract change should ship
with a doc update (not the other way around).
