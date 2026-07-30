# Severity Alias Deprecation & Historical Result Preservation Policy

> **Audience:** Contributors, maintainers, and backend integration engineers.
> This policy governs how severity level aliases (e.g., "critical", "high", "medium", "low")
> are renamed or deprecated while preserving the integrity of historical `SLAResult` entries.

## Problem Statement

The `apexchainx_calculator` contract stores `SLAResult` entries in on-chain history with
severity symbols as immutable fields. If a severity alias is renamed (e.g., "critical" → "crit"),
historical results would continue to reference the old symbol. Without a documented migration
path, backend indexers and dashboards would encounter orphaned severity references that no
longer match the active configuration.

## Policy Scope

This policy applies to:

- **Canonical severities**: The four built-in severity levels (`critical`, `high`, `medium`, `low`)
- **Custom severities**: User-defined severity levels added via `set_custom_severity`
- **Historical results**: All `SLAResult` entries stored in `HISTORY_KEY`

## Severity Alias Deprecation Lifecycle

When a severity alias needs to change, follow this three-phase lifecycle:

### Phase 1: Introduction (Minor Release)

**Action**: Add the new severity alias alongside the old one.

**Requirements**:
- Both the old and new severity symbols must be valid in `CONFIG_KEY` or `CUSTOM_CONFIG_KEY`
- `get_result_schema()` is updated to include a `severity_aliases` entry mapping old → new
- The `deprecated_at` field is set to the current `RESULT_SCHEMA_VERSION`
- The `removal_version` field is set to `None` (TBD)

**Backend Impact**:
- Backends can query `get_result_schema()` to detect the pending deprecation
- Historical results with the old severity symbol remain valid
- New calculations can use either the old or new severity symbol

**Example**:
```rust
// In get_result_schema():
severity_aliases: vec![
    SeverityAliasMapping {
        old_severity: symbol_short!("critical"),
        new_severity: symbol_short!("crit"),
        deprecated_at: 2,
        removal_version: None,
    }
]
```

### Phase 2: Coexistence (At Least One Minor Release)

**Action**: Maintain both aliases in active configuration.

**Requirements**:
- Both severity symbols continue to resolve to the same `SLAConfig`
- No breaking changes to event payloads or result schemas
- Backend consumers are notified via the `severity_aliases` field
- Documentation is updated to recommend the new alias

**Backend Impact**:
- Backends should log warnings when processing historical results with deprecated severities
- New submissions should prefer the new alias, but the old alias remains accepted
- Indexers should maintain a mapping table for historical queries

**Duration**: Minimum of one minor release cycle to allow backend migration.

### Phase 3: Removal (Major Release)

**Action**: Remove the old severity alias from active configuration.

**Requirements**:
- The old severity symbol is removed from `CONFIG_KEY` or `CUSTOM_CONFIG_KEY`
- `get_result_schema()` updates the `severity_aliases` entry with `removal_version` set
- `RESULT_SCHEMA_VERSION` is incremented to signal breaking change
- Historical results are NOT modified (they retain the old symbol permanently)

**Backend Impact**:
- Backends MUST use the `severity_aliases` mapping to interpret historical results
- New calculations using the old severity symbol return `InvalidSeverity`
- Indexers MUST query `get_result_schema()` at startup to build the alias translation table

**Example**:
```rust
// In get_result_schema():
severity_aliases: vec![
    SeverityAliasMapping {
        old_severity: symbol_short!("critical"),
        new_severity: symbol_short!("crit"),
        deprecated_at: 2,
        removal_version: Some(3),
    }
]
```

## Data Structures

### SeverityAliasMapping

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeverityAliasMapping {
    /// The deprecated severity symbol still present in historical results.
    pub old_severity: Symbol,
    /// The replacement severity symbol that supersedes it.
    pub new_severity: Symbol,
    /// The schema version at which this deprecation was introduced.
    pub deprecated_at: u32,
    /// The schema version at which the old severity was removed from active config.
    /// None indicates the severity is still valid (coexistence phase).
    pub removal_version: Option<u32>,
}
```

### SLAResultSchema Extension

The `SLAResultSchema` struct includes a `severity_aliases` field:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SLAResultSchema {
    // ... existing fields ...
    
    /// Deprecated severity alias mappings for historical result interpretation.
    /// Backends use this to translate old severity symbols to current ones.
    pub severity_aliases: Vec<SeverityAliasMapping>,
}
```

