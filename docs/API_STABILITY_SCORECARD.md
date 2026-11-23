# API Stability Scorecard

> **Issue:** [#248](https://github.com/ApexChainx/ApexChainx-Contracts/issues/248)
> **Status:** Active
> **Applies to:** All public contract entrypoints in `apexchainx_calculator`

This document classifies every public contract entrypoint by compatibility risk
so maintainers and contributors can quickly assess whether a proposed change is
safe (additive) or dangerous (breaking).

---

## Table of Contents

- [1. Stability Tiers](#1-stability-tiers)
- [2. Entrypoint Scorecard](#2-entrypoint-scorecard)
- [3. Change Impact Guide](#3-change-impact-guide)
- [4. PR Review Checklist for API Changes](#4-pr-review-checklist-for-api-changes)
- [5. Version Bump Rules](#5-version-bump-rules)

---

## 1. Stability Tiers

| Tier | Meaning | Breaking Change Test |
|------|---------|---------------------|
| **🔒 Frozen** | Signature must never change. Any modification is a MAJOR breaking change. | Changing the function name, parameter count, parameter types, or return type breaks all backends. |
| **⚠️ Stable** | Signature is stable but additive changes (new optional fields at the end of return types) are allowed. | Changing existing parameter order/type or removing return fields is breaking. Appending new return fields is safe. |
| **🔄 Evolving** | May change with a documented migration path. Backends must check `get_version_info()` before consuming. | Changes require a `STORAGE_VERSION` or `RESULT_SCHEMA_VERSION` bump. |
| **🛡️ Admin-Gated** | Signature is stable but the function is restricted to the admin role. Changes affect only the admin workflow. | Same rules as Stable, but changes are scoped to admin tooling (not backend bridge). |

---

## 2. Entrypoint Scorecard

### 2.1 Initialisation & Migration

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `initialize` | 🔒 Frozen | Public (one-time) | **Extreme** | Called once at deploy. Changing signature means new deployments are incompatible with existing backend bootstrap scripts. |
| `migrate` | 🔄 Evolving | Admin | **High** | New migration arms are additive (safe). Removing an arm or changing the step order is breaking. Must bump `STORAGE_VERSION`. |

### 2.2 Core SLA Computation

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `calculate_sla` | 🔒 Frozen | Operator | **Extreme** | Primary business-logic entrypoint. Backends, tests, and audit replay all depend on its signature. Any change to the `SLAResult` return type must bump `RESULT_SCHEMA_VERSION`. |
| `calculate_sla_view` | 🔒 Frozen | Public | **Extreme** | Must produce **identical** results to `calculate_sla` for the same inputs. Backend parity tests enforce this. Signature must match `calculate_sla`. |
| `replay_calculate_sla` | ⚠️ Stable | Public | **High** | Used for backend reconciliation. Changing return shape requires `RESULT_SCHEMA_VERSION` bump. |

### 2.3 Configuration Management

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `set_config` | 🔒 Frozen | Admin | **High** | Core admin workflow. Parameter order is depended on by admin tooling. |
| `get_config` | ⚠️ Stable | Public | **Medium** | Backend consumers cache config. Return type `SLAConfig` changes are breaking. |
| `get_config_snapshot` | ⚠️ Stable | Public | **Medium** | Ordered export consumed by backends. Changing entry order breaks downstream. |
| `get_config_version_hash` | ⚠️ Stable | Public | **Low** | Return type `u64` is stable. Hash algorithm change is additive if versioned. |
| `get_last_config_update` | ⚠️ Stable | Public | **Low** | Return type `Option<ConfigUpdateInfo>`. Wrapper struct change is breaking. |
| `list_configs` | ⚠️ Stable | Public | **Low** | Return type `Map<Symbol, SLAConfig>`. Map key changes are breaking. |
| `get_config_count` | ⚠️ Stable | Public | **Low** | Return type `u32`. Adding new config categories is additive. |
| `get_config_bundle` | ⚠️ Stable | Public | **Medium** | Composes `get_config_snapshot` + `get_result_schema`. Changes cascade. |

### 2.4 Custom Severities

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `set_custom_severity` | ⚠️ Stable | Admin | **Medium** | Admin-gated. Parameter changes break admin tooling. |
| `remove_custom_severity` | ⚠️ Stable | Admin | **Medium** | Admin-gated. Signature is stable. |
| `get_custom_severity` | ⚠️ Stable | Public | **Low** | Read-only. Return type is `SLAConfig`. |
| `get_custom_config_snapshot` | ⚠️ Stable | Public | **Low** | Read-only. Mirrors `get_config_snapshot` shape. |

### 2.5 Governance & Role Management

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `get_admin` | 🔒 Frozen | Public | **Low** | Return type `Address`. Stable. |
| `get_operator` | 🔒 Frozen | Public | **Low** | Return type `Address`. Stable. |
| `set_operator` | 🛡️ Admin-Gated | Admin | **Low** | Admin tooling only. |
| `propose_admin` | 🛡️ Admin-Gated | Admin | **Low** | Two-step transfer step 1. |
| `accept_admin` | 🛡️ Admin-Gated | Pending Admin | **Low** | Two-step transfer step 2. |
| `cancel_admin_proposal` | 🛡️ Admin-Gated | Admin | **Low** | Admin recovery action. |
| `get_pending_admin` | 🛡️ Admin-Gated | Public | **Low** | Return type `Option<Address>`. |
| `propose_operator` | 🛡️ Admin-Gated | Admin | **Low** | Two-step handoff step 1. |
| `accept_operator` | 🛡️ Admin-Gated | Pending Op | **Low** | Two-step handoff step 2. |
| `cancel_operator_proposal` | 🛡️ Admin-Gated | Admin | **Low** | Admin recovery action. |
| `get_pending_operator` | 🛡️ Admin-Gated | Public | **Low** | Return type `Option<Address>`. |
| `renounce_admin` | 🔒 Frozen | Admin | **High** | **Irreversible.** Adding guardrails is additive; removing is not. |

### 2.6 Pause / Unpause

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `pause` | 🔒 Frozen | Admin | **Medium** | Parameter `reason: String` is depended on by monitoring. Changing max length affects `MAX_REASON_LEN`. |
| `unpause` | 🔒 Frozen | Admin | **Low** | Simple signature. |
| `is_paused` | 🔒 Frozen | Public | **Low** | Return type `bool`. |
| `get_pause_info` | ⚠️ Stable | Public | **Low** | Return type `Option<PauseInfo>`. Adding fields to `PauseInfo` is additive. |

### 2.7 Config Freeze

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `freeze_config` | 🛡️ Admin-Gated | Admin | **Low** | Admin tooling only. |
| `unfreeze_config` | 🛡️ Admin-Gated | Admin | **Low** | Admin tooling only. |
| `is_config_frozen` | ⚠️ Stable | Public | **Low** | Return type `bool`. |

### 2.8 History & Pruning

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `get_history` | ⚠️ Stable | Public | **Medium** | Return type `Vec<SLAResult>`. `SLAResult` changes are breaking. |
| `get_history_page` | ⚠️ Stable | Public | **Medium** | Pagination parameters must remain `(offset: u32, limit: u32)`. |
| `get_history_page_with_meta` | ⚠️ Stable | Public | **Medium** | Return type `HistoryPage`. Pagination parameters must remain `(offset: u32, limit: u32)`. |
| `get_history_by_outage` | ⚠️ Stable | Public | **Low** | Filtered read. Return type follows `SLAResult`. |
| `get_latest_by_outage` | ⚠️ Stable | Public | **Low** | Return type `Option<SLAResult>`. |
| `prune_history` | 🛡️ Admin-Gated | Admin | **Medium** | Parameter `keep_latest: u32` must stay. |
| `prune_history_by_age` | 🛡️ Admin-Gated | Admin | **Medium** | Parameter `min_age_seconds: u64` must stay. |

### 2.9 Statistics & Telemetry

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `get_stats` | ⚠️ Stable | Public | **Medium** | Return type `SLAStats`. Adding fields to `SLAStats` is additive. |
| `get_economic_exposure` | ⚠️ Stable | Public | **Medium** | Return type `EconomicExposure`. Dashboard consumer. |
| `get_severity_telemetry` | ⚠️ Stable | Public | **Medium** | Return type `Vec<SeverityTelemetry>`. Weekly rollup consumer. |

### 2.10 Version & Metadata Views

| Entrypoint | Tier | Auth | Breaking Risk | Notes |
|-----------|------|------|--------------|-------|
| `get_version_info` | 🔒 Frozen | Public | **Medium** | **Critical** — backend startup handshake. Return type `VersionInfo` changes break all backends simultaneously. |
| `get_storage_version` | ⚠️ Stable | Public | **Low** | Return type `u32`. |
| `get_result_schema` | ⚠️ Stable | Public | **Medium** | Return type `SLAResultSchema`. Deprecated symbols are additive. |
| `get_failure_schema` | ⚠️ Stable | Public | **Low** | Return type `FailureSchema`. New error codes are additive. |
| `get_contract_metadata` | ⚠️ Stable | Public | **Low** | Return type `ContractMetadata`. Adding features is additive. |
| `get_full_audit_state` | ⚠️ Stable | Public | **Medium** | Composes multiple views. Changes cascade. |

---

## 3. Change Impact Guide

### Additive (Safe)

| Change | Example | Version Bump? |
|--------|---------|--------------|
| New read-only view function | `get_foo()` | No |
| New field appended to return struct | Adding `new_field` to end of `SLAResult` | `RESULT_SCHEMA_VERSION` +1 |
| New error code appended to `SLAError` | Adding `NewError = 20` | No (additive per schema policy) |
| New feature flag in `ContractMetadata.features` | `features.push("new_feat")` | No |
| New event constant with unique name | `EVENT_NEW_THING` | No |

### Breaking (Dangerous)

| Change | Example | Version Bump? |
|--------|---------|--------------|
| Removing a public function | Delete `get_foo()` | MAJOR — coordinate with all consumers |
| Changing parameter count or order | `fn foo(a, b)` → `fn foo(a)` | MAJOR |
| Changing parameter type | `mttr_minutes: u32` → `mttr_minutes: u64` | MAJOR |
| Changing return type | `Result<A, E>` → `Result<B, E>` | `RESULT_SCHEMA_VERSION` +1 |
| Removing a field from a return struct | Drop `rating` from `SLAResult` | MAJOR |
| Reordering fields in a return struct | Swap field positions | MAJOR (even though types are same) |
| Renaming a public function | `get_foo()` → `fetch_foo()` | MAJOR |
| Changing auth gate on an entrypoint | Public → Admin | MAJOR (breaks existing callers) |
| Incrementing `STORAGE_VERSION` | v1 → v2 | Requires migration arm in `migrate()` |

---

## 4. PR Review Checklist for API Changes

When reviewing a PR that modifies any public contract entrypoint, confirm:

- [ ] **Tier check:** What stability tier is the affected function? (Consult §2)
- [ ] **Additive vs Breaking:** Is this an additive change (new field at end, new standalone function) or a breaking change (parameter change, removal, reorder)?
- [ ] **Version bump:** If breaking, is `STORAGE_VERSION` or `RESULT_SCHEMA_VERSION` incremented?
- [ ] **Migration path:** If `STORAGE_VERSION` changed, is there a corresponding arm in `migrate()`?
- [ ] **Schema update:** If `SLAResult` or return types changed, is `get_result_schema()` updated?
- [ ] **Backend parity:** Will `calculate_sla_view` still produce the same results for the same inputs?
- [ ] **Event compatibility:** Do event payloads follow append-only field ordering? (See `docs/EVENT_COMPATIBILITY_POLICY.md`)
- [ ] **Tests:** Are there tests covering both old and new behaviour?

---

## 5. Version Bump Rules

| Scenario | What to Bump | Migration Required? |
|----------|-------------|---------------------|
| New storage key introduced | `STORAGE_VERSION` +1 | Yes — add arm in `migrate()` |
| `SLAResult` field added at end | `RESULT_SCHEMA_VERSION` +1 | No |
| `SLAResult` field removed/reordered | `RESULT_SCHEMA_VERSION` +1 | No (but coordinate with all consumers) |
| New public read-only function | Nothing | No |
| New error variant in `SLAError` | Nothing | No |
| Event payload field added at end | Nothing (additive) | No |
| Event payload field removed/reordered | `EVENT_VERSION` bump | No (but coordinate with all consumers) |
| Multi-crate protocol change | `PROTOCOL_VERSION` in `version_negotiation.rs` | Coordinate with `apexchainx-be` |

### Related Documents

- [Contract API Compatibility](CONTRACT_API_COMPATIBILITY.md) — Backend adapter verification suite
- [Event Compatibility Policy](EVENT_COMPATIBILITY_POLICY.md) — Event schema immutability rules
- [Event Topic Compatibility](EVENT_TOPIC_COMPATIBILITY.md) — Topic symbol deprecation lifecycle
- [Upgrade Playbook](UPGRADE_PLAYBOOK.md) — Full storage-version upgrade procedures
- [Reserved Keys Policy](RESERVED_KEYS_POLICY.md) — Storage key prefix reservations
