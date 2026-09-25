# Test Module Split Plan

> **Issue:** [#657](https://github.com/ApexChainx/ApexChainx-Contracts/issues/657)

## Problem

`tests.rs` has grown to ~10k lines covering contract-level assertions across
many unrelated features. Large monolithic test files slow failure localization,
bloat review diffs, and make per-module coverage attribution difficult.

## Existing focused modules (pattern to follow)

| File | Coverage area |
|---|---|
| `auth_matrix_tests.rs` | Role authorization matrix |
| `event_ordering_tests.rs` | Event emission ordering |
| `event_state_tests.rs` | Event + state consistency |
| `schema_migration_tests.rs` | Storage schema version guard |
| `parity_tests.rs` | Canonical golden vector parity |
| `history_invariant_tests.rs` | HISTORY_LEN_KEY cache invariant |
| `severity_curve_tests.rs` | Severity curve boundary fixtures |

## Proposed split targets from `tests.rs`

| New file | Sections to extract |
|---|---|
| `config_validation_tests.rs` | `set_config` bounds, cross-severity ordering, freeze gating |
| `retention_tests.rs` | Retention limit, `prune_history`, `prune_history_by_age` |
| `governance_tests.rs` | Admin/operator two-step handoff, expiry, renounce |
| `calculation_tests.rs` | `calculate_sla` happy path, duplicate detection, recalc cap |
| `pagination_tests.rs` | `get_history_page`, `get_history_page_with_meta`, edge cases |
| `initialization_tests.rs` | `initialize`, version checks, storage migration |

## Completion criteria

- Each new file follows the `#[cfg(test)] mod <name>_tests { ... }` pattern.
- `tests.rs` retains only thin cross-cutting integration smoke tests.
- `cargo test` passes with identical coverage after the split.
- Each module is importable individually for targeted `cargo test <module>` runs.