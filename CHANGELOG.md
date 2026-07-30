# Changelog

> All interface-affecting changes to `apexchainx-contracts` are recorded here.
> This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
> and follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) conventions.

---

## [Unreleased]

### Changed
- `test_storage_key_namespace_symbols_are_distinct` now covers all 17 on-chain storage key constants (previously omitted `SEVERITY_CALC_COUNTS_KEY`, `SEVERITY_VIOL_COUNTS_KEY`, `LAST_CALCULATION_LEDGER_KEY`, `LAST_VIOLATION_LEDGER_KEY`, and `LAST_CFG_UPDATE_KEY`). The assertion now includes the colliding indices in its error message for faster diagnosis. A maintenance comment listing every key and a pointer to this test was added to both the storage-key block in `lib.rs` and the test itself so future contributors know to update both locations when adding a new key.
### Fixed
- Replaced stale `test_zero_threshold_always_violated` test in `threshold_config.rs`
  with two correct tests that verify `set_config` rejects `threshold_minutes = 0`
  with `InvalidThreshold` (code 8). The previous test incorrectly assumed a
  zero-threshold write would succeed and then tested calculation behaviour on an
  impossible stored state.
- Hardened `validate_cross_severity_penalty_ordering` in `lib.rs` to use
  `.ok_or(SLAError::InvalidSeverity)?` instead of `.unwrap()` when indexing
  into the canonical severity list. The function is now panic-free: if the
  internal severity list invariant is ever broken the call surfaces a
  deterministic `InvalidSeverity` error rather than an unrecoverable host trap.
