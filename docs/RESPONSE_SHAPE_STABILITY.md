# Response-Shape Stability Policy

> **Version:** 1.0.0
> **Last Updated:** 2026-07-30
> **Applies To:** `apexchainx_calculator` Soroban smart contract
> **References:** `get_result_schema()`, `get_failure_schema()`, `get_version_info()`, `RESULT_SCHEMA_VERSION`

---

## 1. Purpose

This document defines the **response-shape stability policy** for all typed return structures exposed by the `apexchainx_calculator` contract. Backend maintainers rely on these guarantees to avoid integration breaks when the contract is upgraded.

The term **response shape** refers to the field set, field ordering, and field types of every `#[contracttype]` struct returned by a public contract method — including `SLAResult`, `SLAConfig`, `SLAConfigSnapshot`, `SLAResultSchema`, `VersionInfo`, `FailureSchema`, `HealthcheckResult`, `StorageVersionInfo`, `ContractMetadata`, `SLAStats`, `SeverityTelemetry`, `EconomicExposure`, `PauseInfo`, `ConfigUpdateInfo`, `ConfigBundle`, and `AuditState`.

---

## 2. Stability Guarantees

### 2.1 Additive-Only Field Evolution

Fields may be **appended** to the end of any response struct. A new field never breaks existing consumers that deserialise by positional index or by name — they simply ignore trailing data they do not expect.

### 2.2 Prohibited Changes

The following changes are **breaking** and MUST be avoided without a major schema version bump:

| Change | Example | Consequence |
|--------|---------|-------------|
| Field reordering | Swapping `amount` and `rating` in `SLAResult` | Deserialisation misalignment |
| Field type change | `threshold_minutes: u32` → `threshold_minutes: u64` | Downstream type errors |
| Field removal | Deleting `config_version_hash` from `SLAResult` | Consumer index shifts |
| Renaming an existing struct | `SLAStats` → `SlaStatistics` | Client code compilation failure |
| Removing a struct variant | Deleting `Negotiated` from `NegotiationOutcome` | Match exhaustiveness breakage |

### 2.3 Deprecation Over Removal

When a field is no longer needed, deprecate it rather than removing it:

1. **Deprecation phase** — the field remains in the struct but is documented as deprecated in `get_result_schema()` (for symbol fields) or in release notes (for non-symbol fields).
2. **Removal phase** — the field is removed only in the next major schema version, communicated via `RESULT_SCHEMA_VERSION` bump and `get_version_info()`.

This mirrors the symbol deprecation protocol defined in [EVENT_TOPIC_COMPATIBILITY.md](EVENT_TOPIC_COMPATIBILITY.md).

---

## 3. Schema Versioning Surfaces

### 3.1 `RESULT_SCHEMA_VERSION` (lib.rs:103)

A `u32` constant incremented whenever the `SLAResult` struct or the result symbol set undergoes a **breaking** change. Backends should monitor this value via `get_version_info()` to detect when their parsing logic must be updated.

### 3.2 `get_result_schema()` (lib.rs:1421)

Returns a `SLAResultSchema` struct containing:

- `schema_version: u32` — mirrors `RESULT_SCHEMA_VERSION`
- `status_met`, `status_violated` — current status symbols
- `payment_reward`, `payment_penalty` — current payment symbols
- `rating_exceptional`, `rating_excellent`, `rating_good`, `rating_poor` — current rating symbols
- `includes_config_version_hash: bool` — whether `SLAResult` carries `config_version_hash`
- `deprecated_symbols: Vec<DeprecatedSymbol>` — symbol deprecation catalogue

This is the **canonical introspection endpoint** for result encoding. Backends MUST call it at startup and after every contract upgrade.

### 3.3 `get_failure_schema()` (lib.rs:1379)

Returns a versioned `FailureSchema` with the full catalogue of `SLAError` codes, machine-readable labels, and descriptions. Error codes are stable: once assigned, a code is never reused. New codes are appended to the end of the enum.

### 3.4 `get_version_info()` (lib.rs:2644)

Returns a combined `VersionInfo` struct with `storage_version`, `result_schema_version`, `needs_migration`, `is_paused`, and `contract_name`. This is the single endpoint to check before any other operation.

### 3.5 `get_contract_metadata()` (lib.rs:1499)

Returns `ContractMetadata` with a `features: Vec<Symbol>` field. New features are appended; existing feature symbols are never removed or renamed.

