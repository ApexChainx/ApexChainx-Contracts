# Documentation Index

> **Issue:** [#672](https://github.com/ApexChainx/ApexChainx-Contracts/issues/672)
> **Status:** Active
> **Applies to:** every markdown file in the repository (root and `docs/`)

This page is the **single navigation entry point** for the repository's
documentation. It classifies each file so new contributors can tell a *current
reference* from a *design note* or a *superseded* scratch page, and links each
one to the code-adjacent source of truth it describes.

Legend:

| Status | Meaning |
|---|---|
| **Reference** | Current source of truth. Kept in sync with the code; update it in the same PR as the behaviour it documents. |
| **Design note** | Background/rationale. Still useful context, but not the contract you should code against. |
| **Superseded** | Kept only for history; do not rely on it. Superseded by the linked document. |
| **Archived** | Frozen point-in-time context (e.g. PR/issue write-ups). Never updated. |

---

## Start here

| Document | Status | Covers |
|---|---|---|
| [README.md](../README.md) | **Reference** | Project overview, FAQ, API archetypes, related repos |
| [QUICKSTART.md](QUICKSTART.md) | **Reference** | Developer environment setup and common `just`/npm targets |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | **Reference** | Development workflow, code style, PR checklist |
| [CODING_STYLE.md](../CODING_STYLE.md) | **Reference** | Soroban Symbol naming and Rust conventions |
| [CODE_OF_CONDUCT.md](../CODE_OF_CONDUCT.md) | **Reference** | Community standards |
| [SECURITY.md](../SECURITY.md) | **Reference** | Vulnerability reporting and binary provenance |
| [CHANGELOG.md](../CHANGELOG.md) | **Reference** | Release notes, newest first |

## Architecture & project context

| Document | Status | Covers |
|---|---|---|
| [PROJECT_CONTEXT.md](PROJECT_CONTEXT.md) | **Reference** | System architecture, contract store map, project roadmap |
| [CODEX_CONTEXT.md](CODEX_CONTEXT.md) | **Reference** | Technical deep-dive for contributors |
| [CONTRACT_LIFECYCLE.md](CONTRACT_LIFECYCLE.md) | **Reference** | Init / pause / migrate / freeze / handoff state machine and matrices |
| [MODULE_OWNERSHIP.md](MODULE_OWNERSHIP.md) | **Reference** | Module-to-owner map and review routing (see also [.github/CODEOWNERS](../.github/CODEOWNERS)) |
| [AUTH_MODEL.md](AUTH_MODEL.md) | **Reference** | Role model (admin / operator) and auth boundaries |
| [CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md](CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md) | **Reference** | Multi-contract deployment compatibility checklist |

## Contract behaviour & config

| Document | Status | Covers |
|---|---|---|
| [config-validation.md](config-validation.md) | **Reference** | `set_config` / custom-severity validation rules and error mapping |
| [SEVERITY_CURVES.md](SEVERITY_CURVES.md) | **Reference** | Admissible per-severity config region and cross-severity ordering |
| [severity-compatibility-matrix.md](severity-compatibility-matrix.md) | **Reference** | Severity-axis compatibility classification |
| [SEVERITY_ALIAS_POLICY.md](SEVERITY_ALIAS_POLICY.md) | **Reference** | Severity alias rename/deprecation policy |
| [result-schema-migration-guard.md](result-schema-migration-guard.md) | **Reference** | `RESULT_SCHEMA_VERSION` guard rules |
| [RESULT_PAYLOAD_HASHING.md](RESULT_PAYLOAD_HASHING.md) | **Reference** | Result payload hashing semantics |
| [PUBLIC_FUNCTION_DOC_POLICY.md](PUBLIC_FUNCTION_DOC_POLICY.md) | **Reference** | Doc-comment requirements for public entrypoints |
| [API_MANIFEST.md](API_MANIFEST.md) | **Reference** | Compiled public surface manifest (`get_public_api`) |
| [API_STABILITY_SCORECARD.md](API_STABILITY_SCORECARD.md) | **Reference** | Compatibility risk classification for public entrypoints |
| [CONTRACT_API_COMPATIBILITY.md](CONTRACT_API_COMPATIBILITY.md) | **Reference** | Backend adapter API-compatibility assertions |
| [CONTRACT_SHAPE_CHANGE_CHECKLIST.md](CONTRACT_SHAPE_CHANGE_CHECKLIST.md) | **Reference** | Checklist for shape-changing contract edits |
| [CONTRACT_MAINTENANCE_POLICY.md](CONTRACT_MAINTENANCE_POLICY.md) | **Reference** | Ongoing maintenance rules (retention, pagination, etc.) |
| [ERROR_DOMAIN_CONVENTION.md](ERROR_DOMAIN_CONVENTION.md) | **Reference** | `SLAError` code/domain conventions |
| [sla-error-additions-guide.md](sla-error-additions-guide.md) | **Reference** | How to add a new `SLAError` variant and predicate |
| [FAILURE_TAXONOMY.md](FAILURE_TAXONOMY.md) | **Reference** | Formal failure taxonomy, recovery strategies |

## Events & interoperability

| Document | Status | Covers |
|---|---|---|
| [EVENT_COMPATIBILITY_POLICY.md](EVENT_COMPATIBILITY_POLICY.md) | **Reference** | Event schema immutability and append-only field rules |
| [EVENT_TOPIC_COMPATIBILITY.md](EVENT_TOPIC_COMPATIBILITY.md) | **Reference** | Event-topic namespace and deprecation lifecycle |
| [event-ordering-guarantees.md](event-ordering-guarantees.md) | **Reference** | Deterministic event ordering contract (SC-W5-042) |
| [EVENT_DRIFT_CHECKLIST.md](EVENT_DRIFT_CHECKLIST.md) | **Reference** | Checklist for detecting event drift |
| [OBSERVABILITY_CONTRACT.md](OBSERVABILITY_CONTRACT.md) | **Reference** | What backends may rely on for observability |
| [SETTLEMENT_INTENT.md](SETTLEMENT_INTENT.md) | **Reference** | Settlement-intent vs on-chain-execution boundary (#676) |
| [correlation.md](../correlation.md) | **Design note** | Resolution write-up for correlation-id placement (#565/#566); canonical schema lives in `apexchainx_calculator/src/event_schema.rs` |

## Storage, migrations & retention

| Document | Status | Covers |
|---|---|---|
| [UPGRADE_PLAYBOOK.md](UPGRADE_PLAYBOOK.md) | **Reference** | Storage-version upgrade procedures and migration arms |
| [COMPATIBILITY_TRACKING_MATRIX.md](COMPATIBILITY_TRACKING_MATRIX.md) | **Reference** | Storage/event/API drift tracking across releases |
| [RESERVED_KEYS_POLICY.md](RESERVED_KEYS_POLICY.md) | **Reference** | Reserved instance-storage key namespace |
| [STORAGE_KEY_MIGRATION_CHECKLIST.md](STORAGE_KEY_MIGRATION_CHECKLIST.md) | **Reference** | Checklist for adding/removing a storage key |
| [STORAGE_FOOTPRINT_POLICY.md](STORAGE_FOOTPRINT_POLICY.md) | **Reference** | Storage-footprint estimate policy |
| [sc-w5-storage-and-cost-baselines.md](sc-w5-storage-and-cost-baselines.md) | **Reference** | Storage and cost baselines |
| [MIGRATION_STATE_CONSUMPTION.md](MIGRATION_STATE_CONSUMPTION.md) | **Reference** | How backends consume `get_migration_state()` |
| [HISTORY_PAGINATION_POLICY.md](HISTORY_PAGINATION_POLICY.md) | **Reference** | Pagination contract and end-of-history signalling |
| [HISTORY_CACHE_READ_ENVELOPE.md](HISTORY_CACHE_READ_ENVELOPE.md) | **Reference** | Read-cost envelope for history accessors |
| [HISTORY_PAGE_READ_COST.md](HISTORY_PAGE_READ_COST.md) | **Reference** | Per-page read-cost accounting |
| [PRUNE_INTERACTION.md](PRUNE_INTERACTION.md) | **Reference** | How pruning interacts with reads/retention |
| [PRUNING_BENCHMARK_NOTE.md](PRUNING_BENCHMARK_NOTE.md) | **Reference** | Pruning benchmark notes |
| [retention-benchmark-guidance.md](retention-benchmark-guidance.md) | **Reference** | `MAX_HISTORY_SIZE` and retention-polish guidance |
| [BENCHMARK_SEVERITY_COSTS.md](BENCHMARK_SEVERITY_COSTS.md) | **Reference** | Severity cost benchmarks |
| [SNAPSHOT_NORMALIZATION.md](SNAPSHOT_NORMALIZATION.md) | **Reference** | Snapshot normalization rules for CI |
| [test-fixtures-guide.md](test-fixtures-guide.md) | **Reference** | Snapshot/fixture directories and their lifecycle |

## Governance

| Document | Status | Covers |
|---|---|---|
| [GOVERNANCE_REFERENCE.md](GOVERNANCE_REFERENCE.md) | **Reference** | Governance module reference (proposals, expiry, renounce) |
| [PROPOSAL_EXPIRY_SEMANTICS.md](PROPOSAL_EXPIRY_SEMANTICS.md) | **Reference** | Proposal expiry semantics and events |
| [OPERATOR_ROTATION_GUIDE.md](OPERATOR_ROTATION_GUIDE.md) | **Reference** | Operator handoff runbook |
| [AUDIT_TRAIL.md](AUDIT_TRAIL.md) | **Reference** | Event audit-trail guarantees, incl. config-update actor attribution (#671) |

## Operations, release & tooling

| Document | Status | Covers |
|---|---|---|
| [DeploymentPolicy.md](DeploymentPolicy.md) | **Reference** | Deployment compatibility policy surface |
| [Testnet-deployment.md](Testnet-deployment.md) | **Reference** | Testnet deployment notes |
| [RELEASE_PROVENANCE_POLICY.md](RELEASE_PROVENANCE_POLICY.md) | **Reference** | WASM provenance and snapshot check-in |
| [RELEASE_SUMMARY_FORMAT.md](RELEASE_SUMMARY_FORMAT.md) | **Reference** | Structured release-note format |
| [OFFCHAIN_SCRIPTS.md](OFFCHAIN_SCRIPTS.md) | **Reference** | Off-chain validation scripts and what they check |
| [TS_PARITY_CONTRACT.md](TS_PARITY_CONTRACT.md) | **Reference** | TypeScript ↔ contract parity contract |
| [SC_MARKER_POLICY.md](SC_MARKER_POLICY.md) | **Reference** | `SC-*` marker policy |
| [SECURITY_REVIEW_TEMPLATE.md](SECURITY_REVIEW_TEMPLATE.md) | **Reference** | Security review template |
| [REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md](REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md) | **Reference** | ReentrancyGuard (#768, #763, #766) and Pausable (#770) review |
| [REENTRANCY_PAUSABLE_AND_UPGRADE_AUDIT.md](REENTRANCY_PAUSABLE_AND_UPGRADE_AUDIT.md) | **Reference** | Audit of ReentrancyGuard, ProxyUpgrade storage gap, and Pausable gas optimization (#761, #762, #764, #771) |
| [UPGRADE_AND_REENTRANCY_REVIEW.md](UPGRADE_AND_REENTRANCY_REVIEW.md) | **Reference** | Upgrade, reentrancy, access control and gas review |

## Testing & fuzzing

| Document | Status | Covers |
|---|---|---|
| [FUZZING_GUARANTEES.md](FUZZING_GUARANTEES.md) | **Reference** | Invariants guaranteed by the fuzz targets |
| [FUZZING_TRIAGE.md](FUZZING_TRIAGE.md) | **Reference** | How to triage a fuzz failure |
| [TEST_MODULE_SPLIT_PLAN.md](TEST_MODULE_SPLIT_PLAN.md) | **Reference** | Test-module organisation plan |
| [COVERAGE_MATRIX.md](COVERAGE_MATRIX.md) | **Reference** | Test coverage matrix |

## Archived / historical

These files are frozen point-in-time write-ups. They are kept for provenance
and are **not** updated; where they describe behaviour, the linked **Reference**
document is authoritative.

| Document | Status | Superseded by |
|---|---|---|
| [pr-224-226-228-229-context.md](pr-224-226-228-229-context.md) | **Archived** | [CHANGELOG.md](../CHANGELOG.md) / relevant reference docs |
| [pr-233-230-234-236-context.md](pr-233-230-234-236-context.md) | **Archived** | [CHANGELOG.md](../CHANGELOG.md) |
| [pr-238-235-232-231-context.md](pr-238-235-232-231-context.md) | **Archived** | [CHANGELOG.md](../CHANGELOG.md) |
| [calculator.md](../calculator.md) | **Superseded** | [SEVERITY_CURVES.md](SEVERITY_CURVES.md) / [RESULT_PAYLOAD_HASHING.md](RESULT_PAYLOAD_HASHING.md) |
| [governance.md](../governance.md) | **Superseded** | [GOVERNANCE_REFERENCE.md](GOVERNANCE_REFERENCE.md) |
| [constant.md](../constant.md) | **Superseded** | [API_STABILITY_SCORECARD.md](API_STABILITY_SCORECARD.md) / [RESULT_PAYLOAD_HASHING.md](RESULT_PAYLOAD_HASHING.md) |
| [CONTRACT_FAILURE.md](../CONTRACT_FAILURE.md) | **Superseded** | [FAILURE_TAXONOMY.md](FAILURE_TAXONOMY.md) |
| [Docs-Order.MD](../Docs-Order.MD) | **Superseded** | This index (the custom-severity bundle gap it describes is tracked by its own issue) |

---

## Keeping this index honest

- Adding a new markdown file? Add a row here in the same PR.
- The pinned `//!` module docs in `apexchainx_calculator/src/lib.rs` are the
  code-adjacent source of truth for storage keys, events, and error codes; when
  a doc here disagrees with the source, the source wins and the doc must be
  fixed.
- Root-level design notes are allowed only while they are classified above.
  When a note is fully absorbed into a reference doc, mark it **Superseded**
  (and, once no longer useful, delete it).