## Backend Integration Guidance

### Startup Handshake

Backends MUST call `get_result_schema()` at startup and:

1. Parse the `severity_aliases` vector
2. Build an in-memory translation table: `old_severity → new_severity`
3. Log warnings for any deprecated severities still in use
4. Store the `removal_version` for each mapping to detect when old aliases are fully removed

### Historical Query Processing

When querying historical results:

1. Check each `SLAResult.severity` against the alias translation table
2. If a match is found, translate to `new_severity` before dashboard rendering
3. If no match is found, the severity is current (no translation needed)
4. If the severity is not in active config AND not in aliases, log an error (data corruption)

### Event Processing

When processing `sla_calc` or `set_int` events:

1. The `severity` in `topic[2]` is the authoritative source at emission time
2. Do not translate event severity symbols — they represent the state at that ledger
3. Use the alias table only when aggregating or comparing across time periods

## Canonical Severity Stability Guarantee

The four canonical severities (`critical`, `high`, `medium`, `low`) are **guaranteed stable**
for the v1.x contract lifecycle. Any change to these requires:

1. A major version bump (v2.0)
2. A storage migration
3. Coordination with the backend team
4. A minimum 6-month deprecation notice period

Custom severities added via `set_custom_severity` follow the standard deprecation lifecycle
and may be removed with a single minor release cycle.

## Migration Example

### Scenario: Rename "critical" to "crit"

**v1.2 (Introduction)**:
```rust
// Both severities valid in config
CONFIG_KEY: {
    "critical": SLAConfig { ... },
    "crit": SLAConfig { ... },  // New alias points to same config
}

// get_result_schema() returns:
severity_aliases: [
    { old_severity: "critical", new_severity: "crit", deprecated_at: 2, removal_version: None }
]
```

**v1.3 (Coexistence)**:
- No changes to config
- Backend teams migrate their dashboards to use "crit"
- Historical results still show "critical"

**v2.0 (Removal)**:
```rust
// Old alias removed from config
CONFIG_KEY: {
    "crit": SLAConfig { ... },  // Only new alias remains
}

// get_result_schema() returns:
severity_aliases: [
    { old_severity: "critical", new_severity: "crit", deprecated_at: 2, removal_version: Some(3) }
]
```

Backend queries for historical results automatically translate "critical" → "crit".

## Enforcement

### PR Review Checklist

Before merging a PR that modifies severity aliases:

- [ ] `get_result_schema()` includes the updated `severity_aliases` vector
- [ ] `deprecated_at` is set to the current `RESULT_SCHEMA_VERSION`
- [ ] `removal_version` is `None` for introduction, set for removal
- [ ] Documentation is updated with the deprecation timeline
- [ ] Backend team is notified via the PR description
- [ ] Tests verify the alias mapping is returned correctly
- [ ] Historical result queries work with the translation table

### CI Validation

A test in `tests.rs` verifies:

1. `get_result_schema()` returns a non-empty `severity_aliases` when deprecations exist
2. Each mapping has valid `old_severity`, `new_severity`, and `deprecated_at`
3. `removal_version` is either `None` or a valid schema version
4. No circular mappings exist (old → new → old)

## Related Policies

- [Event Compatibility Policy](EVENT_COMPATIBILITY_POLICY.md) — Event payload stability
- [Contract Maintenance Policy](CONTRACT_MAINTENANCE_POLICY.md) — SC-500 through SC-508
- [SC-501: Response-Shape Stability Policy](CONTRACT_MAINTENANCE_POLICY.md#sc-501-response-shape-stability-policy)

## Issue Cross-Reference

| Issue | Policy Section |
|-------|---------------|
| #239 | Full policy |