---

## 4. Per-Type Shape Stability

| Struct | Fields Frozen At | Versioned Via | Notes |
|--------|------------------|---------------|-------|
| `SLAResult` | v1 | `RESULT_SCHEMA_VERSION` | See §2.1 for additive-only changes |
| `SLAConfig` | v1 | — | Immutable shape; new params → new struct |
| `SLAConfigSnapshot` | v1 | `version` field (`"v1"`) | Entry ordering is canonical severity order |
| `SLAResultSchema` | v1 | `schema_version` | Self-describing by design |
| `FailureSchema` | v1 | `version` field (`"v1"`) | Codes appended, never removed |
| `VersionInfo` | v1 | — | Additive-only |
| `HealthcheckResult` | v1 | — | Minimal; additive-only |
| `StorageVersionInfo` | v1 | — | Additive-only |
| `SLAStats` | v1 | — | Counter fields; additive-only |
| `EconomicExposure` | v1 | — | Breakdown severity order is canonical |
| `SeverityTelemetry` | v1 | — | Additive-only |
| `PauseInfo` | v1 | — | Additive-only |
| `ConfigUpdateInfo` | v1 | — | Additive-only |
| `ConfigBundle` | v1 | — | Delegates to contained types |
| `AuditState` | v1 | — | Aggregation type; additive-only |
| `VersionNegotiationInfo` | v1 | `protocol_version` | Part of version negotiation protocol |
| `NegotiationOutcome` | v1 | — | Variants may be added; never removed |

---

## 5. Versioning Future Constraints

When a new constraint or field is added to a response type, follow this process:

1. **Append** the new field to the end of the struct.
2. **If** the change affects how backends interpret existing fields (e.g. a new field changes the semantics of an existing one), increment `RESULT_SCHEMA_VERSION`.
3. **If** the change is purely additive and self-describing (e.g. a new telemetry field whose meaning is independent of existing fields), `RESULT_SCHEMA_VERSION` remains unchanged.
4. **Document** the new field in the struct's doc comment and in this table.
5. **Verify** with the compatibility test suite: `cargo test --package apexchainx_calculator -- api_compatibility_tests`

---

## 6. Backend Integration Guide

### 6.1 Startup Sequence

```python
# Recommended backend startup sequence (in order)
info = client.get_version_info()                  # check result_schema_version
if info.needs_migration:
    raise BlockingOperation("migrate first")

schema = client.get_result_schema()                # symbol mappings + deprecations
failure_schema = client.get_failure_schema()        # error code catalogue
config_bundle = client.get_config_bundle()          # severity configs + schema
```

### 6.2 Detecting Response-Shape Changes

```python
# Store the last-known schema version and compare on each startup
last_known_version = load_cached_version()
info = client.get_version_info()

if info.result_schema_version > last_known_version:
    logger.warning("Result schema changed: %d → %d. Re-reading symbol maps.",
                   last_known_version, info.result_schema_version)
    schema = client.get_result_schema()
    check_deprecated_symbols(schema.deprecated_symbols)
    save_cached_version(info.result_schema_version)
```

### 6.3 Handling Additive Fields

Backend deserialisers SHOULD ignore unknown trailing fields when decoding response structs. This ensures that an older backend continues to function after a non-breaking upgrade adds new fields.

```python
# Resilient deserialisation: use positional destructuring with ignore
outage_id, status, mttr, threshold, amount, payment_type, rating, *_ = result
# The *_ captures any future trailing fields we do not yet understand.
```

---

## 7. Related Documents

| Document | Relation |
|----------|----------|
| [EVENT_COMPATIBILITY_POLICY.md](EVENT_COMPATIBILITY_POLICY.md) | Event payload field ordering rules |
| [EVENT_TOPIC_COMPATIBILITY.md](EVENT_TOPIC_COMPATIBILITY.md) | Event topic layout and symbol deprecation protocol |
| [CONTRACT_API_COMPATIBILITY.md](CONTRACT_API_COMPATIBILITY.md) | Public method signature stability |
| [RESULT_PAYLOAD_HASHING.md](RESULT_PAYLOAD_HASHING.md) | Config version hash algorithm and replay verification |

---

## 8. Change Log

| Date | Version | Description |
|------|---------|-------------|
| 2026-07-30 | 1.0.0 | Initial response-shape stability policy |
