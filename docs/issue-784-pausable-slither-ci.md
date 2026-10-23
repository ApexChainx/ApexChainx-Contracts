# Issue #784: Add CI step to run slither on Pausable

**Status:** Implemented via `.github/workflows/pausable-slither.yml`

## Overview

This issue requests a CI step to run Slither static analysis on the Pausable module.
The `apexchainx_calculator` contract is a **Soroban / Rust** crate, not a Solidity
contract, so Slither (a Solidity-only analyser) cannot run against it directly.

This PR provides the equivalent security-analysis gate using Rust-native tools:

| Slither role | Rust equivalent in this workflow |
|---|---|
| Static vulnerability scanning | `cargo clippy` with arithmetic, conversion, and unwrap lints |
| Unsafe code detection | `cargo-geiger` — reports any `unsafe` blocks in the crate |
| Focused module validation | `cargo test --lib pausable_unit_tests::` — pause lifecycle tests |

## Workflow file

`.github/workflows/pausable-slither.yml`

The workflow triggers on PRs touching:
- `apexchainx_calculator/src/lib.rs` (pause/unpause implementation)
- `apexchainx_calculator/src/tests.rs` (pause-lifecycle tests)
- `apexchainx_calculator/src/cross_contract_safety.rs`
- `apexchainx_calculator/src/audit_state.rs`
- the workflow file itself

## Jobs

1. **clippy-pausable-security** — runs `cargo clippy` with `-D clippy::integer_arithmetic`,
   `-D clippy::unwrap_used`, `-D clippy::panic`, and related security lints. Uploads a
   lint report as a CI artifact.

2. **geiger-pausable-audit** — runs `cargo-geiger` to detect any `unsafe` blocks
   introduced into the crate, failing the job if any are found in crate-local source.

3. **pausable-tests** — runs `cargo test --lib pausable_unit_tests::` as a targeted
   gate, confirming pause lifecycle coverage passes independently of the full suite.

## Related docs

- `docs/REENTRANCY_GUARD_AND_PAUSABLE_REVIEW.md` — broader Pausable review
- `docs/REENTRANCY_PAUSABLE_AND_UPGRADE_AUDIT.md` — audit record
- `.github/workflows/governance-slither.yml` — pattern this workflow follows

Fixes #784
