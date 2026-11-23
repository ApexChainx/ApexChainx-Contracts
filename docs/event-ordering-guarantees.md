# Event Ordering Guarantees

> **Status:** Canonical ordering contract for backend consumers and contributors  
> **References:** Issue #252, SC-W5-041, SC-W5-042  
> **Last updated:** 2026-07-29  
> **Backed by tests in:** `apexchainx_calculator/src/event_ordering_tests.rs`  
> **Canonical event names:** `apexchainx_calculator/src/event_schema.rs`

## Table of Contents

- [Overview](#overview)
- [Event Catalog (Canonical Names)](#event-catalog-canonical-names)
- [Ordering Guarantees](#ordering-guarantees)
- [Single-Ledger Multi-Operation Semantics](#single-ledger-multi-operation-semantics)
- [What Is NOT Guaranteed](#what-is-not-guaranteed)
- [Backend Integration Guidance](#backend-integration-guidance)
- [Test Coverage](#test-coverage)

---

## Overview

The `apexchainx_calculator` contract emits events in a **predictable,
deterministic order** within each ledger. Backend consumers can rely on
these guarantees for correct event processing, reconciliation, and audit
trails.

Every contract event follows the same structural layout:

| Topic | Description | Example |
|---|---|---|
| `topic[0]` | Event name (Symbol constant) | `sla_calc`, `cfg_upd`, `paused` |
| `topic[1]` | Event version | `v1` (incremented on breaking changes) |
| `topic[2]` | Event-specific context | Severity, caller Address, counter name |

Payload ordering is documented per event variant in `event_schema.rs`.
**Additive changes** (new fields appended to the end) are not considered
breaking; **removals, reorderings, or type changes** require a version bump.

---

## Event Catalog (Canonical Names)

### Calculation Events

| Event | Name | Context (topic[2]) | Emitted When |
|---|---|---|---|
| `sla_calc` | SLA calculation result | Severity | Every successful `calculate_sla` call |
| `set_int` | Settlement intent | Severity | Alongside `sla_calc` for reconciliation |

### Configuration Events

| Event | Name | Context (topic[2]) | Emitted When |
|---|---|---|---|
| `cfg_upd` | Configuration update | Severity | Every successful `set_config` call |
| `cfg_frz` | Configuration frozen | Caller Address | `freeze_config()` succeeds |
| `cfg_unfrz` | Configuration unfrozen | Caller Address | `unfreeze_config()` succeeds |

### Pause / Unpause Events

| Event | Name | Context (topic[2]) | Emitted When |
|---|---|---|---|
| `paused` | Contract paused | Caller Address | `pause()` succeeds |
| `unpause` | Contract unpaused | Caller Address | `unpause()` succeeds |

### Role-Management Events (Admin)

| Event | Name | Context (topic[2]) | Emitted When |
|---|---|---|---|
| `adm_prop` | Admin proposed | Caller Address | `propose_admin()` succeeds |
| `adm_acc` | Admin accepted | Caller Address | `accept_admin()` succeeds |
| `adm_can` | Admin proposal cancelled | Caller Address | `cancel_admin_proposal()` succeeds |
| `adm_ren` | Admin renounced | Caller Address | `renounce_admin()` succeeds |

### Role-Management Events (Operator)

| Event | Name | Context (topic[2]) | Emitted When |
|---|---|---|---|
| `op_prop` | Operator proposed | Caller Address | `propose_operator()` succeeds |
| `op_acc` | Operator accepted | Caller Address | `accept_operator()` succeeds |
| `op_can` | Operator proposal cancelled | Caller Address | `cancel_operator_proposal()` succeeds |
| `op_set` | Operator set directly | Caller Address | `set_operator()` succeeds |

### History & Maintenance Events

| Event | Name | Context (topic[2]) | Emitted When |
|---|---|---|---|
| `pruned` | History pruned (by count) | Caller Address | `prune_history()` removes entries |
| `pruned_a` | History pruned (by age) | Caller Address | `prune_history_by_age()` removes entries |
| `stats_sat` | Stats counter saturated | Counter name | A running-stats counter reaches its bound |
| `migrate_done` | Migration completed | Caller Address | `migrate()` completes successfully |

---

## Ordering Guarantees

The following guarantees are **backed by deterministic tests** in
`apexchainx_calculator/src/event_ordering_tests.rs`. Each guarantee has a
corresponding test that verifies the ordering using Soroban's event API.

### G1 — `cfg_upd` always precedes dependent `sla_calc`/`set_int`

**Test:** `test_cfg_upd_event_precedes_sla_calc`

When an admin updates configuration and an operator immediately calculates
SLA in the same invocation flow, the `cfg_upd` event is emitted **before**
any `sla_calc` or `set_int` events that use the updated configuration.

```
cfg_upd(severity) → sla_calc(severity) → set_int(severity)
```

### G2 — `sla_calc` and `set_int` maintain exact call order

**Test:** `test_sla_calc_and_set_int_emit_in_call_order`

Multiple `calculate_sla` calls within one ledger emit `sla_calc` and
`set_int` events in the **same order** as the calls themselves. Each
`set_int` follows (not necessarily consecutively, but positionally after)
its corresponding `sla_calc`.

```
Call order:  calculate_sla(A) → calculate_sla(B) → calculate_sla(C)
Event order: sla_calc(A) → set_int(A) → sla_calc(B) → set_int(B) → sla_calc(C) → set_int(C)
```

### G3 — `paused` precedes any operation it blocks; `unpause` precedes resumed operations

**Test:** `test_pause_and_unpause_events_in_correct_order`

```
paused → unpause
```

The `paused` event is emitted before the contract enters the paused state.
The `unpause` event is emitted before the contract resumes accepting
`calculate_sla` calls.

### G4 — Each `calculate_sla` call produces exactly 2 events

**Test:** `test_each_calculation_produces_exactly_two_events`

Every successful `calculate_sla` call emits exactly:
- 1 `sla_calc` event (primary result)
- 1 `set_int` event (settlement intent)

No additional events are emitted on a successful calculation path
(beyond these two). If the call is a config-change-driven recalculation,
a `cfg_upd` event may precede them (see G1).

### G5 — Mixed operation sequences preserve cross-operation ordering

**Test:** `test_event_order_in_mixed_operation_sequence`

When config updates and calculations are interleaved, each operation's
events appear in the same relative order as the operations themselves:

```
set_config(severity1) → calculate_sla(A) → set_config(severity2) → calculate_sla(B)
                   ↓                         ↓
cfg_upd(sev1) → sla_calc(A) → set_int(A) → cfg_upd(sev2) → sla_calc(B) → set_int(B)
```

---

## Single-Ledger Multi-Operation Semantics

When **multiple privileged changes** occur within a single ledger (e.g.,
admin proposes a new admin, then pauses the contract, then an operator
calculates SLA), events are emitted in the **exact order** of the
corresponding contract calls.

### Example: Admin Handoff + Pause + Calculation

```
Call sequence:
  propose_admin(admin, new_admin)   → adm_prop event
  pause(admin, "maintenance")       → paused event
  calculate_sla(op, EVT001, crit, 5) → sla_calc + set_int events

Event stream (in order):
  adm_prop → paused → sla_calc → set_int
```

### Example: Config Update + Freeze + Unfreeze

```
Call sequence:
  set_config(admin, critical, 20, 200, 1000) → cfg_upd event
  freeze_config(admin)                       → cfg_frz event
  unfreeze_config(admin)                     → cfg_unfrz event

Event stream (in order):
  cfg_upd → cfg_frz → cfg_unfrz
```

---

## What Is NOT Guaranteed

To avoid implying stronger guarantees than the current test suite
demonstrates, the following are explicitly **non-guarantees**:

| Non-Guarantee | Reason |
|---|---|
| Events from different contracts are interleaved deterministically | This contract does not control other contracts' event emission |
| Events from **failed** calls appear in a specific position | Failed calls do not emit events (validation occurs before writes) |
| `set_int` immediately follows `sla_calc` with no intervening events | Other events (e.g., `cfg_upd` from an interleaved `set_config`) may appear between them |
| Concurrent transactions from different callers are ordered | Soroban ledger ordering is deterministic at the protocol level, but this document only covers single-transaction ordering within this contract |
| Admin transfer lifecycle events follow a fixed pattern for cancelled proposals | A proposal may be cancelled at any point between `adm_prop` and `adm_acc` |

---

## Backend Integration Guidance

### Startup Checklist

1. **Load the event catalog** from `get_result_schema()` and
   `get_failure_schema()` at startup.
2. **Pin the event version** (`v1`) and alert on unexpected versions.
3. **Process events in ledger order** — the guarantees in this document
   apply within a single ledger; across ledgers, use ledger sequence.
4. **Do not assume adjacency** between `sla_calc` and `set_int` — use
   `outage_id` + `config_version_hash` for correlation, not position.

### Correlation Strategy

To correlate `sla_calc` with `set_int`, match on **`outage_id`** —
never assume they are adjacent in the event stream. The `config_version_hash`
can be used as a secondary key to detect config changes between submissions.

### Event Replay

All events emitted by this contract are **deterministic**: replaying the
same ledger with the same state will produce the same events in the same
order. Backends can safely replay events for reconciliation.

---

## Test Coverage

All ordering guarantees in this document are verified by tests in
`apexchainx_calculator/src/event_ordering_tests.rs`:

| Guarantee | Test | Assertions |
|---|---|---|
| G1 (cfg_upd precedes sla_calc) | `test_cfg_upd_event_precedes_sla_calc` | Position of final cfg_upd < position of first sla_calc |
| G2 (call order preserved) | `test_sla_calc_and_set_int_emit_in_call_order` | sla_calc positions are monotonic; set_int follows its sla_calc |
| G3 (pause/unpause order) | `test_pause_and_unpause_events_in_correct_order` | Single paused precedes single unpause |
| G4 (2 events per calc) | `test_each_calculation_produces_exactly_two_events` | Count matches exactly for N calculations |
| G5 (mixed sequence) | `test_event_order_in_mixed_operation_sequence` | Exact named event sequence verified for interleaved ops |

Additional negative tests verify that unauthorized callers cannot produce
events (`test_stranger_cannot_calculate_sla`, `test_stranger_cannot_set_config`,
`test_stranger_cannot_pause`).

---

## References

- [Event Schema Source Code](../apexchainx_calculator/src/event_schema.rs)
- [Event Ordering Tests](../apexchainx_calculator/src/event_ordering_tests.rs)
- [Event Compatibility Policy](./EVENT_COMPATIBILITY_POLICY.md)
- [Event Topic Compatibility](./EVENT_TOPIC_COMPATIBILITY.md)
- [Project Context](./PROJECT_CONTEXT.md)
