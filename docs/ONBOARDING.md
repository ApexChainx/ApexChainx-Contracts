# Onboarding Guide for First-Time Contributors

> **Issue:** [#89](https://github.com/ApexChainx/ApexChainx-Contracts/issues/89)
> **Status:** Active reference

Welcome! This guide walks you from zero to your first merged pull request in
the ApexChainx smart-contract repository. It is designed for developers who
already know some Rust but are new to Soroban, the Stellar network, or this
particular codebase.

---

## Table of Contents

1. [What You Are Looking At](#what-you-are-looking-at)
2. [Before You Start](#before-you-start)
3. [Local Environment Setup](#local-environment-setup)
4. [Repository Tour](#repository-tour)
5. [Choosing Your First Issue](#choosing-your-first-issue)
6. [Development Workflow](#development-workflow)
7. [Running Tests](#running-tests)
8. [Code Style](#code-style)
9. [Opening a Pull Request](#opening-a-pull-request)
10. [What Happens After You Open a PR](#what-happens-after-you-open-a-pr)
11. [Getting Help](#getting-help)

---

## What You Are Looking At

ApexChainx is a smart contract platform on the **Stellar network** using the
**Soroban** smart-contract runtime. This repository (`ApexChainx-Contracts`)
contains the execution-layer contract — a deterministic SLA (Service Level
Agreement) calculator with payment escrow and multi-party settlement.

The wider system has two sibling repositories:

| Repository | Role |
|------------|------|
| `apexchainx-fe` | Frontend application (React/TypeScript) |
| `apexchainx-be` | Backend API and contract bridge |
| **`apexchainx-contracts`** (this repo) | **The on-chain contract itself (Rust/Soroban)** |

The contract is written in Rust and compiles to a WASM artifact that deploys
to the Soroban runtime. It is **not** upgradeable in place — upgrades require
deploying a new contract and migrating state through the backend.

---

## Before You Start

### Prerequisites

| Tool | Version | Why |
|------|---------|-----|
| **Rust** (via rustup) | pinned in `rust-toolchain.toml` (currently 1.94.1) | Compile and test the contract |
| **just** | any recent | Task runner that mirrors CI commands |
| **Node.js** | ≥ 18 | Off-chain parity scripts and TypeScript tooling |
| **git** | any recent | Version control |
| **GitHub account** | — | Fork, branch, and open pull requests |

> You do **not** need the Stellar CLI or a wallet for contract development —
> the Soroban SDK's built-in `Env` test harness simulates the on-chain
> environment entirely in-process.

### Read These First

Before writing any code, skim these documents so you understand the
conventions:

1. **[CONTRIBUTING.md](../CONTRIBUTING.md)** — development workflow and PR checklist
2. **[CODING_STYLE.md](CODING_STYLE.md)** — Soroban Symbol naming and Rust conventions
3. **[docs/INDEX.md](INDEX.md)** — the documentation index; classify every doc as reference, design note, or superseded
4. **[docs/QUICKSTART.md](QUICKSTART.md)** — developer quickstart with common `just`/npm targets
5. **[docs/CONTRACT_LIFECYCLE.md](CONTRACT_LIFECYCLE.md)** — the state machine (init, pause, migrate, freeze)

---

## Local Environment Setup

### Option A: Dev Container (recommended for Codespaces / VS Code)

A [dev container](../.devcontainer/) is provided that auto-configures the Rust
toolchain, the `wasm32-unknown-unknown` target, `just`, and Node.js:

```bash
# Open in GitHub Codespaces or VS Code — the devcontainer handles everything.
```

See [`.devcontainer/README.md`](../.devcontainer/README.md) for details.

### Option B: Manual Setup

```bash
# 1. Fork and clone your fork
git clone https://github.com/<your-username>/ApexChainx-Contracts.git
cd ApexChainx-Contracts

# 2. Add the upstream remote (for syncing)
git remote add upstream https://github.com/ApexChainx/ApexChainx-Contracts.git

# 3. Install just (if not already installed)
#    brew install just  |  cargo install just  |  https://just.systems

# 4. Bootstrap the dev environment
#    This installs the pinned Rust 1.94.1 toolchain, adds the WASM target,
#    and verifies cargo is on PATH.
just bootstrap

# 5. Install Node dependencies (for off-chain parity scripts)
npm install

# 6. Verify your setup by running the full local CI equivalent
just ci
```

If `just ci` passes, your environment is ready.

---

## Repository Tour

```
ApexChainx-Contracts/
├── apexchainx_calculator/        ← The contract crate
│   ├── src/
│   │   ├── lib.rs               ← Contract entrypoints, storage keys, types
│   │   ├── config.rs            ← Severity config types and validation
│   │   ├── calculation.rs       ← SLA computation logic
│   │   ├── governance.rs         ← Admin/operator role management
│   │   ├── history.rs           ← History storage and pagination
│   │   ├── oracle.rs            ← Oracle module (metric ingestion)
│   │   ├── event_schema.rs       ← Event topic constants and versioning
│   │   ├── tests.rs              ← Main test suite
│   │   └── *_tests.rs            ← Module-specific test files
│   ├── fuzz/                     ← cargo-fuzz targets (nightly)
│   └── test_snapshots/           ← Golden vector JSON fixtures
├── .github/workflows/            ← CI pipelines
├── docs/                         ← All documentation
├── ts/                           ← TypeScript parity helpers
├── tests/                        ← TS property/regression tests
├── offchain/                     ← Off-chain budget/regression scripts
├── justfile                      ← Developer task runner
├── CONTRIBUTING.md               ← Contribution guide
└── Cargo.toml                    ← Workspace root
```

### Key Concepts

**Contract API Archetypes** — every public entrypoint falls into one of three
categories. Knowing which tells you the auth requirement:

| Archetype | Auth | Examples |
|-----------|------|----------|
| Read-only | None | `healthcheck`, `is_paused`, `get_config`, `get_stats` |
| Mutating (operator) | Operator role | `calculate_sla` |
| Privileged (admin) | Admin role | `set_config`, `pause`, `propose_admin` |

> Quick rule: methods starting with `get_`, `is_`, `list_`, `healthcheck`,
> `calculate_sla_view`, or `replay_calculate_sla` are read-only. Everything
> else requires operator or admin auth.

**Storage Keys** — the contract uses Soroban `Symbol` constants (max 9 chars)
as storage keys. Adding, removing, or renaming a key is a schema change that
requires a `STORAGE_VERSION` bump. See
[docs/STORAGE_KEY_MIGRATION_CHECKLIST.md](STORAGE_KEY_MIGRATION_CHECKLIST.md).

**Events** — the contract emits versioned events for backend indexing. Event
topic constants live in `event_schema.rs`. See
[docs/EVENT_TOPIC_COMPATIBILITY.md](EVENT_TOPIC_COMPATIBILITY.md).

---

## Choosing Your First Issue

1. Browse issues labeled [`good first issue`](https://github.com/ApexChainx/ApexChainx-Contracts/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22).
2. Read the issue description carefully.
3. Comment on the issue to let maintainers know you're interested.
4. **Wait for assignment** before starting work to avoid duplicate efforts.

> **Stellar Wave Program:** ApexChainx participates in the
> [Stellar Wave Program](https://www.drips.network/wave/stellar). Look for
> issues tagged `Stellar Wave`, apply to work on them, and submit a PR when
> ready. Only one contributor per issue.

---

## Development Workflow

```bash
# 1. Sync with upstream main
git checkout main
git fetch upstream
git merge upstream/main
git push origin main

# 2. Create a feature branch (use a descriptive name)
git checkout -b fix/my-first-contribution

# 3. Make your changes, then verify locally
just fmt           # format the code
just lint          # clippy with -D warnings
just check         # cargo check
just test          # run the test suite
just no-std        # verify WASM no_std compliance
just lint-orphans  # verify no orphaned .rs files

# 4. Run the full CI equivalent
just ci

# 5. Commit your changes (follow the project's commit message style)
git add -A
git commit -m "fix: describe what changed and why"

# 6. Push to your fork
git push -u origin fix/my-first-contribution
```

### Branch Naming

There is no enforced convention, but these prefixes are used in practice:

| Prefix | Use |
|--------|-----|
| `fix/` | Bug fixes |
| `feat/` | New features |
| `docs/` | Documentation changes |
| `test/` | Test additions |
| `ci/` | CI/workflow changes |
| `chore/` | Maintenance, refactoring |

---

## Running Tests

The project has multiple test layers. Run them with `just`:

| Command | What it runs |
|---------|-------------|
| `just test` | Rust library test suite (`cargo test --lib`) |
| `just fuzz` | Property-based fuzz tests (proptest, stable toolchain) |
| `just fuzz-spec` | Spec-assertion fuzz tests |
| `just parity-check` | Parity tests against golden vectors |
| `just ts-property-tests` | TypeScript property/regression tests |
| `just offchain-checks` | Off-chain budget and governance checks |
| `just ci` | **Everything** CI gates on, in CI's order |

### Test File Conventions

- Tests live in `#[cfg(test)] mod ...` blocks, either inline in `src/lib.rs`
  or in dedicated `*_tests.rs` files under `apexchainx_calculator/src/`.
- Each test file declares its own `setup()` helper that creates an `Env`,
  registers the contract, and returns a client + actors (admin, operator,
  stranger).
- Test modules are registered in `lib.rs` with `#[cfg(test)] mod module_name;`.
- The orphan module lint (`just lint-orphans`) fails if a `.rs` file is not
  declared as a module — so always register new test files in `lib.rs`.

### Snapshot Tests

Some tests produce JSON snapshots in `apexchainx_calculator/test_snapshots/`.
If contract behavior changes and snapshots need updating:

```bash
just regenerate-snapshots
just normalize-snapshots
```

Always review the diff before committing updated snapshots.

---

## Code Style

- **Formatting:** `cargo fmt` (enforced by CI). Run `just fmt` before committing.
- **Linting:** `cargo clippy --all-targets -- -D warnings` (enforced by CI). Run `just lint`.
- **No `std`:** The contract is `#![no_std]`. Never `use std::` or
  `extern crate std`. Use `alloc::format!`, `alloc::string::ToString`, etc.
  The `check-no-std.sh` script catches this early.
- **Naming:** Follow the Soroban Symbol naming conventions in
  [CODING_STYLE.md](CODING_STYLE.md). Storage keys are `symbol_short!` (≤ 9 chars).
- **Documentation:** Public functions should have doc comments. The
  `missing_docs` lint is intentionally disabled (Soroban proc-macro generated
  items can't be suppressed), so doc discipline is enforced by review.

### Common Gotchas

- **`unwrap()` / `expect()` in contract code:** Clippy with the security lint
  profile denies these in governance code. Prefer explicit error handling.
- **Integer arithmetic:** Use checked arithmetic (`checked_add`, `checked_sub`,
  etc.) to prevent overflow/underflow panics.
- **Storage key collisions:** Every new storage key must be added to the
  collision regression test in `tests.rs` and the `api_stability` key set.

---

## Opening a Pull Request

1. **Push your branch** to your fork:
   ```bash
   git push -u origin fix/my-first-contribution
   ```

2. **Create the PR** targeting the upstream `main` branch:
   ```bash
   gh pr create \
     --repo ApexChainx/ApexChainx-Contracts \
     --base main \
     --head <your-username>:fix/my-first-contribution \
     --title "fix: concise description (≤ 70 chars)" \
     --body "..."
   ```

   Or use the GitHub web UI — it will prompt you after you push.

3. **Fill in the PR template.** The repository uses a
   [PR template](../.github/PULL_REQUEST_TEMPLATE.md) with these sections:
   - Description
   - Type of change
   - Related issues (use `Fixes #NNN` or `Closes #NNN`)
   - Changes made
   - Testing performed
   - Checklist

4. **Link the issue:** Include `Fixes #NNN` or `Closes #NNN` in the PR
   description so GitHub automatically closes the issue on merge.

### PR Title Guidelines

- Keep titles under 70 characters.
- Use a prefix: `fix:`, `feat:`, `docs:`, `test:`, `ci:`, `chore:`.
- Describe the *what* and *why*, not the *how*.

---

## What Happens After You Open a PR

1. **CI runs** (when enabled). The CI pipeline includes:
   - Format check (`cargo fmt --check`)
   - Clippy (`cargo clippy --all-targets -- -D warnings`)
   - WASM no-std compliance check
   - Orphan module lint
   - E2E tests (`cargo test --lib`)
   - Fuzz regression tests
   - TS parity checks
   - Off-chain budget checks
   - Parity check against golden vectors

2. **Code review.** Maintainers will review your PR. The
   [Module Ownership Map](MODULE_OWNERSHIP.md) and
   [CODEOWNERS](../.github/CODEOWNERS) file determine who reviews what.
   Expect feedback — it's normal to go through a few rounds of review.

3. **Address feedback.** Push additional commits to the same branch (do not
   squash or force-push unless asked). Each push re-triggers CI.

4. **Merge.** A maintainer will merge your PR once it's approved and CI passes.

> **Note:** CI triggers are currently disabled in the repository. Your PR may
> not show CI status. Maintainers may run CI manually or ask you to verify
> locally with `just ci`.

---

## Getting Help

| Channel | When to use |
|---------|-------------|
| **GitHub Issues** | Bug reports, feature requests, questions about specific issues |
| **GitHub Discussions** | General questions, design discussions, "how do I..." questions |
| **PR comments** | Questions about specific code changes or review feedback |
| **[CONTRIBUTING.md](../CONTRIBUTING.md)** | Development workflow and guidelines |
| **[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)** | Community standards |

### Quick Reference

| You want to... | Run this |
|----------------|----------|
| Set up the environment | `just bootstrap` |
| Run all CI checks | `just ci` |
| Run tests | `just test` |
| Run fuzz tests | `just fuzz` |
| Check formatting | `just fmt-check` |
| Run clippy | `just lint` |
| Build WASM | `just wasm` |
| Run TS parity | `just ts-check` |

---

Welcome aboard, and happy hacking! 🚀
