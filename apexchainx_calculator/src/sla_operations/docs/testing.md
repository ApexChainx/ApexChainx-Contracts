# SLA Operations Workspace — Testing Strategy

Resolves: https://github.com/ApexChainx/ApexChainx-Contracts/issues/1012

## Overview

This guide explains how to write, organise, and run tests for modules in the
`sla_operations` workspace. All tests here are deterministic, `no_std`-compatible,
and isolated from core contract state.

---

## Test directory layout

```
apexchainx_calculator/src/sla_operations/tests/
├── mod.rs                                     — test root (cfg(test) gated)
├── policy_snapshots_fixtures_tests.rs         — tests for fixtures/policy_snapshots.rs
└── report_expectations_fixtures_tests.rs      — tests for fixtures/report_expectations.rs
```

New test files follow the convention `<module_under_test>_tests.rs`.

---

## Running the tests

All commands run from the **workspace root** (`ApexChainx-Contracts/`).

### Run only the `sla_operations` tests

```bash
cargo test -p apexchainx_calculator sla_operations
```

The filter `sla_operations` matches any test whose full path contains that string,
covering every module under `src/sla_operations/tests/`.

### Run all tests in the calculator crate

```bash
cargo test -p apexchainx_calculator
```

### Run with output printed (useful during development)

```bash
cargo test -p apexchainx_calculator sla_operations -- --nocapture
```

### Run a single test by name

```bash
cargo test -p apexchainx_calculator test_default_snapshot_has_four_entries
```

---

## Writing new tests

### File placement

| What you are testing | Where to put the test file |
|----------------------|---------------------------|
| A fixture module     | `sla_operations/tests/<fixture_name>_tests.rs` |
| A helper/util module | `sla_operations/tests/<util_name>_tests.rs` |

### Module registration

After creating a new test file, register it in `sla_operations/tests/mod.rs`:

```rust
#[cfg(test)]
pub mod my_fixture_tests;
```

### Template

```rust
//! Tests for `sla_operations::fixtures::my_module`.
#![cfg(test)]

use soroban_sdk::Env;
use crate::sla_operations::fixtures::my_module::my_fixture_fn;

#[test]
fn test_my_fixture_is_deterministic() {
    let env = Env::default();
    let a = my_fixture_fn(&env);
    let b = my_fixture_fn(&env);
    assert_eq!(a, b);
}
```

---

## Determinism requirements

Every test in this workspace must be **fully deterministic**:

- Do not read the system clock or ledger sequence directly.
- Do not use randomness.
- Do not call `env.storage()` from fixture constructors — fixtures are pure
  data constructors that accept `&Env` only to construct `soroban_sdk`
  types (e.g. `Vec::new(&env)`, `symbol_short!(...)`).
- If a test needs a contract client, use `Env::default()` and register the
  contract with a fixed address via `env.register_contract(None, ...)`.

---

## Isolation requirements

Workspace tests must be isolated from core contract state:

- Tests in `sla_operations/tests/` may import public types from `lib.rs`
  (`SLAConfig`, `SLAResult`, `SLAConfigSnapshot`, etc.) but must not invoke
  core contract entrypoints unless testing integration between the workspace
  and the contract.
- Fixtures must not carry on-chain side effects between tests. Each test
  constructs its own `Env::default()`.
- Avoid sharing mutable state at module level.

---

## Coverage expectations

Each fixture module should have tests covering at least:

| Scenario | Example assertion |
|----------|-------------------|
| Correct field values | `assert_eq!(result.threshold_minutes, 15)` |
| Determinism | Two constructions from same `Env` are `==` |
| Boundary conditions | `mttr == threshold` for "met at boundary" fixtures |
| Negative / positive amounts | Violations → negative, rewards → positive |
| PartialEq distinguishability | Two different fixtures are `!=` |

---

## Linting and type checks

Before opening a PR, verify the workspace compiles and tests pass:

```bash
# Type-check without running tests
cargo check -p apexchainx_calculator

# Run clippy
cargo clippy -p apexchainx_calculator -- -D warnings

# Run tests
cargo test -p apexchainx_calculator sla_operations
```

---

## Constraints summary

| Constraint | Rule |
|-----------|------|
| `no_std` | No `std::` imports; use `soroban_sdk` primitives only. |
| Determinism | No randomness, no I/O, no direct time/ledger reads. |
| Isolation | No side effects between tests; use `Env::default()` per test. |
| Scope | Stay within `sla_operations/**`; do not modify core modules. |
| ABI stability | Do not add, remove, or reorder fields on `#[contracttype]` structs. |
