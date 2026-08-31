# Changelog

> All interface-affecting changes to `apexchainx-contracts` are recorded here.
> This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
> and follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) conventions.

---

## [Unreleased]

### Changed
- **The event catalog is no longer exempted from dead-code detection** (#496). `event_schema` is a public module and its module-level `#![allow(dead_code)]` was removed, so an event constant that is added to the schema but never emitted can no longer pass CI under a blanket lint. `api_stability::event_name_symbols()` now includes `cfg_rem` (emitted by `remove_custom_severity` but still missing after #419), completing the catalog at 23 emitted events (22 on upstream + `cfg_rem`); `assess_stability()` and its regression tests were updated to the new count
- **Event version bumps are now a coordinated release** (#497). `event_schema::EVENT_ABI_GENERATION` and `EVENT_ABI_TO_SCHEMA_VERSION` tie the event ABI symbol (`v1`, `v2`, …) to the required `STORAGE_VERSION` / `RESULT_SCHEMA_VERSION` posture, documented in `docs/UPGRADE_PLAYBOOK.md`; `contract_info::test_event_abi_cobump_invariant` fails CI if a breaking event change rides along on an unchanged schema posture
- **`renounce_admin` now invalidates a pending operator proposal.** Previously renounce cleared the admin and pending-admin slots but left `PENDING_OP_KEY` untouched, so a stale operator handoff could still complete on an adminless contract. Renounce now clears the pending operator slot (and its timestamp) and emits an `op_can` event when one was pending, so no pending proposal can complete a role change after renounce (#470)
- **Removed the orphaned `history_snapshot` module.** `NormalizedSnapshot`/`normalize_history` were documented as a dashboard aggregate but were never exposed by a contract method and were not `#[contracttype]`, so no backend could obtain them. The dashboard surface is now explicitly `get_stats` (cumulative aggregate) alongside `get_severity_telemetry` (per-severity weekly windows); the module, its `lib.rs` registration, and its ownership list entries were removed (#467)
- **`set_operator` now invalidates any pending operator proposal.** A direct, admin-only operator assignment previously left `PENDING_OP_KEY` intact, so a stale handoff could later complete and silently override the admin's single-step decision. `set_operator` now clears the pending operator slot (and its expiry timestamp) and emits an `op_can` event when one was pending, so the two paths can no longer conflict (#469)
- **Re-proposing a role now emits a distinct supersession event instead of silently overwriting the pending candidate.** `propose_admin`/`propose_operator` previously replaced any pending proposal with no signal that a replacement occurred, so an auditor watching the event stream saw isolated proposals and could not distinguish "cancelled then re-proposed" from "silently replaced". Both now emit an additive `adm_sup`/`op_sup` event carrying `(superseded, replacement)` before the new `adm_prop`/`op_prop`, so the pending-slot history is fully reconstructable (#468)
- **Fuzz targets now assert the contract's documented semantics, not just panic-freedom.** `compute_result` and `validate_config` previously ran the function and let libFuzzer watch for a crash; on code guarded throughout by `checked_mul`/`checked_neg` that finds essentially nothing, and a semantic regression (e.g. treating `mttr == threshold` as a violation) would have left the nightly job green. Both targets now compare every input against `apexchainx_calculator::spec` and fail on any disagreement. Each target header states what it asserts and what it does not; `docs/FUZZING_GUARANTEES.md` states the suite-wide guarantees and the policy for resolving an implementation-vs-documentation conflict
- **`ts/historyPagination.ts` capped pages at 50 where the contract caps at 200** (`history::MAX_PAGE_SIZE`, #409), so a backend paging with `limit = 200` received 50 entries and — because the mirror also derived `hasMore` from the returned length — could conclude history had ended. It also coerced `limit = 0` up to 1, returning an entry where the contract returns an empty page, and reported `hasMore: false` where the contract reports `true`. The helper now imports the contract-generated `MAX_PAGE_SIZE` and mirrors `end = min(offset + limit, total)` / `hasMore = end < total` exactly
- **`ts/configVersionHash.ts` computed an unrelated hash.** It ran djb2 over a canonical JSON serialisation of a snapshot whose fields (`penaltyBps`, `rewardBps`) do not exist on the contract, so a backend comparing it against `get_config_version_hash` would have seen a mismatch on every call. It now reproduces the contract's polynomial rolling hash exactly, in `BigInt` `u64` arithmetic, and is asserted equal to a contract-recorded value
- `docs/HISTORY_PAGINATION_POLICY.md` stated that `limit` is **not** clamped — behaviour from before #409 added the `MAX_PAGE_SIZE` clamp. Corrected, along with the end-of-history signalling guidance that followed from it (a short page is only an unambiguous end-of-history signal when `limit <= MAX_PAGE_SIZE`)
- `.github/workflows/fuzz.yml` no longer swallows the fuzz run's exit code with `2>/dev/null || true`. Building is now a separate step that fails loudly, and a libFuzzer finding fails the job instead of being discarded; the corpus is only persisted back to the repo from a run that actually succeeded
- `ts/historyByOutage.ts` and `ts/historyPruneByAge.ts` gained direct mirrors of `get_latest_by_outage` and `prune_history_by_age`; the per-file `require.main === module` self-tests they carried were removed in favour of the parity suite (they asserted each helper against itself, which is what let the defects above survive)
- `storage_version.rs` is now declared as a module (`pub mod storage_version;`) and wired into the crate; `is_migration_complete()` now reads the crate's own `STORAGE_VERSION_KEY`/`STORAGE_VERSION` (removing the duplicate `"VER"`-keyed constant it previously redeclared) and correctly reports whether on-chain storage matches the version this binary expects (#421)
- `api_stability::event_name_symbols()` now covers all 20 event names declared in `event_schema.rs` (previously missing `cfg_frz`, `cfg_unfrz`, `stats_sat`, `migrate_done`); `assess_stability()` and its regression tests were updated to the new count, and a new test cross-checks the guardrail list against `event_schema`'s constants (#419)
- `api_stability::canonical_field_counts()` now covers `ConfigBundle`, `AuditState`, `ContractInfo`, `HistoryPage`, `PublicApiMethod`, `PublicApiDescriptor`, `SeverityAliasMapping`, `ContractStateFingerprint`, `VersionNegotiationInfo`, `NegotiationOutcome`, `VersionMismatchDetail`, `VersionNegotiationResult`, `HistoryRetentionMetrics`, and `CompensationAction` (previously only the 17 types present at #225); `assess_stability()` and its tests were updated to the new count of 31, with dedicated regression assertions added for `ContractInfo` and `AuditState` (#420)
- `get_public_api()` now includes `get_contract_info`, `get_storage_footprint_estimate`, and `get_rent_estimate` in its returned descriptor (previously missing, so backends discovering the API surface via `get_public_api` never learned these methods existed); method count updated from 58 to 61 (#418)
- `test_storage_key_namespace_symbols_are_distinct` now covers all 17 on-chain storage key constants (previously omitted `SEVERITY_CALC_COUNTS_KEY`, `SEVERITY_VIOL_COUNTS_KEY`, `LAST_CALCULATION_LEDGER_KEY`, `LAST_VIOLATION_LEDGER_KEY`, and `LAST_CFG_UPDATE_KEY`). The assertion now includes the colliding indices in its error message for faster diagnosis. A maintenance comment listing every key and a pointer to this test was added to both the storage-key block in `lib.rs` and the test itself so future contributors know to update both locations when adding a new key.
### Fixed
- **Tests compiled on the base branch but did not build a runnable suite** (three pre-existing errors): a stale `use crate::event::CalculationExecutedEventV1;` reference to a type removed in the event-publication refactor, a `history.entries` field access on a `Vec<SLAResult>`, and a `client.pause(&admin)` call missing its `reason` argument. The stale import/round-trip block was removed, the field access was corrected to `history.len()`, and the pause reason was supplied so `cargo test --lib` builds
- The `compute_result` and `validate_config` fuzz targets **did not compile**. Both called `SLACalculatorContract::compute_result` (private) and `SLACalculatorContract::validate_config` (`pub(crate)`) from outside the crate, and `validate_config.rs` additionally passed a `u32` where an `i128` was expected. This went unnoticed because `cargo fuzz run <target>` builds only the named target — CI's `fuzz-regression` job builds `config_mutation_sequences` alone — and the nightly job discarded its own exit code. CI now runs `cargo fuzz build` (all targets) on every PR
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
- **Governance fuzz target** (`governance_sequences`) (#498) — drives random propose/accept/cancel/renounce sequences through the real contract Env and asserts the pending-slot invariants (only the named proposed address can accept; a successful accept/cancel/renounce clears the pending slot). Added to `fuzz/Cargo.toml` and the nightly `fuzz.yml` matrix
- **History state-machine fuzz target** (`history_state_machine`) (#498) — asserts retention (a successful `calculate_sla` never exceeds the limit, `prune_history(N)` bounds to `N`), age-prune, and offset-based pagination invariants, plus a full page-walk reconstruction. Added to `fuzz/Cargo.toml` and the nightly `fuzz.yml` matrix
- **stats_sat ordering test** (#495) — `event_ordering_tests::test_stats_sat_precedes_sla_calc_and_set_int_for_same_calculation` pins that the saturation signal precedes the `sla_calc`/`set_int` decision events for the same calculation and asserts the pre-cap payload, so indexers can rely on the saturation signal's position
- **Event emit-site audit** (#496) — `event_schema::test_every_declared_event_has_an_emit_site` fails when an event constant declared in the catalog has no publish site in `src/`, and `event_schema::test_event_abi_generation_tracks_version_symbol` pins generation 1 == "v1"
- `apexchainx_calculator::spec` — an executable restatement of the contract's documented pure semantics (config validation with its error precedence, the SLA outcome rules, the pagination oracle). Deliberately independent of the implementation: `impl == spec` is only a meaningful assertion when the two are written separately. This is the single place the invariants live; tests, the fuzz targets and the docs all reference it instead of re-deriving them
- `apexchainx_calculator::fuzz_spec` — the assertion bodies the cargo-fuzz targets call. Keeping them in the library means `cargo test --lib` type-checks them and their unit tests pin the boundary vectors on the stable toolchain, so the fuzz suite's contract is enforced by `just test` rather than only by a nightly job that needs nightly, `cargo-fuzz` and a C++ toolchain. It also lets a target reach `compute_result`/`validate_config` without widening them to `pub`, which would have added them to the deployed contract ABI
- Contract-derived fixtures for the `ts/` mirrors — `apexchainx_calculator/src/ts_parity_fixtures.rs` executes the real contract in a Soroban `Env` and writes `ts/generated/contractConstants.ts` (constants, symbol vocabulary, event topic names) and `ts/fixtures/contract-read-semantics.json` (250 recorded SLA results, 14 pagination probes, per-outage lookups, age-prune probes, a config version hash). Both are committed and regenerated by `cargo test`
- `ts/parity/readSemanticsParity.test.ts` — replays those contract-recorded inputs through the TypeScript helpers. No expectation in it is hand-written
- `ts/contractSemantics.ts` — the single point at which contract facts enter TypeScript, re-exporting the generated module so helpers need no filesystem access
- CI job `TS Parity (contract-derived fixtures)` — regenerates the artefacts, fails on an uncommitted diff (a Rust change that never reached TypeScript), then runs the parity suite. A read-semantics change that misses the mirrors cannot go green in either order
- `docs/FUZZING_GUARANTEES.md` — what the fuzz suite guarantees, what it explicitly does not, where the spec lives, and which statement wins when the implementation and a document disagree
- `docs/TS_PARITY_CONTRACT.md` — the in-contract surface `ts/` must mirror, the helpers that are explicitly out of contract and why, and the drift this replaced
- `just` recipes `fuzz-spec`, `fuzz-build`, `fuzz-run`, `ts-fixtures`, `ts-parity` and `ts-check`; `just ci` now includes `fuzz-spec` and `ts-check`
- `get_history_page_with_meta` — paginated history read that returns a `HistoryPage` struct (`items`, `total`, `has_more`) so consumers can detect end-of-history and total size in one read without a separate `get_history` call; `get_history_page` is unchanged (#380)
- Per-severity telemetry counter saturation regression coverage — documented the `u32` lane saturation behavior in the `record_severity_telemetry` code docs and `docs/CONTRACT_MAINTENANCE_POLICY.md`, and added `test_severity_telemetry_counters_saturate_at_u32_max` to verify counters saturate at `u32::MAX` instead of wrapping (release) or panicking (debug) (#387)
- `docs/CONTRACT_SHAPE_CHANGE_CHECKLIST.md` — release-readiness checklist for PRs that touch storage keys, `STORAGE_VERSION`, event topic constants, or event payload fields; cross-referenced from `CONTRIBUTING.md` as SC-100
- **[SC-509] SLAError Addition Workflow** (#253) — comprehensive contributor guide for adding, deprecating, or reviewing `SLAError` variants without breaking backend adapter logic. See `docs/sla-error-additions-guide.md`.
- `error_responses::is_severity_not_in_set` — typed helper predicate for `SLAError::SeverityNotInSet` (#253)
- `docs/sla-error-additions-guide.md` — step-by-step guide covering SLAError enum management, the typed helper layer, compatibility expectations, and testing requirements (#253)- `docs/CONTRACT_MAINTENANCE_POLICY.md` — comprehensive maintenance policy covering `#[contracttype]` compatibility notes (#279), response-shape stability (#283), version negotiation (#284), API archetypes (#285), event payload size checks (#286), event drift review (#287), history write audit (#288), telemetry counters (#289), and role-change incident review (#290)
- `docs/CONTRACT_LIFECYCLE.md` — Mermaid state-transition diagrams for the `apexchainx_calculator` contract lifecycle: top-level lifecycle, pause/unpause, storage migration, config-freeze, admin transfer (two-step), and operator handoff flows; plus the combined orthogonal state matrix and invariants table (closes #256)- `docs/CONTRACT_MAINTENANCE_POLICY.md` — comprehensive maintenance policy covering `#[contracttype]` compatibility notes (#279), response-shape stability (#283), version negotiation (#284), API archetypes (#285), event payload size checks (#286), event drift review (#287), history write audit (#288), telemetry counters (#289), and role-change incident review (#290)- `tooling/release-summary.ts` — release summary generator for maintainers (#280)
- CI release-tests job — runs `cargo test --release --lib` to catch release-mode-only regressions (#90)
- `api_stability` module — maintainer-facing stability scoring for contract types, error codes, event symbols, and storage keys (#225)
- Concurrency policy tests for `calculate_sla` — verifies deterministic idempotency, contradictory-input rejection, config-change reset, and anti-spam recalc limit (#221)
- Retryable vs terminal error classification harness — maps every `SLAError` variant to retryable or terminal category with stability assertions (#227)
- `docs/CONTRACT_MAINTENANCE_POLICY.md` — comprehensive maintenance policy covering `#[contracttype]` compatibility notes (#279), response-shape stability (#283), version negotiation (#284), API archetypes (#285), event payload size checks (#286), event drift review (#287), history write audit (#288), telemetry counters (#289), and role-change incident review (#290)
- `tooling/release-summary.ts` — release summary generator for maintainers (#280)- `.devcontainer/` — reproducible dev container workspace with Rust + WASM target + just + Node.js (#281)
- `just bootstrap` target — session-safe, idempotent one-command local bootstrap for the Rust WASM contract workflow: verifies rustup, installs the pinned `1.94.1` toolchain with `rustfmt` + `clippy` components, adds `wasm32-unknown-unknown` target, and verifies `cargo` is on `PATH` (closes #257)
- `docs/CONTRACT_MAINTENANCE_POLICY.md` — comprehensive maintenance policy covering `#[contracttype]` compatibility notes (#279), response-shape stability (#283), version negotiation (#284), API archetypes (#285), event payload size checks (#286), event drift review (#287), history write audit (#288), telemetry counters (#289), and role-change incident review (#290)
- `docs/EVENT_DRIFT_CHECKLIST.md` — standalone quick-reference event drift review checklist for everyday maintainer use (#287)
- `RESULT_SCHEMA_FIELD_COUNT` constant — compile-time sentinel recording the number of named fields in `SLAResult`; must be updated alongside `RESULT_SCHEMA_VERSION` when the result layout changes (#255)
- `SLAResultSchema::result_field_count` — exposes `RESULT_SCHEMA_FIELD_COUNT` to backend consumers via `get_result_schema()` so they can detect layout drift at runtime (#255)
- `schema_migration_tests.rs` — CI-backed guardrail tests for `get_result_schema()`: exhaustive `SLAResult` destructure (compile-time gate), field count sentinel, symbol stability, deprecated-symbols invariant, and `get_config_bundle` consistency (closes #255)
- `docs/result-schema-migration-guard.md` — documentation for the result schema migration process, describing the two-level guardrail, step-by-step change process, and backend consumer guidance (closes #255)- `tooling/release-summary.ts` — release summary generator for maintainers (#280)
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
- `get_contract_state_fingerprint` no longer reports `config_version_hash = 0` when the config is unreadable. Previously an unreadable `CONFIG_KEY` (storage corruption, bad migration) was masked with `unwrap_or(0)`, making a corrupt config indistinguishable from a legitimate hash — which is never 0 — and defeating the endpoint's incident-response purpose. A config-hash failure now propagates as `NotInitialized`/`ConfigNotFound`, and the documented error contract was updated to match; tests cover the missing-key and partially-missing-config paths (#494)

### Added
- `scripts/check-orphan-modules.sh` — orphan-module lint that fails when a `.rs` file under the crate is never declared as a module (an undeclared file is invisible to cargo check/clippy/test, so its tests rot silently). Wired into CI as an early gate and available as `just lint-orphans`. The leftover `apexchainx_calculator/scratch_test.rs` was removed as part of this (#491)
- `get_public_api` descriptor completeness guardrail — `test_492_descriptor_covers_every_public_method` and `test_492_descriptor_lists_only_declared_methods` compare the hand-enumerated descriptor against the canonical `CANONICAL_PUBLIC_METHODS` list in both directions, so a public method missing from the descriptor or a descriptor entry that no longer exists on the impl fails CI (#492)
- `AuditState.pause_info` invariant enforcement — a test pins the `Vec`-stands-in-for-`Option` convention (empty when unpaused, exactly one element when paused, always matching `get_pause_info`), and `CODING_STYLE.md` Part 3 documents the single convention governing absent state across `#[contracttype]` surfaces; Part 4 documents the module-declaration and scratch-code policy (#493)

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
