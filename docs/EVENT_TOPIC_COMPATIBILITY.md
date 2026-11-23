# Formal Offline Policy: Contract Event-Topic Compatibility & Symbol Deprecation Tracking

> **Version:** 1.0.0  
> **Last Updated:** 2026-07-29  
> **Applies To:** `apexchainx_calculator` Soroban smart contract  
> **References:** Issue #193, SC-W5-041, SC-W5-043

---

## 1. Purpose

This document defines the **formal offline policy** for how the `apexchainx_calculator` contract emits events, how backend indexers should consume them, and how symbol deprecation is tracked across contract upgrades. Adherence to this policy ensures backend consumers never break when the contract is upgraded.

---

## 2. Event Structure Contract

### 2.1 Three-Topic Layout

Every contract event follows a rigid 3-topic layout:

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| `topic[0]` | Event Name | `Symbol` | Identifies the event type (e.g. `"sla_calc"`, `"cfg_upd"`) |
| `topic[1]` | Event Version | `Symbol` | Canonical event schema version (always `"v1"` for current events) |
| `topic[2]` | Context | `Symbol` or `Address` | Event-specific qualifier (severity, caller address, etc.) |

### 2.2 Payload Stability

- **Field ordering is fixed** — fields MUST NOT be reordered without a version bump.
- **Type changes are breaking** — changing a field's type (e.g. `u32` → `u64`) requires a version bump.
- **Additive fields are safe** — appending new fields at the end of the payload does NOT require a version bump, provided old consumers ignore trailing fields.
- **Field removal is breaking** — removing a field requires a version bump.

---

## 3. Event Catalog

### 3.1 Operational Events

| Event Name | Topic[2] Context | Payload | Emitted By |
|------------|------------------|---------|------------|
| `sla_calc` | `severity: Symbol` | `(outage_id, status, payment_type, rating, mttr_minutes, threshold_minutes, amount)` | `calculate_sla` |
| `set_int` | `severity: Symbol` | `(outage_id, status, payment_type, amount, config_version_hash, recorded_at)` | `calculate_sla` (alongside `sla_calc`) |
| `cfg_upd` | `severity: Symbol` | `(threshold_minutes, penalty_per_minute, reward_base)` | `set_config` |
| `paused` | `caller: Address` | `(true,)` | `pause` |
| `unpause` | `caller: Address` | `(false,)` | `unpause` |
| `op_set` | `caller: Address` | `(new_operator,)` | `set_operator` |

### 3.2 Governance Events

| Event Name | Topic[2] Context | Payload | Emitted By |
|------------|------------------|---------|------------|
| `adm_prop` | `caller: Address` | `(new_admin,)` | `propose_admin` |
| `adm_acc` | `caller: Address` | `()` | `accept_admin` |
| `adm_can` | `caller: Address` | `()` | `cancel_admin_proposal` |
| `adm_ren` | `caller: Address` | `()` | `renounce_admin` |
| `op_prop` | `caller: Address` | `(new_operator,)` | `propose_operator` |
| `op_acc` | `caller: Address` | `()` | `accept_operator` |
| `op_can` | `caller: Address` | `()` | `cancel_operator_proposal` |

### 3.3 Maintenance Events

| Event Name | Topic[2] Context | Payload | Emitted By |
|------------|------------------|---------|------------|
| `pruned` | `caller: Address` | `(removed_count, kept_count)` | `prune_history` |
| `pruned_a` | `caller: Address` | `(removed_count, kept_count)` | `prune_history_by_age` |
| `cfg_frz` | `caller: Address` | `()` | `freeze_config` |
| `cfg_unfrz` | `caller: Address` | `()` | `unfreeze_config` |
| `stats_sat` | `counter_name: Symbol` | `(field, previous_value, attempted_increment)` | `increment_stats` (on saturation) |
| `migrate_done` | `caller: Address` | `(old_version, new_version)` | `migrate` |

---

## 4. Versioning Policy

### 4.1 Event Version Bumps

The `topic[1]` version symbol (`"v1"`, `"v2"`, etc.) MUST be incremented when any of the following occur:

1. A field is **removed** from the payload
2. A field's **type changes** (e.g. `u32` → `u64`)
3. Fields are **reordered**

The version symbol is **not** incremented when:

1. New fields are **appended** to the end of the payload
2. Internal logic changes without affecting event structure
3. The event name remains unchanged

### 4.2 Storage Version Independence

- The event version (`topic[1]`) is **independent** of the storage schema version (`STORAGE_VERSION`).
- A storage migration (`migrate`) does NOT automatically bump event versions.
- Event versions are bumped only when the event payload schema changes.

---

## 5. Symbol Deprecation Protocol

### 5.1 Lifecycle

When a result or severity symbol needs to change, follow this 3-phase lifecycle:

#### Phase 1: Introduction (Minor Release)

- Add the new symbol **alongside** the old one.
- Both symbols are emitted in events.
- `get_result_schema()` returns a `deprecated_symbols` entry marking the old symbol as deprecated.

```json
{
  "status_met": "met",
  "status_violated": "violated",
  "deprecated_symbols": [
    {
      "old_symbol": "viol",
      "new_symbol": "violated",
      "deprecated_at": 2,
      "removal_version": null
    }
  ]
}
```

#### Phase 2: Coexistence (At Least One Minor Release)

- The old symbol **continues to be emitted** alongside the new one.
- Backend consumers migrate at their own pace.
- The `deprecated_symbols` entry maintains `removal_version: null`.

#### Phase 3: Removal (Major Release)

- The old symbol is **removed** from event emission.
- `schema_version` in `get_result_schema()` is bumped.
- The `deprecated_symbols` entry is updated with `removal_version` set to the schema version at which removal occurred.

### 5.2 Backend Responsibilities

Backends MUST:

1. Call `get_result_schema()` at startup.
2. Check `deprecated_symbols` for any symbols they still consume.
3. Log **warnings** for deprecated symbols still in use.
4. Log **errors** for symbols with `removal_version` set (the old symbol is gone).

### 5.3 Current Deprecated Symbols

As of `RESULT_SCHEMA_VERSION = 1`, there are no deprecated symbols.

---

## 6. Backend Integration Guide

### 6.1 Startup Sequence

```python
# Recommended backend startup sequence
contract_info = client.get_contract_info()           # #191 – version posture
result_schema = client.get_result_schema()            # symbol mappings + deprecations
failure_schema = client.get_failure_schema()          # error code catalogue
config_bundle = client.get_config_bundle()            # severity configs + schema
```

### 6.2 Event Consumption

```python
# Event structure is guaranteed to be:
#   topics: [event_name, event_version, context]
#   data: tuple of typed fields

def handle_sla_calc(topics, data):
    assert topics[1] == "v1"  # verify version
    severity = topics[2]       # context
    outage_id, status, payment_type, rating, mttr, threshold, amount = data
    # ... process event
```

### 6.3 Detecting Breaking Changes

- Monitor `get_contract_info().event_version` — if it changes, re-read `get_result_schema()`.
- Monitor `get_contract_info().result_schema_version` — if it bumps, check `deprecated_symbols`.
- Subscribe to `cfg_upd` events — they signal config changes.

---

## 7. Stability Guarantees

| Guarantee | Scope |
|-----------|-------|
| 3-topic layout | **Permanent** — all events use this structure |
| `topic[1]` = `"v1"` | **Stable** — bumped only on breaking changes |
| Event name uniqueness | **Guaranteed** — verified by `event_schema.rs` tests |
| Payload field ordering | **Stable** — changes require version bumps |
| Additive field safety | **Guaranteed** — new fields go at the end |

---

## 8. Testing & Validation

The contract tests in `apexchainx_calculator/src/topic_stability_tests.rs` verify:

- All events have exactly 3 topics
- `topic[1]` is always the current event version
- Event names match their expected constants
- Event contexts match expected values (severity, caller address)
- Topic structures remain stable across repeated operations

Run: `cargo test --package apexchainx_calculator -- topic_stability`

---

## 9. Change Log

| Date | Version | Description |
|------|---------|-------------|
| 2026-07-29 | 1.0.0 | Initial formal policy; codifies existing event conventions from SC-W5-041 and SC-W5-043 |
