# Severity Compatibility Matrix

> **Status:** Maintained compatibility reference for release planning  
> **References:** Issue #254  
> **Last updated:** 2026-07-29  
> **Validation logic:** `apexchainx_calculator/src/config.rs`

## Table of Contents

- [Overview](#overview)
- [Severity Families](#severity-families)
- [Canonical Severity Matrix](#canonical-severity-matrix)
- [Custom Severity Matrix](#custom-severity-matrix)
- [Cross-Family Compatibility](#cross-family-compatibility)
- [Change Classification](#change-classification)
- [Release Planning Checklist](#release-planning-checklist)
- [Backend Integration Impact](#backend-integration-impact)

---

## Overview

The `apexchainx_calculator` contract manages configuration for **two
severity families** stored in separate on-chain maps:

| Family | Storage Key | Map Type | Number of Entries |
|---|---|---|---|
| **Canonical** | `CONFIG` | `Map<Symbol, SLAConfig>` | Exactly 4 (fixed) |
| **Custom** | `CUSTCFG` | `Map<Symbol, SLAConfig>` | 0–N (admin-managed) |

The canonical family is **immutable in structure** (severities cannot be
added or removed), while the custom family supports **dynamic registration
and removal**.

Understanding how changes to one severity family affect the other is
critical for release planning and contract upgrades.

---

## Severity Families

### Canonical Severities (Fixed Set)

The four canonical severities form a **fixed, ordered set** used by:

- `get_config_snapshot()` — always returns these 4 in order
- `compute_config_version_hash()` — includes only these 4
- `get_economic_exposure()` — iterates only these 4
- `get_severity_telemetry()` — reports per-severity stats for these 4
- `validate_cross_severity_penalty_ordering()` — enforces ordering among
  these 4

**Canonical order (immutable):** `critical` → `high` → `medium` → `low`

### Custom Severities (Dynamic Set)

Custom severities are registered via `set_custom_severity()` and removed
via `remove_custom_severity()`. They are stored in a **separate map**
(`CUSTCFG`) and are:

- **Excluded** from `get_config_snapshot()` — returned separately via
  `get_custom_config_snapshot()`
- **Excluded** from `compute_config_version_hash()` — custom changes do not
  affect the canonical config hash
- **Excluded** from cross-severity penalty ordering validation
- **Excluded** from `get_economic_exposure()`
- **Excluded** from `get_severity_telemetry()`
- Subject to **general bounds only** (no severity-specific limits)

---

## Canonical Severity Matrix

| Property | `critical` | `high` | `medium` | `low` |
|---|---|---|---|---|
| **Max threshold** | 60 min | 120 min | 240 min | 1,440 min |
| **Min penalty/min** | 50 | 25 | 10 | (no floor, max 100) |
| **Default threshold** | 15 min | 30 min | 60 min | 120 min |
| **Default penalty/min** | 100 | 50 | 25 | 10 |
| **Default reward base** | 750 | 750 | 750 | 600 |
| **Cross-severity rule** | penalty ≥ high | penalty ≥ medium | penalty ≥ low | (lowest — no lower bound) |
| **Config hash** | Included | Included | Included | Included |
| **Config snapshot** | Included | Included | Included | Included |
| **Economic exposure** | Included | Included | Included | Included |
| **Severity telemetry** | Included | Included | Included | Included |

### Canonical Additive Changes

The following changes to canonical severities are **additive** (backward
compatible, no version bump required):

| Change | Impact |
|---|---|
| Updating `threshold_minutes` within severity-specific bounds | No schema change; affects future `calculate_sla` outcomes |
| Updating `penalty_per_minute` within severity-specific bounds | No schema change; must maintain cross-severity ordering |
| Updating `reward_base` within general bounds | No schema change; must maintain `penalty × 1.5 < reward` |
| No-op writes (same values) | No config hash change; emits `cfg_upd` event |

### Canonical Breaking Changes

The following changes are **breaking** and require a contract upgrade:

| Change | Impact | Mitigation |
|---|---|---|
| Adding a new canonical severity | Changes `canonical_severities()` length; breaks snapshot consumers | Add as custom severity instead |
| Removing a canonical severity | Breaks all consumers expecting 4 entries | Deprecate before removal |
| Changing the canonical order | Breaks config version hash computation | Requires `STORAGE_VERSION` bump |
| Changing severity-specific bounds | Older clients may submit valid values rejected by new contract | Document in release notes |
| Removing cross-severity penalty ordering | Allows penalty inversions; breaks economic assumptions | Requires governance approval |

---

## Custom Severity Matrix

| Property | Custom Severities |
|---|---|
| **Storage location** | `CUSTCFG` (separate from canonical `CONFIG`) |
| **Registration** | `set_custom_severity(admin, name, threshold, penalty, reward)` |
| **Removal** | `remove_custom_severity(admin, name)` |
| **Query** | `get_custom_severity(name)` |
| **Snapshot** | `get_custom_config_snapshot()` — returns all registered |
| **Validation** | General bounds only (1–1440 threshold, 1–10000 penalty, 1–100000 reward) |
| **Cross-severity rules** | Not enforced against canonical or other custom severities |
| **Config hash** | Not included |
| **Economic exposure** | Not included |
| **Severity telemetry** | Not included |
| **Canonical shadowing** | Rejected — custom name must not match a canonical severity |

### Custom Additive Changes

| Change | Impact |
|---|---|
| Registering a new custom severity | Only general bounds enforced; no effect on canonical functions |
| Updating a custom severity's config | Same as registration path; overwrites existing parameters in-place |
| Reading custom snapshot | Pure view; no state mutation |

### Custom Severity Lifecycle & Overwrite

- **Registration & Overwrite**: Calling `set_custom_severity(admin, name, ...)` with a new custom severity registers it in `CUSTCFG`. Calling `set_custom_severity` with an already-registered custom severity name updates and overwrites its parameters (`threshold_minutes`, `penalty_per_minute`, `reward_base`) in-place.
- **Event Emission**: Both initial registration and subsequent updates emit the canonical `cfg_upd` (`EVENT_CONFIG_UPD`) event carrying the custom severity symbol and new parameters. Backends should treat `cfg_upd` events for custom severities as upserts.

### Custom Breaking Changes

| Change | Impact | Mitigation |
|---|---|---|
| Removing a custom severity that backends depend on | `calculate_sla` with that severity returns `SeverityNotInSet` | Notify backend consumers before removal |
| Changing `set_custom_severity` validation rules | Previously valid registrations may be rejected after upgrade | Document new bounds |
| Adding cross-severity ordering for custom severities | Existing custom configs may suddenly violate ordering | Migration step required |

---

## Cross-Family Compatibility

### What Does NOT Cross-Contaminate

| Operation | Canonical | Custom |
|---|---|---|
| `set_config(critical, ...)` | ✅ Updates canonical map | ❌ No effect on custom map |
| `set_custom_severity(my_sev, ...)` | ❌ No effect on canonical map | ✅ Updates custom map |
| `get_config_snapshot()` | ✅ Returns canonical only | ❌ Excludes custom |
| `get_custom_config_snapshot()` | ❌ Excludes canonical | ✅ Returns custom only |
| `compute_config_version_hash()` | ✅ Uses canonical only | ❌ Ignores custom |
| `validate_cross_severity_penalty_ordering()` | ✅ Checks canonical only | ❌ Ignores custom |
| `calculate_sla(severity)` | ✅ Works for canonical names | ✅ Works for registered custom names |
| `calculate_sla_view(severity)` | ✅ Works for canonical names | ✅ Works for registered custom names |
| `get_economic_exposure()` | ✅ Returns canonical only | ❌ Excludes custom |
| `get_severity_telemetry()` | ✅ Returns canonical only | ❌ Excludes custom |

### Key Principle

**Canonical and custom severities are completely isolated in storage,
snapshot generation, hash computation, and telemetry.** The only shared
path is `calculate_sla` / `calculate_sla_view`, which looks up a severity
in both maps (canonical first, then custom).

### Slot Reservation

Custom severity names **must never collide** with canonical names.
`set_custom_severity` rejects any name that is a canonical severity
(`critical`, `high`, `medium`, `low`). This prevents shadowing attacks.

---

## Change Classification

Use this table during release planning to classify planned changes:

| Change | Classification | Version Bump | Backend Impact |
|---|---|---|---|
| Update a canonical severity's threshold/penalty/reward via `set_config` | **Additive** (admin action) | None | Backend re-fetches config if using `get_last_config_update` |
| Register a new custom severity | **Additive** (admin action) | None | None — custom severities are opt-in by backend |
| Remove a custom severity | **Breaking** (if backend depends on it) | None | Backend must stop referencing removed severity |
| Add a new field to `SLAConfig` | **Additive** (if appended) | None | Old consumers ignore trailing fields |
| Remove/reorder `SLAConfig` fields | **Breaking** | `RESULT_SCHEMA_VERSION` bump | Backend must update deserialization |
| Change canonical severity order | **Breaking** | `STORAGE_VERSION` bump, migration required | Backend must re-fetch config snapshot |
| Tighten severity-specific bounds | **Potentially breaking** | None (no storage schema change) | Previously valid configs may be rejected |
| Add a new canonical severity | **Breaking** | `STORAGE_VERSION` bump | All snapshot consumers break |

---

## Release Planning Checklist

When planning a release that touches severity configuration:

- [ ] Review this matrix for planned changes
- [ ] Classify each change as additive or breaking
- [ ] If breaking: plan `STORAGE_VERSION` bump and migration arm in `migrate()`
- [ ] If adding bounds: check existing on-chain configs against new bounds
- [ ] If removing a custom severity: notify backend consumers
- [ ] Update `config-validation.md` if validation rules change
- [ ] Run `cargo test -p apexchainx_calculator` — all config tests must pass
- [ ] Verify `compute_config_version_hash()` output is stable for unchanged configs
- [ ] Verify cross-severity penalty ordering test still validates all pairs

---

## Backend Integration Impact

### Snapshot Consumers

Backends consuming `get_config_snapshot()` receive **only** the four
canonical severities in canonical order. Custom severities must be fetched
separately via `get_custom_config_snapshot()`.

### Config Hash Consumers

Backends using `config_version_hash` for cache invalidation do **not** need
to re-fetch when custom severities change — the hash only reflects canonical
configuration.

### Event Consumers

Both canonical and custom severity updates emit the same `cfg_upd` event.
Backends must differentiate by the `topic[2]` severity symbol:
- Canonical names: `critical`, `high`, `medium`, `low`
- Custom names: any other Symbol

---

## References

- [Configuration Validation Rules](./config-validation.md)
- [Config Source Code](../apexchainx_calculator/src/config.rs)
- [CONTRACT_API_COMPATIBILITY.md](./CONTRACT_API_COMPATIBILITY.md)
- [CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md](./CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md)
- [Project Context](./PROJECT_CONTEXT.md)