### Added
- `docs/CONTRACT_SHAPE_CHANGE_CHECKLIST.md` — release-readiness checklist for PRs that touch storage keys, `STORAGE_VERSION`, event topic constants, or event payload fields; cross-referenced from `CONTRIBUTING.md` as SC-100
- **[SC-509] SLAError Addition Workflow** (#253) — comprehensive contributor guide for adding, deprecating, or reviewing `SLAError` variants without breaking backend adapter logic. See `docs/sla-error-additions-guide.md`.
- `error_responses::is_severity_not_in_set` — typed helper predicate for `SLAError::SeverityNotInSet` (#253)
- `docs/sla-error-additions-guide.md` — step-by-step guide covering SLAError enum management, the typed helper layer, compatibility expectations, and testing requirements (#253)- `docs/CONTRACT_MAINTENANCE_POLICY.md` — comprehensive maintenance policy covering `#[contracttype]` compatibility notes (#279), response-shape stability (#283), version negotiation (#284), API archetypes (#285), event payload size checks (#286), event drift review (#287), history write audit (#288), telemetry counters (#289), and role-change incident review (#290)
- `docs/CONTRACT_LIFECYCLE.md` — Mermaid state-transition diagrams for the `apexchainx_calculator` contract lifecycle: top-level lifecycle, pause/unpause, storage migration, config-freeze, admin transfer (two-step), and operator handoff flows; plus the combined orthogonal state matrix and invariants table (closes #256)- `docs/CONTRACT_MAINTENANCE_POLICY.md` — comprehensive maintenance policy covering `#[contracttype]` compatibility notes (#279), response-shape stability (#283), version negotiation (#284), API archetypes (#285), event payload size checks (#286), event drift review (#287), history write audit (#288), telemetry counters (#289), and role-change incident review (#290)- `tooling/release-summary.ts` — release summary generator for maintainers (#280)
- `.devcontainer/` — reproducible dev container workspace with Rust + WASM target + just + Node.js (#281)
- `just bootstrap` target — session-safe, idempotent one-command local bootstrap for the Rust WASM contract workflow: verifies rustup, installs the pinned `1.94.1` toolchain with `rustfmt` + `clippy` components, adds `wasm32-unknown-unknown` target, and verifies `cargo` is on `PATH` (closes #257)
- `docs/CONTRACT_MAINTENANCE_POLICY.md` — comprehensive maintenance policy covering `#[contracttype]` compatibility notes (#279), response-shape stability (#283), version negotiation (#284), API archetypes (#285), event payload size checks (#286), event drift review (#287), history write audit (#288), telemetry counters (#289), and role-change incident review (#290)
- `docs/EVENT_DRIFT_CHECKLIST.md` — standalone quick-reference event drift review checklist for everyday maintainer use (#287)
- `tooling/release-summary.ts` — release summary generator for maintainers (#280)
- `scripts/release-replay.ts` — minimal release candidate validation command for fast pre-release checks (#270)
- `just release-replay` and `just release-replay-full` targets — fast and full release validation (#270)
- `.devcontainer/` — reproducible dev container workspace with Rust + WASM target + just + Node.js, including setup README (#281)
- `just bootstrap` target — one-command local environment setup (#281)- Historical parity checker test (`test_historical_parity_golden_results`) — validates current contract behavior against known golden results for release regression detection (#282)
- `get_config_version_hash` — deterministic hash of the current config snapshot for backend parity validation
- `get_result_schema` — explicit schema descriptor for SLA result encoding (status, payment type, rating symbols)
- `calculate_sla_view` — read-only simulation of SLA calculation without state mutation or auth requirement
- `get_config_snapshot` — ordered snapshot of all severity configs with version tag
- `migrate` — admin-only migration function to bump the storage schema version (SC-021)
- `get_admin` — read the current admin address
- Two-step admin transfer governance functions: `propose_admin`, `accept_admin`, `cancel_admin_proposal`, and `get_pending_admin` (SC-024, SC-063)
- Two-step operator handoff governance functions: `propose_operator`, `accept_operator`, `cancel_operator_proposal`, and `get_pending_operator` (SC-024, SC-064)
- `renounce_admin` — admin-only irreversible governance renouncement (SC-065)
- `is_paused` — query if the contract is paused
- `get_pause_info` — query pause reason, timestamp, and initiator metadata
- `list_configs` — read all severity configurations as a Map
- `get_last_config_update` — cheap invalidation check returning metadata (ledger sequence) on the most recent configuration update (#4)
- `get_failure_schema` — returns the full catalogue of typed failure codes mapping numeric error codes to machine-readable labels and descriptions (SC-W5-046)
- `get_config_bundle` — combines configuration snapshot and result schema in a single read for one-shot backend bootstrapping (#1)
- `get_contract_metadata` — returns static contract capabilities including features, supported severities, storage/result schema versions (SC-060)
- `prune_history_by_age` — admin-only history compaction removing entries older than a specified duration (SC-063)
- Paginated history access: `get_history_page` returning bounded history page (SC-059)
- `get_history_by_outage` — query all history entries matching a given outage identifier (SC-060)
- `get_latest_by_outage` — query the most recent history entry for a given outage identifier (SC-061)
- `get_config_count` — read total number of configured severity tiers (SC-079)
- `get_storage_version` — query the current storage version stamped in storage
- Configurable retention limit functions: `set_retention_limit` and `get_retention_limit` (SC-013)
- `get_migration_state` — query storage version and migration posture (SC-021)
- `get_version_info` — version negotiation snapshot for backend startup handshake (SC-W5-029)
- Event Correlation IDs — cross-contract tracing via deterministic correlation IDs generated by `generate_correlation_id` from ledger sequence and formatted with `correlation_event_topics` (SC-W5-079)
- Settlement Intent Event (`set_int`) — Published on every `calculate_sla` call alongside `sla_calc` event for backend reconciliation. It uses topics `(set_int, v1, severity)` and payload `(outage_id: Symbol, status: Symbol, payment_type: Symbol, amount: i128, config_version_hash: u64, recorded_at: u64)` (SC-W5-041)
- `docs/AUDIT_TRAIL.md` — human-readable one-pager cataloguing every event topic, payload field, emission site, and backend recovery implication, sourced directly from `event_schema.rs` and the `EVENT_*` constants in `lib.rs` (closes #106)
- `docs/PUBLIC_FUNCTION_DOC_POLICY.md` (SC-102) — repository-level policy enforcing doc comments on all public items with compile-time enforcement via `#![deny(missing_docs)]` (closes #214)
- `docs/UPGRADE_REVIEW_CHECKLIST.md` (SC-103) — admin-facing checklist for safely reviewing contract upgrade proposals (closes #212)
- `docs/SECURITY_REVIEW_TEMPLATE.md` (SC-104) — standardised security review template for new contract modules (closes #210)

### Changed
- `pause` now requires a `reason: String` parameter, records pause metadata (reason, timestamp, initiator), and emits an event payload with the paused status (breaking)
- `calculate_sla` now:
  - Emits a settlement intent event (`set_int`) alongside the SLA calculation event
  - Enforces a configurable retention limit (SC-013) and drops the oldest entry when the limit is exceeded (SC-062)
  - Performs configuration parameter validation (SC-W5-046)
- `get_stats` now returns a `SLAStats` struct; callers should use field access rather than tuple destructuring
- History entries returned by `get_history` include `schema_version` for result envelope versioning

---

## [0.3.0] — Operator role and pause controls

### Added
- `set_operator` — admin-only function to update the operator address
- `pause` / `unpause` — admin-only controls; `calculate_sla` panics with `ContractPaused` when paused
- `get_operator` — read the current operator address

### Changed
- `calculate_sla` now requires the `operator` address as the first argument (breaking)
- `SLAError` extended with `ContractPaused = 6`

---

## [0.2.0] — Statistics and history

### Added
- `get_stats` — cumulative totals for calculations, violations, rewards, penalties
- `get_history` — ordered log of recent SLA calculation results
- `prune_history` — admin-only compaction to bound on-chain storage

---

## [0.1.0] — Initial contract surface

### Added
- `initialize(admin, operator)` — one-time setup; stores roles and default severity configs
- `set_config(caller, severity, threshold_minutes, penalty_per_minute, reward_base)` — admin-only config update
- `get_config(severity)` — read a single severity config
- `calculate_sla(caller, outage_id, severity, mttr_minutes)` — operator-gated SLA calculation

---

## Changelog Process

When making an interface-affecting change, follow these steps:

1. **Add an entry** under `[Unreleased]` in the appropriate section (`Added`, `Changed`, `Removed`, `Fixed`)
2. **Use exact function names** as they appear in the contract interface
3. **Mark breaking changes** explicitly with **(breaking)**
4. **On release**, rename `[Unreleased]` to the version tag and date, then open a fresh `[Unreleased]` block

### Change Categories

| Category | Usage |
|----------|-------|
| `Added` | New functions, features, or parameters |
| `Changed` | Modifications to existing behavior (non-breaking) |
| `Fixed` | Bug fixes or corrections |
| `Removed` | Deprecated or deleted functionality |
| `Security` | Vulnerability patches or security improvements |
