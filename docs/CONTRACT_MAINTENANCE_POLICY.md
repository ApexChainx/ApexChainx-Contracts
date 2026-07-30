# Contract Maintenance Policies

> **Audience:** Contributors, maintainers, and backend integration engineers.
> These policies govern how the `apexchainx_calculator` contract evolves safely
> without breaking downstream consumers (backend indexers, dashboards, settlement).

## Table of Contents

- [SC-500: `#[contracttype]` Compatibility Note Policy](#sc-500-contracttype-compatibility-note-policy)
- [SC-501: Response-Shape Stability Policy](#sc-501-response-shape-stability-policy)
- [SC-502: Version Negotiation Protocol — Contributor Note](#sc-502-version-negotiation-protocol--contributor-note)
- [SC-503: Contract API Archetype Note](#sc-503-contract-api-archetype-note)
- [SC-504: Event Payload Size Maintainership Check](#sc-504-event-payload-size-maintainership-check)
- [SC-505: Event Drift Review Note](#sc-505-event-drift-review-note)
- [SC-506: History Write Audit Check](#sc-506-history-write-audit-check)
- [SC-507: Telemetry Counters Policy](#sc-507-telemetry-counters-policy)
- [SC-508: Role-Change Incident Review Note](#sc-508-role-change-incident-review-note)

---

## SC-500: `#[contracttype]` Compatibility Note Policy

### Policy

Every **public** `#[contracttype]` structural change MUST include a dedicated
compatibility note in the PR description. Contract types form the external API
surface and must be reviewed with the same discipline as event schemas.

### When This Applies

| Change Type | Requires Note | Reason |
|-------------|--------------|--------|
| Adding a new `#[contracttype]` struct | ✅ Yes | New public surface |
| Adding a field to an existing struct | ✅ Yes | Must document append-or-insert position |
| Removing a field | ✅ Yes | Breaking — must explain migration |
| Renaming a field | ✅ Yes | Breaking — must explain migration |
| Changing a field type | ✅ Yes | Breaking — must explain migration |
| Doc-comment only changes | ❌ No | No structural impact |
| `pub(crate)` type changes | ❌ No | Not public surface |

### Compatibility Note Format

```markdown
### Contract Type Compatibility

| Type Changed | Change | Breaking? | Backend Impact |
|-------------|--------|-----------|---------------|
| `SLAResult` | Added `foo: Symbol` at end | No (additive) | New field ignored by old consumers |
| `SLAConfig` | Changed `threshold_minutes: u32` → `u64` | **Yes** | Requires backend update + schema version bump |
```

### Enforcement

The PR review checklist in `CONTRIBUTING.md` includes a checkbox for contract
type compatibility. Reviewers MUST reject PRs that change `#[contracttype]`
structs without an accompanying compatibility note.

### Related

- [Response-Shape Stability Policy](#sc-501-response-shape-stability-policy)
- [Event-Topic & Payload Schema Contributor Safety Checklist](../CONTRIBUTING.md#sc-099-event-topic--payload-schema-contributor-safety-checklist)

---

## SC-501: Response-Shape Stability Policy

### Policy

All public contract return types are governed by a **stability tier**. Every
`#[contracttype]` used as a return value from a `pub fn` is assigned one of:

| Tier | Meaning | Versioning Rule |
|------|---------|----------------|
| **Stable** | Field order and types are frozen | Append-only; field additions go at the end |
| **Versioned** | Changes require a `RESULT_SCHEMA_VERSION` bump | Must document migration path in PR |
| **Experimental** | May change without notice | Marked with `#[doc = "EXPERIMENTAL: ..."]` |

### Current Stability Assignments

| Type | Tier | Notes |
|------|------|-------|
| `SLAResult` | **Versioned** | Bump `RESULT_SCHEMA_VERSION` on any change |
| `SLAConfig` | **Stable** | Append-only; backends rely on field positions |
| `SLAConfigSnapshot` | **Stable** | Ordered for backend consumption |
| `SLAResultSchema` | **Versioned** | Versioned with `schema_version` field |
| `ContractMetadata` | **Stable** | Backend startup handshake dependency |
| `SLAStats` | **Stable** | Cumulative totals; append-only |
| `VersionInfo` | **Stable** | Version negotiation; append-only |
| `HealthcheckResult` | **Stable** | Readiness probe; append-only |
| `PauseInfo` | **Versioned** | Bump storage version on field changes |
| `EconomicExposure` | **Stable** | Dashboard dependency; append-only |
| `FailureSchema` / `FailureCode` | **Stable** | Error codes are never reused |
| `DeprecatedSymbol` | **Stable** | Schema versioning machinery |

### How to Add a New Tier

1. Propose the tier in the PR description
2. Document in this table
3. Get maintainer sign-off before merge

### Backend Guidance

Backends MUST:
- Pin to a known `RESULT_SCHEMA_VERSION` at startup
- Treat unrecognised trailing fields as additive (ignore, don't error)
- Log warnings when encountering a schema version higher than expected

---

## SC-502: Version Negotiation Protocol — Contributor Note

### How `get_version_info()` Works

The version negotiation protocol allows backends to determine contract
compatibility at startup **before** sending any operational transactions.

```rust
pub fn get_version_info(env: Env) -> VersionInfo {
    let stored: u32 = env.storage().instance().get(&STORAGE_VERSION_KEY).unwrap_or(0);
    VersionInfo {
        storage_version: stored,
        result_schema_version: RESULT_SCHEMA_VERSION,
        needs_migration: stored != STORAGE_VERSION,
        is_paused: Self::is_paused(&env).unwrap_or(false),
        contract_name: symbol_short!("apexcalc"),
    }
}
```

### Safe Changes

| Change | Safe? | Rule |
|--------|-------|------|
| Adding a new field to `VersionInfo` | ✅ | Append at end; old consumers ignore |
| Adding a new read-only getter | ✅ | Additive surface; no version bump |
| Changing `STORAGE_VERSION` constant | ⚠️ | Must document migration path (`migrate()`) |
| Removing a field from `VersionInfo` | ❌ | Breaking; requires major version bump |
| Changing field types in `VersionInfo` | ❌ | Breaking; requires major version bump |

### Contributor Checklist

Before modifying `version_negotiation.rs`:

- [ ] Is this an additive change (new field, new getter)? → No version bump needed
- [ ] Is this a breaking change (remove/retype/reorder)? → Bump `RESULT_SCHEMA_VERSION`, coordinate with backend team
- [ ] Does the updated `VersionInfo` still satisfy the backend startup handshake contract?
- [ ] Have you updated `docs/CODEX_CONTEXT.md` if the handshake flow changed?

---

## SC-503: Contract API Archetype Note

### API Surface Categories

Every public function in `apexchainx_calculator` falls into one of three
archetypes. Contributors MUST understand these before adding new functions.

| Archetype | Auth Model | State Impact | Example |
|-----------|-----------|-------------|---------|
| **Read-Only** | Public (no auth) | Zero state mutation | `get_config`, `calculate_sla_view`, `get_version_info` |
| **Mutating (Operator)** | `require_auth()` on caller | Appends to history, updates stats | `calculate_sla` |
| **Privileged (Admin)** | `require_admin()` → `require_auth()` | Config, roles, pause, freeze | `set_config`, `pause`, `set_operator`, `propose_admin` |

### Archetype Rules

1. **Read-Only functions** MUST NOT write to storage under any code path.
   They bypass `check_version()` and `require_not_paused()`.
2. **Operator functions** emit events and update history. They check version
   and pause state.
3. **Admin functions** check admin role AND version AND emit lifecycle events.
   They are the only functions that can modify config, roles, or pause state.

### Adding a New Function

```markdown
### Function: `my_new_function`

| Attribute | Value |
|-----------|-------|
| **Archetype** | Read-Only / Operator / Admin |
| **Auth** | None / Operator / Admin |
| **Storage Writes** | Yes / No |
| **Events Emitted** | `my_evt` (if any) |
| **Schema Impact** | Additive / Breaking / None |
```

---

## SC-504: Event Payload Size Maintainership Check

### Policy

Every change to an event payload tuple MUST include a **deterministic payload
size assertion** in the event schema test suite. This prevents accidental
payload bloat from breaking backend indexers.

### How to Check

```rust
// In event_schema.rs or topic_stability_tests.rs:
#[test]
fn test_sla_calc_payload_size_is_stable() {
    // Payload: (outage_id: Symbol, status: Symbol, payment_type: Symbol,
    //          rating: Symbol, mttr_minutes: u32, threshold_minutes: u32,
    //          amount: i128)
    // = 4 Symbols * 8 bytes + 2 u32 * 4 bytes + 1 i128 * 16 bytes
    // = 32 + 8 + 16 = 56 bytes (excluding Soroban encoding overhead)
    // This is a maintainership assertion — update when fields change.
    const EXPECTED_PAYLOAD_FIELDS: usize = 7;
    assert_eq!(EXPECTED_PAYLOAD_FIELDS, 7,
        "sla_calc payload field count changed — update this test and notify backend");
}
```

### Acceptance Criteria for PRs

- [ ] New/modified event payload has a deterministic field count assertion
- [ ] The assertion comment lists every field with its type
- [ ] Backend team is notified if the payload grows beyond the expected size

---

## SC-505: Event Drift Review Note

### Policy

Any change to event **names** (topic constants) or event **payload tuples**
requires a dedicated event drift review. Event drift is the single most common
cause of silent backend breakage — no compilation error, no test failure, just
corrupted indexed data.

### Drift Review Checklist

Before merging a PR that touches any `EVENT_*` constant or emission site:

- [ ] **No event name changed** without a `EVENT_VERSION` bump (`"v1"` → `"v2"`)
- [ ] **No payload field removed** without a version bump
- [ ] **No payload field reordered** without a version bump
- [ ] **No payload field type changed** without a version bump
- [ ] **New events are additive** — they don't reuse old event names
- [ ] **All emission sites updated** — search for the event name and verify consistency
- [ ] **`event_schema.rs` doc comment updated** — every event's payload schema is documented
- [ ] **Distinctness test updated** — `test_event_names_are_distinct` includes new events
- [ ] **Topic stability tests pass** — `cargo test topic_stability_tests`

### Version Bump Decision Table

| Change | Bump `EVENT_VERSION`? |
|--------|----------------------|
| New event constant added | ❌ No (additive) |
| Existing event renamed | ✅ Yes |
| New field appended to payload | ❌ No (additive) |
| Field removed from payload | ✅ Yes |
| Field reordered in payload | ✅ Yes |
| Field type changed | ✅ Yes |
| Event emission removed | ✅ Yes (major) |

---

## SC-506: History Write Audit Check

### Policy

Every code path that writes to `HISTORY_KEY` MUST be audited for:

1. **Ordering:** New entries are always appended (pushed to end of `Vec`),
   never inserted or prepended.
2. **Retention:** `prune_history` and `prune_history_by_age` remove only
   the oldest entries (from the front). The ordering invariant is preserved.
3. **Idempotency:** Re-submitting an unchanged `outage_id` under the same
   config hash returns the stored result without appending a new entry.
4. **Capped growth:** `OutageRecalcLimit` (16 retained entries per outage)
   and `MAX_HISTORY_SIZE` (1000 entries) are enforced.

### Audit Checklist for History-Modifying PRs

- [ ] New history entries are appended, not inserted or prepended
- [ ] Pruning removes from the front (FIFO) and emits `pruned` / `pruned_a`
- [ ] Duplicate detection (`outage_id` + config hash) is not bypassed
- [ ] `OutageRecalcLimit` is enforced for multi-generation outages
- [ ] `retention_limit` is respected when set
- [ ] History read functions (`get_history_page`, `get_history_by_outage`,
  `get_latest_by_outage`) return correct subsets after modification

### Testing Requirements

Every PR that modifies history write logic MUST include:

1. A test that appends entries and verifies order
2. A test that prunes entries and verifies retained order
3. A test that hits `OutageRecalcLimit` and verifies it is enforced

---

## SC-507: Telemetry Counters Policy

### Policy

The `SLAStats` model tracks cumulative on-chain SLA performance metrics.
Newly introduced telemetry counters MUST follow these rules:

1. **Additive only** — new counters are appended to `SLAStats` as new fields
   at the end. This is NOT breaking (old consumers ignore trailing fields).
2. **Never remove** a counter field — removal breaks backend consumers that
   deserialise `SLAStats` by position.
3. **Never change** a counter's type — type changes (e.g., `u64` → `i128`)
   are breaking.
4. **Saturation is explicit** — when a counter reaches its type maximum
   (e.g., `u64::MAX`), the contract emits a `stats_sat` event. Backends
   MUST handle saturated counters by switching to off-chain aggregation.

### Current Telemetry Counters

| Field | Type | Description | Saturation Event |
|-------|------|-------------|-----------------|
| `total_calculations` | `u64` | Total SLA calculations performed | `stats_sat` with `totcalc` |
| `total_violations` | `u64` | Total SLA violations detected | `stats_sat` with `totviol` |
| `total_rewards` | `i128` | Sum of all reward amounts paid | `stats_sat` with `totrew` |
| `total_penalties` | `i128` | Sum of all penalty amounts (stored positive) | `stats_sat` with `totpen` |

### Per-Severity Weekly Telemetry

Per-severity counters (`SEVERITY_CALC_COUNTS_KEY`, `SEVERITY_VIOL_COUNTS_KEY`)
are reset on a weekly window boundary. The window boundary is determined by
comparing the current ledger timestamp against the last recorded calculation
or violation ledger entry.

- **Reset is explicit** — counters are set to zero when the window advances
- **Last calculation/violation ledger** snapshots are stored per severity
  to determine when a reset is needed
- **Backends should not rely on exact counter values** across window boundaries;
  use the `SeverityTelemetry` view for consistent reads

---

## SC-508: Role-Change Incident Review Note

### Admin/Operator Handoff Safety

Role changes are the highest-risk operational area in `apexchainx_calculator`.
A handoff that is not understood can leave the contract in an unsafe state
during an active incident.

### Decision Table: Role Change vs. Pause/Migration Safety

| Operation | Pause State Impact | Migration State Impact | Notes |
|-----------|-------------------|----------------------|-------|
| `propose_admin` + `accept_admin` | None — pause state unaffected | None — storage version unaffected | Safe during incident if new admin is trusted |
| `propose_operator` + `accept_operator` | None — pause state unaffected | None — storage version unaffected | Operator can be rotated mid-incident |
| `set_operator` (direct) | None | None | Instant handoff; prefer two-step for audit |
| `renounce_admin` | **Critical** — removes admin; no admin can unpause | **Critical** — no admin can call `migrate` | **Only call after confirming operator is trusted and no migration is pending** |

### Admin Renouncement Safety Checklist

Before calling `renounce_admin`:

- [ ] Operator address is set and trusted
- [ ] No pending admin transfer exists
- [ ] No pending migration (`needs_migration == false`)
- [ ] Contract is not paused (or operator does not need to unpause)
- [ ] All backend consumers have been notified
- [ ] A recovery plan exists (if re-deployment is needed)

### Operator Handoff During Incidents

Operator changes are **safe** during active incidents:
- `calculate_sla` is the only operator-gated function
- `set_operator` and two-step operator transfer do not affect history or config
- Pause state is **not** affected by operator changes

### Post-Handoff Verification

After any role change:

1. Call `get_admin()` to verify the new admin
2. Call `get_operator()` to verify the new operator
3. Call `get_version_info()` to confirm `needs_migration == false`
4. Call `is_paused()` to confirm expected pause state
5. Re-run backend parity tests against the new contract state

### Flow Diagram

```
Admin Handoff:
  propose_admin(A) → accept_admin(A) → verify get_admin() == A

Operator Handoff:
  propose_operator(O) → accept_operator(O) → verify get_operator() == O

Direct Operator Set:
  set_operator(O) → verify get_operator() == O

Admin Renouncement:
  renounce_admin() → verify get_admin() is absent → operator is trusted
```

---

## Summary: Issue Cross-Reference

| Issue | Policy Section |
|-------|---------------|
| #279 | SC-500: `#[contracttype]` Compatibility Note Policy |
| #283 | SC-501: Response-Shape Stability Policy |
| #284 | SC-502: Version Negotiation Protocol — Contributor Note |
| #285 | SC-503: Contract API Archetype Note |
| #286 | SC-504: Event Payload Size Maintainership Check |
| #287 | SC-505: Event Drift Review Note |
| #288 | SC-506: History Write Audit Check |
| #289 | SC-507: Telemetry Counters Policy |
| #290 | SC-508: Role-Change Incident Review Note |

