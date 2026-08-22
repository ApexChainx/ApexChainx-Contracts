# Module Ownership Map

> **Issue:** [#249](https://github.com/ApexChainx/ApexChainx-Contracts/issues/249)
> **Status:** Active
> **Applies to:** All modules, directories, and workflows in this repository

This document maps every module, directory, and workflow in the repository to
its ownership domain so that pull requests can be routed to the correct
reviewers and merge bottlenecks are reduced.

---

## Table of Contents

- [1. Ownership Domains](#1-ownership-domains)
- [2. Contract Crate Modules](#2-contract-crate-modules)
- [3. Documentation](#3-documentation)
- [4. CI & Workflows](#4-ci--workflows)
- [5. Tooling & Scripts](#5-tooling--scripts)
- [6. TypeScript Off-Chain Tests](#6-typescript-off-chain-tests)
- [7. Review Routing Guide](#7-review-routing-guide)

---

## 1. Ownership Domains

| Domain | Conceptual Review Group | Scope |
|--------|------------------------|-------|
| **Contract Core** | Contract Core reviewers | `apexchainx_calculator/src/lib.rs`, `calculation.rs`, `config.rs`, core types and entrypoints |
| **Contract Governance** | Contract Core reviewers | `governance.rs`, `config_freeze.rs`, role management, pause/unpause |
| **Contract Infrastructure** | Contract Core reviewers | `storage_version.rs`, `version_negotiation.rs`, `deployment_policy.rs`, `cross_contract_safety.rs` |
| **Contract Data Layer** | Contract Core reviewers | `history.rs`, `history_snapshot.rs`, `config_metadata.rs`, `config_bundle.rs`, `metadata.rs` |
| **Event System** | Contract Core reviewers | `event.rs`, `event_schema.rs`, `event_correlation.rs`, event test modules |
| **Audit & Telemetry** | Contract Core reviewers | `audit_state.rs`, `error_responses.rs` |
| **Testing** | Contract Core reviewers | `tests.rs`, `fuzz_tests.rs`, fuzz targets, property tests |
| **Docs** | Docs reviewers | All `docs/*.md` files |
| **CI/CD** | DevOps reviewers | `.github/workflows/*.yml`, `codecov.yml`, CI configuration |
| **Tooling** | DevOps reviewers | `tooling/*.ts`, `tools/*.ts`, `scripts/*` |
| **TypeScript Tests** | Backend reviewers | `tests/*.test.ts`, `ts/*.ts` |
| **Off-Chain Metadata** | Backend reviewers | `offchain/*.ts` |
| **Root Config** | DevOps reviewers | `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `justfile`, `codecov.yml` |

---

## 2. Contract Crate Modules

All paths are relative to `apexchainx_calculator/src/`.

### 2.1 Core Entrypoint (`lib.rs`)

| File | Owner | Risk Level | Review Notes |
|------|-------|-----------|-------------|
| `lib.rs` | Contract Core | **Critical** | Every PR touching this file needs at least 2 reviewers. Contains all public entrypoints, storage keys, error codes, and core data types. |

### 2.2 Domain Modules

| File | Owner | Public Functions | Risk Level |
|------|-------|-----------------|-----------|
| `calculation.rs` | Contract Core | `calculate_sla`, `calculate_sla_view`, `compute_result`, `get_stats`, `get_severity_telemetry`, `increment_stats`, `record_severity_telemetry` | **Critical** |
| `config.rs` | Contract Core | `set_config`, `get_config`, `get_config_snapshot`, `list_configs`, `get_config_version_hash`, `get_last_config_update`, `set_custom_severity`, `remove_custom_severity`, `get_custom_severity`, `get_custom_config_snapshot` | **High** |
| `governance.rs` | Contract Governance | `set_operator`, `propose_admin`, `accept_admin`, `cancel_admin_proposal`, `get_pending_admin`, `propose_operator`, `accept_operator`, `cancel_operator_proposal`, `get_pending_operator`, `renounce_admin` | **High** |
| `config_freeze.rs` | Contract Governance | `freeze_config`, `unfreeze_config`, `is_config_frozen` | **Medium** |
| `metadata.rs` | Contract Governance | `pause`, `unpause`, `is_paused`, `get_pause_info`, `require_not_paused` | **High** |
| `history.rs` | Contract Data Layer | `get_history`, `prune_history`, `prune_history_by_age`, `get_history_page`, `get_history_page_with_meta`, `get_history_by_outage`, `get_latest_by_outage`, `get_config_count`, `set_retention_limit`, `get_retention_limit` | **High** |
| `history_snapshot.rs` | Contract Data Layer | `normalize_history` | **Medium** |
| `config_metadata.rs` | Contract Data Layer | `record_config_update`, `get_last_config_update` | **Medium** |
| `config_bundle.rs` | Contract Data Layer | (composed types for `get_config_bundle`) | **Low** |
| `audit_state.rs` | Audit & Telemetry | (composed types for `get_full_audit_state`) | **Low** |
| `error_responses.rs` | Audit & Telemetry | `is_already_initialized` through `is_outage_recalc_limit` | **Low** |

### 2.3 Infrastructure Modules

| File | Owner | Public Functions | Risk Level |
|------|-------|-----------------|-----------|
| `storage_version.rs` | Contract Infrastructure | `read_storage_version`, `is_migration_complete` | **High** |
| `version_negotiation.rs` | Contract Infrastructure | `build_negotiation_info`, `negotiate_contract_versions`, `version_discovery_interfaces` | **High** |
| `cross_contract_safety.rs` | Contract Infrastructure | `safe_invoke_contract`, `requires_rollback`, `CrossContractCallStack` | **High** |
| `deployment_policy.rs` | Contract Infrastructure | `verify_deployment_compatibility` | **Medium** |
| `payload_optimizer.rs` | Contract Infrastructure | `derive_payment_type`, `is_valid_status`, `is_consistent_payment`, `is_valid_rating` | **Medium** |
| `policy.rs` | Contract Infrastructure | `validate_storage_key` | **Medium** |

### 2.4 Event System

| File | Owner | Public Functions | Risk Level |
|------|-------|-----------------|-----------|
| `event_schema.rs` | Event System | `current_event_version` | **High** |
| `event.rs` | Event System | `publish_calculation_executed` (2 overloads) | **Medium** |
| `event_correlation.rs` | Event System | `generate_correlation_id`, `correlation_event_topics` | **Medium** |
| `event_ordering_tests.rs` | Event System | (test-only) | **Low** |
| `event_state_tests.rs` | Event System | (test-only) | **Low** |
| `topic_stability_tests.rs` | Event System | (test-only) | **Low** |
| `payload_versioning_tests.rs` | Event System | (test-only) | **Low** |

### 2.5 Test Modules

| File | Owner | Risk Level | Review Notes |
|------|-------|-----------|-------------|
| `tests.rs` | Testing | **Low** | Integration tests. Changes must match the production code they test. |
| `fuzz_tests.rs` | Testing | **Low** | Property-based tests. Check that invariants are preserved. |
| `scratch_test.rs` | Testing | **Low** | Experimental / scratch tests. |
| `fuzz/` directory | Testing | **Low** | Cargo-fuzz targets. |

---

## 3. Documentation

All paths relative to repository root.

### 3.1 Core Documentation

| File | Owner | Covers |
|------|-------|--------|
| `README.md` | Docs | Project overview, FAQ, related repos |
| `CONTRIBUTING.md` | Docs | Development workflow, PR guidelines, security review checklists |
| `CODING_STYLE.md` | Docs | Soroban Symbol naming conventions |
| `CODE_OF_CONDUCT.md` | Docs | Community standards |
| `SECURITY.md` | Docs | Vulnerability reporting, binary provenance |
| `CHANGELOG.md` | Docs | Release notes |

### 3.2 Policy & Governance Docs

| File | Owner | Covers |
|------|-------|--------|
| `docs/API_STABILITY_SCORECARD.md` | Docs + Contract Core | API compatibility risk classification |
| `docs/CONTRACT_API_COMPATIBILITY.md` | Docs + Contract Core | Backend adapter verification |
| `docs/EVENT_COMPATIBILITY_POLICY.md` | Docs + Event System | Event schema immutability rules |
| `docs/EVENT_TOPIC_COMPATIBILITY.md` | Docs + Event System | Topic symbol deprecation lifecycle |
| `docs/RELEASE_PROVENANCE_POLICY.md` | Docs + DevOps | WASM provenance and snapshot check-in |
| `docs/RESERVED_KEYS_POLICY.md` | Docs + Contract Core | Storage key prefix reservations |
| `docs/config-validation.md` | Docs + Contract Core | Config validation rules |

### 3.3 Technical & Architecture Docs

| File | Owner | Covers |
|------|-------|--------|
| `docs/PROJECT_CONTEXT.md` | Docs | System architecture and roadmap |
| `docs/CODEX_CONTEXT.md` | Docs | Technical deep-dive for developers |
| `docs/CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md` | Docs + Contract Infrastructure | Deployment compatibility |
| `docs/UPGRADE_PLAYBOOK.md` | Docs + Contract Infrastructure | Storage version upgrade procedures |
| `docs/SNAPSHOT_NORMALIZATION.md` | Docs + Testing | Snapshot normalization rules |
| `docs/AUDIT_TRAIL.md` | Docs + Audit | Event audit trail |
| `docs/RESULT_PAYLOAD_HASHING.md` | Docs + Contract Data Layer | Result payload hashing |
| `docs/BENCHMARK_SEVERITY_COSTS.md` | Docs + Contract Core | Severity cost benchmarks |
| `docs/sc-w5-storage-and-cost-baselines.md` | Docs + Contract Core | Storage and cost baselines |

---

## 4. CI & Workflows

All paths relative to `.github/workflows/`.

| File | Owner | Purpose | Trigger |
|------|-------|---------|---------|
| `ci.yml` | DevOps | Full CI pipeline: format, clippy, no-std, tests, fuzz, provenance | PR → `main`, push → `main` |
| `release-hash.yml` | DevOps | Build WASM, generate SHA-256 manifest, attach to release | Tag `v*` |
| `fuzz.yml` | DevOps + Testing | Standalone fuzz target runs | Scheduled / manual |
| `testnet-deploy.yml` | DevOps | Testnet contract deployment | Scheduled / manual |
| `coverage.yml` | DevOps | Code coverage (codecov) | PR → `main` |
| `security.yml` | DevOps | Security scanning (cargo-audit, secret leaks) | PR → `main` |
| `workflow-lint.yml` | DevOps | Workflow file validation | PR touching `.github/workflows/` |

### 4.1 Root CI Configuration

| File | Owner | Purpose |
|------|-------|---------|
| `codecov.yml` | DevOps | Coverage reporting thresholds |
| `deny.toml` | DevOps | Dependency licence and advisory policy |

---

## 5. Tooling & Scripts

### 5.1 TypeScript Tooling (`tooling/`)

| File | Owner | Purpose |
|------|-------|---------|
| `moduleMap.ts` | Contract Core | Module-to-function ownership map (source of truth) |
| `wasmAttestation.ts` | DevOps | WASM artifact checksum computation and verification |
| `releaseChecklist.ts` | DevOps | Release readiness checklist |
| `deterministicSeedManagement.ts` | Testing | Deterministic seeds for fuzzing |
| `fuzzCiIntegration.ts` | DevOps + Testing | Fuzz CI integration config |
| `governanceSummary.ts` | Contract Governance | Governance event consistency checks |
| `regressionThresholds.ts` | DevOps | Regression threshold configuration |
| `changelogLint.ts` | DevOps | CHANGELOG.md linting |
| `multiStageCiGates.ts` | DevOps | Multi-stage CI gate enforcement |
| `securityGateEnforcement.ts` | DevOps | Security gate checks |

### 5.2 Build Tools (`tools/`)

| File | Owner | Purpose |
|------|-------|---------|
| `secret-leak-check.ts` | DevOps | Secret/key leak detection |
| `link-check.ts` | Docs | Documentation link validation |
| `wasm-size-check.ts` | DevOps | WASM artifact size budget enforcement |
| `security-checklist.ts` | DevOps | Security review checklist automation |
| `normalize-snapshots.ts` | Testing | Snapshot normalization for CI |
| `gen-roadmap.ts` | Docs | ROADMAP.md generation |
| `prune-perf.ts` | Contract Data Layer | History pruning performance bench |

### 5.3 Shell Scripts (`scripts/`)

| File | Owner | Purpose |
|------|-------|---------|
| `normalize-snapshots.js` | Testing | Snapshot normalization (JS version) |
| `bench-prune.ts` | Contract Data Layer | Pruning benchmarks |
| `invoke-examples.ts` | Docs | Contract invocation examples |
| `diff-snapshots.ts` | Testing | Snapshot diff tool |
| `validate-roadmap.ts` | Docs | ROADMAP.md validation |
| `check-no-std.sh` | DevOps | no-std compliance lint |
| `run-tests.ts` | Testing | Test runner script |
| `check-dep-policy.ts` | DevOps | Dependency policy check |
| `security-gate.ts` | DevOps | Security gate enforcement |
| `check-wasm-size.ts` | DevOps | WASM size check |

---

## 6. TypeScript Off-Chain Tests

### 6.1 Integration & Parity Tests (`tests/`)

| Directory / File | Owner | Purpose |
|-----------------|-------|---------|
| `tests/*.test.ts` (all) | Backend | Contract response parity, simulation reproducibility, auth matrix, threshold edge cases, fuzz configs — all verify contract behaviour matches backend expectations |

### 6.2 TypeScript Modules (`ts/`)

| File | Owner | Purpose |
|------|-------|---------|
| `upgradeGuardTests.ts` | Contract Infrastructure | Storage version guard tests |
| `configVersionHash.ts` | Contract Core | Config version hash computation |
| `configUpdateMeta.ts` | Contract Core | Config update metadata |
| `governanceEvents.ts` | Contract Governance | Governance event helpers |
| `historyPagination.ts` | Contract Data Layer | History pagination helpers |
| `historyPruneByAge.ts` | Contract Data Layer | Age-based prune helpers |
| `historyByOutage.ts` | Contract Data Layer | Outage-based history lookup |
| `aggregateReadHelper.ts` | Contract Data Layer | Aggregate read helper |

### 6.3 Off-Chain Metadata (`offchain/`)

| File | Owner | Purpose |
|------|-------|---------|
| `contractMetadata.ts` | Backend | Contract metadata introspection simulation |
| `governanceConsistency.ts` | Backend | Governance state consistency checks |
| `eventSizeRegression.ts` | Backend | Event size regression tests |
| `payoutDisbursementInterface.ts` | Backend | Payout disbursement interface |
| `readCostRegression.ts` | Backend | Read cost regression tests |

---

## 7. Review Routing Guide

### 7.1 When to Request Reviews

| Change touches… | Request review from… |
|----------------|---------------------|
| `lib.rs` (any change) | **2 reviewers** from Contract Core |
| `calculation.rs`, `config.rs` | 1 reviewer from Contract Core |
| `governance.rs`, `config_freeze.rs` | 1 reviewer from Contract Governance |
| `storage_version.rs`, `version_negotiation.rs`, `cross_contract_safety.rs` | 1 reviewer from Contract Infrastructure |
| `history.rs`, `history_snapshot.rs`, `config_metadata.rs` | 1 reviewer from Contract Data Layer |
| `event.rs`, `event_schema.rs`, `event_correlation.rs` | 1 reviewer from Event System |
| Any `.github/workflows/*.yml` | 1 reviewer from DevOps |
| Any `docs/*.md` | 1 reviewer from Docs |
| `CONTRIBUTING.md`, `SECURITY.md`, `README.md` | 1 reviewer from Docs |
| `tests/*.test.ts`, `ts/*.ts` | 1 reviewer from Backend |
| `offchain/*.ts` | 1 reviewer from Backend |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | 1 reviewer from DevOps |
| `tooling/*.ts`, `tools/*.ts`, `scripts/*` | 1 reviewer from DevOps |

### 7.2 High-Risk Module Special Rules

These modules require **additional scrutiny** due to their security or financial
impact:

| Module | Extra Rule |
|--------|-----------|
| `calculation.rs` | Any change must include backend parity tests |
| `governance.rs` | Any auth-related change requires a security review |
| `storage_version.rs` | Must reference the [Upgrade Playbook](docs/UPGRADE_PLAYBOOK.md) and include a migration path |
| `event_schema.rs` | Must follow [Event Compatibility Policy](docs/EVENT_COMPATIBILITY_POLICY.md) and [Event Topic Compatibility](docs/EVENT_TOPIC_COMPATIBILITY.md) |
| `config.rs` | Config validation changes must include cross-severity ordering tests |
| `cross_contract_safety.rs` | Any change requires multi-contract test coverage |

### 7.3 Review Load Balancing

- If a PR touches modules in **3+ ownership domains**, request at least **2
  reviewers** from different domains.
- CI-only changes (workflows, scripts, tooling) can usually be reviewed by a
  single DevOps reviewer.
- Documentation-only changes can be reviewed by a single Docs reviewer unless
  they document a security-sensitive or financial-critical path.

### Related Documents

- [API Stability Scorecard](docs/API_STABILITY_SCORECARD.md) — helps determine
  whether a change is additive or breaking before routing for review
- [Contributing Guide](CONTRIBUTING.md) — general contribution workflow and guidelines
