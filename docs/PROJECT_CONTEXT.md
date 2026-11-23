# ApexChainx System — Project Context

> **Purpose:** This document describes the high-level system architecture, repository landscape,
> and future contract roadmap for the ApexChainx platform.

## Table of Contents

- [Repository Architecture](#repository-architecture)
- [System Flow](#system-flow)
- [Architectural Rules](#architectural-rules)
- [Contract API Archetypes](#contract-api-archetypes)
- [Contract Lifecycle](#contract-lifecycle)- [SC-100: Future Contract Roadmap](#sc-100-future-contract-roadmap)
- [Pruning benchmark note](PRUNING_BENCHMARK_NOTE.md)

---

## Repository Architecture

The ApexChainx platform is composed of three repositories:

| Repository | Role | Technology |
|------------|------|------------|
| `apexchainx-fe` | Frontend application | React / TypeScript |
| `apexchainx-be` | Backend API and integration layer | Python / FastAPI |
| `apexchainx-contracts` | Soroban smart contracts (this repo) | Rust / Soroban SDK |

## System Flow

```
 User
  |
  v
┌─────────┐     ┌─────────┐     ┌──────────────┐
│   FE    │ ──→ │   BE    │ ──→ │  Contracts   │
│ (React) │ ←── │ (API)   │ ←── │  (Soroban)   │
└─────────┘     └─────────┘     └──────────────┘
```

## Architectural Rules

1. **Frontend never calls contracts directly** — all contract interactions go through the backend
2. **Backend is the exclusive bridge** — translates contract data to frontend-friendly responses
3. **Contracts are execution-layer only** — pure deterministic computation, no external dependencies

---

## Contract API Archetypes

Every public entrypoint in `apexchainx_calculator` falls into one of three
archetypes. This classification is the fastest way for contributors and
integrators to decide whether a call is safe to make freely, requires the
operator role, or is gated behind admin authority.

### Read-only (no auth required, no state written)

Safe to call by anyone at any time, including while the contract is paused.
No on-chain state is written and no events are emitted.

| Group | Methods |
|-------|---------|
| Healthcheck & version | `healthcheck`, `get_version_info`, `get_migration_state` |
| Pause / freeze status | `is_paused`, `get_pause_info`, `is_config_frozen` |
| Config views | `get_config`, `get_config_snapshot`, `get_config_version_hash`, `list_configs`, `get_last_config_update`, `get_config_bundle` |
| Custom severity views | `get_custom_severity`, `get_custom_config_snapshot` |
| Stats & telemetry | `get_stats`, `get_economic_exposure`, `get_severity_telemetry` |
| History views | `get_history`, `get_history_page`, `get_history_page_with_meta`, `get_history_by_outage`, `get_latest_by_outage` |
| Role queries | `get_admin`, `get_operator`, `get_pending_admin`, `get_pending_operator` |
| Introspection | `get_result_schema`, `get_failure_schema`, `get_contract_metadata`, `get_full_audit_state` |
| Retention helpers | `get_retention_limit`, `get_config_count`, `get_storage_version` |
| View-mode calculation | `calculate_sla_view`, `replay_calculate_sla` |

### Mutating — operator role

Writes state and emits events. Only the current **operator** address may call
these.

| Method | What it writes |
|--------|----------------|
| `calculate_sla` | Appends a result to history, updates cumulative stats and per-severity telemetry, emits `sla_calc` and `set_int` events. Idempotent on exact replay; rejects conflicting duplicates with a `dup_input` event carrying the stored result. |

### Privileged — admin role

Only the current **admin** address may call these. They control lifecycle,
configuration, and role management. Some are irreversible (`renounce_admin`)
or have broad blast radius (`prune_history`).

| Group | Methods |
|-------|---------|
| Lifecycle | `initialize`, `migrate` |
| Config management | `set_config`, `set_custom_severity`, `remove_custom_severity`, `freeze_config`, `unfreeze_config` |
| Operational controls | `pause`, `unpause`, `set_retention_limit` |
| Admin transfer | `propose_admin`, `accept_admin`, `cancel_admin_proposal`, `renounce_admin` |
| Operator transfer | `set_operator` *(legacy direct)*, `propose_operator`, `accept_operator`, `cancel_operator_proposal` |
| History pruning | `prune_history`, `prune_history_by_age` |

> **Quick rule of thumb:** methods whose names start with `get_`, `is_`,
> `list_`, `healthcheck`, `calculate_sla_view`, or `replay_calculate_sla` are
> read-only and safe to call freely. `calculate_sla` requires the operator
> role. Everything else requires the admin role.
## Telemetry & Weekly Reset Semantics

The `apexchainx_calculator` contract maintains per-severity telemetry (`SeverityTelemetry`) tracking calculation counts, violation counts, and violation rates.

### Operator Posture & Reset Behavior

1. **Lazy On-Execution Evaluation**:
   - The contract checks the timestamp of the last calculation/violation for the invoked severity lane when `calculate_sla` is called.
   - If $\ge 7$ days ($604,800$ seconds) have passed since the recorded timestamp for that severity lane, the calculation and violation counters for that specific lane are reset to `0` before processing the current calculation.

2. **Per-Severity Isolation**:
   - Resets are evaluated per severity lane (`critical`, `high`, `medium`, `low`).
   - Activity in one lane does not reset or refresh timestamps for other lanes.

3. **Impact on Backend Consumers and Monitoring Dashboards**:
   - `get_severity_telemetry()` reflects stored counters. Inactive lanes retain their last updated telemetry state until the next invocation in that lane triggers a lazy reset.
   - Replays and duplicate resubmissions with identical inputs/configs do NOT update or reset telemetry counters.
   - Off-chain monitoring systems or backend consumers desiring continuous 7-day rolling window analytics should aggregate on-chain `EVENT_SLA_CALC` events or poll `get_severity_telemetry()` periodically alongside contract calls.
## Contract Lifecycle

The `apexchainx_calculator` contract has four independent state axes
(initialized, version-matched, paused, config-frozen) that combine to determine
which operations are permitted at any moment.

**→ See the full state-transition diagram: [docs/CONTRACT_LIFECYCLE.md](CONTRACT_LIFECYCLE.md)**

Quick overview of the main lifecycle states:

```
[Uninitialized] ──initialize()──→ [Active]
    [Active] ──pause()──→ [Paused] ──unpause()──→ [Active]
    [Active] ──(binary upgrade)──→ [NeedsMigration] ──migrate()──→ [Active]
    [Active] ──freeze_config()──→ [ConfigFrozen] ──unfreeze_config()──→ [Active]
    [Active] ──renounce_admin()──→ [AdminRenounced]  ← irreversible
```

See [`CONTRACT_LIFECYCLE.md`](CONTRACT_LIFECYCLE.md) for Mermaid diagrams of
each flow, the combined state matrix, and the full invariants table.
## SC- Marker Convention

Throughout the contract crate, inline comments and doc comments carry `SC-NNN`
and `SC-W5-NNN` markers that link code to the GitHub issues that motivated the
design. The rules for adding, updating, and retiring these markers are in
[`docs/SC_MARKER_POLICY.md`](SC_MARKER_POLICY.md).
---

## SC-100: Future Contract Roadmap

This section documents the planned evolution of `apexchainx-contracts` based on
current backend integration needs and business requirements.

### Versioning Strategy

| Version | Scope | Timeline |
|---------|-------|----------|
| v1.0 | Single crate (`apexchainx_calculator`) | ✅ Current |
| v1.1 | Multi-contract version negotiation | ✅ Current |
| v2.0 | Payment escrow integration | Planned |
| v2.1 | Multi-party settlement | Planned |
| v3.0 | On-chain governance with timelocks | Planned |

### Current State

Only one contract crate exists in this repository:

| Crate | Status | Description | Key Features |
|-------|--------|-------------|--------------|
| `apexchainx_calculator` | **Production-ready** | SLA calculator contract | Config management, role-based auth, event emission, version negotiation, result schema |

### Planned Additions

The following crates are planned but **not yet implemented**. Do not import or
reference them until they appear in the repository.

| Crate | Status | Depends On | Description |
|-------|--------|------------|-------------|
| `payment_escrow` | Planned | `apexchainx_calculator` | Locks and conditionally releases Stellar token payments based on SLA results |
| `settlement` | Planned | `payment_escrow` | Splits shared outage costs between multiple parties |
| `governance` | Planned | — | On-chain admin config changes with time-locked execution |

### Event Ordering Guarantees

For the documented event ordering contract that backend consumers can rely on
for correct event processing, see **[Event Ordering Guarantees](./event-ordering-guarantees.md)**.
This document describes the deterministic event sequencing within a single
ledger, backed by the test suite in `apexchainx_calculator/src/event_ordering_tests.rs`.

### Integration Expectations

- The backend (`apexchainx-be`) currently integrates only with `apexchainx_calculator`
- New crates will be introduced incrementally
- Each new crate must expose a `get_result_schema()` equivalent for safe version pinning
- Frontend never calls contracts directly — all invocations go through the backend
- Backend indexers and operators should follow the [Observability Contract](OBSERVABILITY_CONTRACT.md) for health-signal monitoring and alerting guidance

### Upgrade & Migration

Storage-version upgrades follow the [Upgrade Playbook](UPGRADE_PLAYBOOK.md), which documents the complete preflight → migration → verification → rollback workflow for operators.

### API Stability

All public contract entrypoints are classified by compatibility risk in the **[API Stability Scorecard](API_STABILITY_SCORECARD.md)**. Contributors must consult this scorecard before modifying any public function signature to determine whether the change is additive or breaking.
### Config Event Ordering Regression Policy

Every successful `set_config()` call appends exactly one `cfg_upd` event. Across
repeated calls, events must remain in invocation order, including when severities
are interleaved or the same severity is overwritten. Each event must describe
the individual write that produced it:

- topics are `(cfg_upd, v1, severity)`
- payload fields remain `(threshold_minutes, penalty_per_minute, reward_base)`
- payload values must not be reordered, deduplicated, coalesced, or replaced by
  values from a later write
- rejected calls emit no `cfg_upd` event and therefore add nothing to the stream

Treat
`test_repeated_set_config_events_preserve_call_and_payload_order` in
`apexchainx_calculator/src/tests.rs` as the canonical repeated-write ordering
test. The stable schema assertions
`test_set_config_emits_versioned_config_event`,
`test_cfg_upd_event_topic_count_is_three`, and
`test_cfg_upd_event_payload_field_count_is_three` remain the source of truth for
the event's topic and payload shapes. Any intentional change to these invariants
requires updating the tests, this policy, and the versioning rules in
`apexchainx_calculator/src/event_schema.rs`.
### State Reads Policy

This section documents which public getters are **side-effect-free** (safe to call without
auth, no storage writes, no event emission) and which involve non-trivial storage access,
auth checks, or computation. This matters for backend startup handshakes, health probes,
and cache-warming strategies.

#### Bypass endpoints (safe before `initialize` or `migrate`)

These functions do **not** call `check_version` and remain callable even when the contract
is uninitialized or in a pre-migration state. They are the safest choice for startup probes.

| Function | Storage reads | Notes |
|----------|--------------|-------|
| `healthcheck` | `STORAGE_VERSION_KEY` | Pure read; returns `ready` bool + status label. No auth, no events. Ideal for LB probes. |
| `get_migration_state` | `STORAGE_VERSION_KEY` | Pure read; returns stored/expected version + `needs_migration` flag. |
| `get_version_info` | `STORAGE_VERSION_KEY`, `PAUSED_KEY` | Pure read; combines version + pause state in one call. |
| `get_storage_version` | `STORAGE_VERSION_KEY` | Pure read; single `u32`. Bare-minimum probe. |

#### Side-effect-free getters (require `initialize` + version match)

These call `check_version` (reads `STORAGE_VERSION_KEY`), then perform read-only work.
None require caller auth, mutate storage, or emit events.

| Function | Storage reads | Notes |
|----------|--------------|-------|
| `get_admin` | `ADMIN_KEY` | Wrapped in `Result` — returns `NotInitialized` if absent. |
| `get_operator` | `OPERATOR_KEY` | Same pattern as `get_admin`. |
| `get_pending_admin` | `PENDING_ADMIN_KEY` | Returns `Option<Address>`. |
| `get_pending_operator` | `PENDING_OP_KEY` | Returns `Option<Address>`. |
| `is_paused` | `PAUSED_KEY` | Returns `bool`, defaults to `false`. |
| `get_pause_info` | `PAUSE_INFO_KEY` | Returns `Option<PauseInfo>`. |
| `is_config_frozen` | Config-freeze storage key | Delegates to `config_freeze::is_config_frozen`. |
| `get_config` | `CONFIG_KEY` | Single-severity lookup via `load_config`. |
| `list_configs` | `CONFIG_KEY` | Returns the full config `Map`. |
| `get_config_snapshot` | `CONFIG_KEY` | Assembles deterministic `Vec<SLAConfigEntry>` in canonical order. |
| `get_config_version_hash` | `CONFIG_KEY` | Reads all canonical configs, computes polynomial rolling hash. |
| `get_config_bundle` | `CONFIG_KEY` | Composite of `get_config_snapshot` + `get_result_schema`. |
| `get_config_count` | `CONFIG_KEY` | Returns `configs.len()`. |
| `get_last_config_update` | Config-metadata key | Returns `Option<ConfigUpdateInfo>` for cache-invalidation. |
| `get_custom_severity` | `CUSTOM_CONFIG_KEY` | Single custom-severity lookup. |
| `get_custom_config_snapshot` | `CUSTOM_CONFIG_KEY` | Iterates custom map in insertion order. |
| `get_stats` | `STATS_KEY` | Returns `SLAStats` struct. |
| `get_severity_telemetry` | `CALCCNT`, `VIOLCNT` | Reads packed `u128` telemetry lanes, decodes per-severity rates. |
| `get_economic_exposure` | `CONFIG_KEY` | Reads all canonical configs, computes max-reward + penalty-rate totals. |
| `get_history` | `HISTORY_KEY` | Returns full `Vec<SLAResult>`. |
| `get_history_page` | `HISTORY_KEY` | Bounded slice of history. See the [History Pagination Policy](HISTORY_PAGINATION_POLICY.md) (issue #263): offset-based, oldest-first, empty-page end-of-history signalling, saturating `offset + limit` arithmetic. |
| `get_history_page_with_meta` | `HISTORY_KEY` | Same page as `get_history_page` plus `HistoryPage` metadata (`items`, `total`, `has_more`) so consumers can page without a separate `get_history` call (#380). |
| `get_history_by_outage` | `HISTORY_KEY` | Filters history by `outage_id`. |
| `get_latest_by_outage` | `HISTORY_KEY` | Scans history for newest match. |
| `get_retention_limit` | `RETENTION_LIMIT_KEY` | Returns `u32`, defaults to `MAX_HISTORY_SIZE`. |
| `get_full_audit_state` | Multiple keys | Composite read composing many of the above. |
| `get_result_schema` | None (post-version-check) | Constructs `SLAResultSchema` from constants. Static data. |
| `get_failure_schema` | None (post-version-check) | Constructs `FailureSchema` from hardcoded entries. Static data. |
| `get_contract_metadata` | None (post-version-check) | Constructs `ContractMetadata` from constants + static severity list. |
| `calculate_sla_view` | `CONFIG_KEY` | Pure computation; reads config, runs SLA math, returns result. No writes, no events. |
| `replay_calculate_sla` | `CONFIG_KEY` | Same as `calculate_sla_view` but accepts explicit `recorded_at_ledger`. |

#### Non-trivial / state-mutating calls

These require caller auth, write storage, emit events, or all three. They must **not** be
used in startup probes or cache-warming pipelines.

| Function | Auth required | Side-effects |
|----------|--------------|--------------|
| `initialize` | `admin`, `operator` | Writes all initial storage keys. |
| `set_operator` | `admin` | Writes `OPERATOR_KEY`, emits `op_set`. |
| `propose_admin` / `accept_admin` / `cancel_admin_proposal` | `admin` or `pending` | Two-step admin transfer; writes/removes `PADMIN`/`ADMIN`, emits events. |
| `propose_operator` / `accept_operator` / `cancel_operator_proposal` | `admin` or `pending` | Two-step operator handoff; writes/removes `POP`/`OPERATOR`, emits events. |
| `renounce_admin` | `admin` | Removes `ADMIN` + `PADMIN`, emits `adm_ren`. Irreversible. |
| `pause` / `unpause` | `admin` | Writes `PAUSED`/`PAUSEINF`, emits `paused`/`unpause`. |
| `freeze_config` / `unfreeze_config` | `admin` | Delegates to `config_freeze`, emits `cfg_frz`/`cfg_unfrz`. |
| `set_config` | `admin` | Validates + writes `CONFIG_KEY`, stamps `LAST_CFG_UPDATE`, emits `cfg_upd`. |
| `set_custom_severity` / `remove_custom_severity` | `admin` | Mutates `CUSTOM_CONFIG_KEY`, emits `sev_add`/`sev_upd`/`cfg_rem`. |
| `calculate_sla` | `operator` | Writes history, stats, telemetry; emits `sla_calc` + `set_int`. |
| `set_retention_limit` | `admin` | Writes `RETLIM`. |
| `prune_history` / `prune_history_by_age` | `admin` | Truncates `HIST`, emits `pruned`/`pruned_a`. |
| `migrate` | `admin` | Runs migration harness, writes storage version, emits `migrate_done`. |

#### Startup handshake recipe

The recommended startup sequence for backend consumers is:

1. **`healthcheck()`** — quick liveness probe (no auth, no version check).
2. **`get_version_info()`** — if ready, check `needs_migration` + `is_paused`.
3. **`get_config_bundle()`** — warm config cache and result-schema in one RPC.
4. **`get_failure_schema()`** — pre-load error-code catalogue for bridge mapping.

All four are side-effect-free. Steps 2–4 require an initialized contract; step 1 does not.### Contribution Guidelines for New Crates

1. **Open a tracking issue** before creating the crate directory
2. **Follow the established layout**: `src/lib.rs`, `src/tests.rs`, `Cargo.toml`
3. **Add to CI matrix** in `.github/workflows/`
4. **Export a result schema** function so the backend can detect breaking changes
5. **Include version negotiation** support for multi-contract compatibility
6. **Update the upgrade playbook** if the new crate introduces its own storage-version migration path
