# Compatibility Tracking Matrix

> **Audience:** Maintainers, release engineers, and backend integration teams.
> This matrix tracks event-schema drift, storage-version drift, and deployment
> compatibility across the `apexchainx_calculator` contract lifecycle.
>
> **Purpose:** Prevent silent breaking changes by providing a single source of
> truth for version relationships, schema compatibility, and deployment
> constraints.

## Table of Contents

- [Event-Schema Version Matrix](#event-schema-version-matrix)
- [Storage-Version Migration Matrix](#storage-version-migration-matrix)
- [Deployment Compatibility Matrix](#deployment-compatibility-matrix)
- [Cross-Contract Dependency Matrix](#cross-contract-dependency-matrix)
- [Release Compatibility Table](#release-compatibility-table)
- [Change Log](#change-log)

---

## Event-Schema Version Matrix

Tracks the relationship between contract versions and their emitted event
schemas. Each row represents a contract release; columns represent event
variants and their schema version at that release.

| Contract Version | Event Schema Ver | `sla_calc` | `set_int` | `cfg_upd` | `paused` | `unpause` | `op_set` | `pruned` | `pruned_a` | Governance Events¹ | `stats_sat` | `migrate_done` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **v0.1.0** (current) | v1 | v1 | v1 | v1 | v1 | v1 | v1 | v1 | v1 | v1 | v1 | v1 |

> ¹ Governance events: `adm_prop`, `adm_acc`, `adm_can`, `adm_ren`, `op_prop`,
>   `op_acc`, `op_can`, `cfg_frz`, `cfg_unfrz`.

### When to Bump

| Trigger | Action | Example |
|---------|--------|---------|
| New event added (no existing event changed) | No bump — additive | Adding a new `my_new_event` constant |
| Event payload field appended to end | No bump — additive | Adding `new_field: u32` at end of `sla_calc` payload |
| Event payload field removed, reordered, or type-changed | **Bump to v2** | Changing `u32` → `i128` in `paused` payload |
| Event topic name changed | **Bump to v2** | Renaming `sla_calc` → `sla_calc_v2` |
| Event topic index layout changed | **Bump to v2** | Adding a 4th topic |
| Symbol deprecation lifecycle started | See [Symbol Deprecation Protocol](event_schema.rs#L120-L160) | Deprecating `"viol"` → `"violated"` |

### Schema Version History

| Schema Version | Contract Version | Changes | Date |
|----------------|-----------------|---------|------|
| v1 | v0.1.0 | Initial event schema | 2026-07-23 |

---

## Storage-Version Migration Matrix

Tracks the on-chain storage version (`STORAGE_VERSION` constant in `lib.rs`)
and the migration paths between versions.

| Storage Ver | Contract Ver | Breaking Changes | Migration Function | Backward Compat? | Forward Compat? |
|---|---|---|---|---|---|
| **1** | v0.1.0 | Initial storage layout | `initialize()` | N/A | N/A |

### Migration Paths

```mermaid
graph LR
    V1[Version 1] -->|migrate()| V2[Version 2]
    V2 -->|migrate()| V3[Version 3]
    V1 -.->|direct v1→v3| migrate_v3[future]
```

**Legend:** Solid lines = existing migration path. Dashed lines = planned but
not yet implemented.

### Version Negotiation Contract

The `get_version_info()` endpoint returns the current storage version and
migration posture. Backends MUST call this before issuing operational
transactions.

| Field | Current Value | Notes |
|-------|--------------|-------|
| `storage_version` | 1 | Value from `STORAGE_VERSION_KEY` |
| `result_schema_version` | 1 | Value from `RESULT_SCHEMA_VERSION` |
| `needs_migration` | `false` | `true` when storage ≠ expected |
| `is_paused` | varies | Runtime-dependent |
| `contract_name` | `"sla_calc"` | Fixed identifier |

### When to Increment `STORAGE_VERSION`

| Change Type | Requires Increment? | Migration Required? |
|---|---|---|
| Adding a new storage key | **Yes**, if existing data is reinterpreted | Usually no (new key is empty) |
| Adding a new storage key | **No**, if it's truly additive (new feature, no existing data change) | No |
| Changing serialisation format of an existing value | **Yes** | Yes — `migrate()` must rewrite values |
| Removing a storage key | **Yes** | Yes — `migrate()` must clean up stale entries |
| Changing the meaning of an existing stored value | **Yes** | Yes — `migrate()` must transform values |

---

## Deployment Compatibility Matrix

Describes which contract versions can be safely deployed alongside which
backend versions.

### Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Fully compatible |
| ⚠️ | Compatible with caveats (see notes) |
| ❌ | Incompatible — must upgrade one side |

### Contract vs Backend Compatibility

| Contract Version | Backend v0.1.x | Notes |
|---|---|---|
| **v0.1.0** | ✅ | Initial release; all events, storage, and APIs match |

### Contract vs Frontend Compatibility

| Contract Version | Frontend v0.1.x | Notes |
|---|---|---|
| **v0.1.0** | ✅ | Initial release |

### Rollback Safety

| Scenario | Safe? | Notes |
|----------|-------|-------|
| Upgrade from v0.1.0 to v0.2.0 | ✅ | Forward-compatible if storage version matches |
| Rollback from v0.2.0 to v0.1.0 | ⚠️ | Only safe if v0.2.0 did not write new storage keys or change existing serialisation |
| Rollback after migration | ❌ | `migrate()` may rewrite data incompatibly; rollback requires a restore |

---

## Cross-Contract Dependency Matrix

Tracks compatibility between `apexchainx_calculator` and other contracts in the
ApexChainx ecosystem.

| Contract | Current Version | Compatible With | Notes |
|---|---|---|---|
| `apexchainx_calculator` | v0.1.0 | Self | N/A |
| *(Future contracts TBD)* | — | — | This section will be populated as new contracts are added |

---

## Release Compatibility Table

A chronological log of release compatibility assertions. Each row documents
what was verified at release time.

| Release | Date | Storage Ver | Event Schema Ver | Backend Compat | Frontend Compat | Verified By |
|---|---|---|---|---|---|---|
| v0.1.0 | 2026-07-23 | 1 | v1 | ✅ v0.1.x | ✅ v0.1.x | CI |

### How to Add a New Release

1. Add a row to the [Event-Schema Version Matrix](#event-schema-version-matrix)
2. Add a row to the [Storage-Version Migration Matrix](#storage-version-migration-matrix)
3. Update the [Deployment Compatibility Matrix](#deployment-compatibility-matrix)
4. Add a row to this [Release Compatibility Table](#release-compatibility-table)
5. Update `CHANGELOG.md` with the breaking/additive changes
6. Generate a release summary: `just release-summary <version>`

---

## Change Log

| Date | Author | Change | Rationale |
|---|---|---|---|
| 2026-07-30 | — | Initial matrix created | Issue #223 — tracking event-schema drift, storage-version drift, and deployment compatibility |

---

## Related Documents

- [RESERVED_KEYS_POLICY.md](RESERVED_KEYS_POLICY.md) — Reserved key prefix conventions
- [CONTRACT_MAINTENANCE_POLICY.md](CONTRACT_MAINTENANCE_POLICY.md) — SC-500–SC-508 maintenance policies
- [EVENT_TOPIC_COMPATIBILITY.md](EVENT_TOPIC_COMPATIBILITY.md) — Event topic compatibility policy
- [CONTRACT_API_COMPATIBILITY.md](CONTRACT_API_COMPATIBILITY.md) — API compatibility assertions
- [CONTRIBUTING.md](../CONTRIBUTING.md) — Contributor guide and pre-merge checklists
