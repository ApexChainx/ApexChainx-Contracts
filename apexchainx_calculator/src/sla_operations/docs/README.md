# SLA Operations Workspace — Architecture and Boundaries

Resolves: https://github.com/ApexChainx/ApexChainx-Contracts/issues/1007

## Purpose

The `sla_operations` namespace is an **isolated, additive workspace** inside
the `apexchainx_calculator` crate. It provides workspace-local fixtures,
helpers, and test utilities for SLA-related modules without touching any
stabilised contract code.

Nothing in this workspace is compiled into the production WASM binary except
items explicitly re-exported through the crate root.

---

## Module layout

```
apexchainx_calculator/src/sla_operations/
├── mod.rs                               — workspace root; registers sub-modules
├── fixtures/
│   ├── mod.rs                           — fixture sub-module root
│   ├── policy_snapshots.rs              — SLAConfigSnapshot test fixtures
│   └── report_expectations.rs           — SLAResult expectation fixtures
├── tests/
│   ├── mod.rs                           — test root (cfg(test) gated)
│   ├── policy_snapshots_fixtures_tests.rs
│   └── report_expectations_fixtures_tests.rs
└── docs/
    ├── README.md                        — this file
    └── testing.md                       — testing strategy guide
```

The workspace is declared in `lib.rs` as:

```rust
pub mod sla_operations;
```

---

## Contribution boundaries

### You MUST

| Rule | Rationale |
|------|-----------|
| Work only within `apexchainx_calculator/src/sla_operations/**` | Keeps the workspace isolated from stabilised contract code. |
| Use only `soroban_sdk` primitives (`Symbol`, `Vec`, `Env`, etc.) | The crate is `no_std`; standard library types are unavailable. |
| Write deterministic code (no randomness, no I/O, no time reads) | Soroban contracts must be reproducible across environments. |
| Register new sub-modules inside `sla_operations/mod.rs` only | Prevents accidental pollution of the top-level module tree. |
| Gate test code behind `#[cfg(test)]` | Keeps test code out of production WASM. |

### You MUST NOT

| Prohibition | Rationale |
|-------------|-----------|
| Modify `lib.rs` beyond the `pub mod sla_operations;` declaration | Core contract ABI must remain stable. |
| Alter storage keys (`symbol_short!("ADMIN")`, `CONFIG_KEY`, etc.) | On-chain storage layout is frozen after deployment. |
| Change existing event topic layouts or payload schemas | Event consumers depend on a stable schema. |
| Add `std` imports | The crate is compiled with `#![no_std]`. |
| Call `env.storage()` from fixture functions | Fixtures are pure data constructors; on-chain state is out of scope. |

---

## Data types used by this workspace

All public types referenced here are defined in `lib.rs` and re-used read-only:

| Type | Purpose |
|------|---------|
| `SLAConfig` | Threshold, penalty-per-minute, and reward-base for one severity level. |
| `SLAConfigEntry` | A `(severity, SLAConfig)` pair inside a snapshot. |
| `SLAConfigSnapshot` | An ordered, versioned list of `SLAConfigEntry` values. |
| `SLAResult` | Complete output of an SLA calculation (status, amount, rating, etc.). |

---

## Default configuration reference

The defaults set by `initialize()` are replicated in
`fixtures/policy_snapshots.rs::default_policy_snapshot()`. They are:

| Severity | Threshold (min) | Penalty/min | Reward base |
|----------|----------------|-------------|-------------|
| critical | 15             | 100         | 750         |
| high     | 30             | 50          | 750         |
| medium   | 60             | 25          | 750         |
| low      | 120            | 10          | 600         |

---

## Adding new workspace modules

1. Create the file under `sla_operations/`.
2. Declare it in `sla_operations/mod.rs`:
   ```rust
   pub mod my_new_module;
   ```
3. If the module is test-only, gate the declaration:
   ```rust
   #[cfg(test)]
   pub mod my_new_tests;
   ```
4. Do **not** touch `lib.rs` or any file outside `sla_operations/`.

---

## Related documentation

- [`testing.md`](./testing.md) — how to run tests scoped to this workspace.
- [`CONTRIBUTING.md`](../../../../../CONTRIBUTING.md) — project-wide contribution guide.
- [`CODING_STYLE.md`](../../../../../CODING_STYLE.md) — coding conventions for the crate.
