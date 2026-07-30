# Contributing to ApexChainx

First off, thank you for considering contributing to ApexChainx! Your time and
expertise help make this project better for everyone in the Stellar ecosystem.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Stellar Wave Program](#stellar-wave-program)
- [Ways to Contribute](#ways-to-contribute)
- [Good First Issues](#good-first-issues)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style Guidelines](#code-style-guidelines)
- [Pull Request Guidelines](#pull-request-guidelines)
- [Testing Guidelines](#testing-guidelines)
- [Dependency Hygiene](#-dependency-hygiene)
- [Documentation Guidelines](#documentation-guidelines)
- [Security Guidelines](#security-guidelines)
- [SLAError Addition Workflow](#slaerror-addition-workflow)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)
- [Getting Help](#getting-help)
- [Repository Policies & Checklists](#repository-policies--checklists)

---

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to
follow. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

---

## 🌊 Stellar Wave Program

ApexChainx participates in the [Stellar Wave Program](https://www.drips.network/wave/stellar)!

### How to Participate

1. **Browse Issues**: Look for issues tagged with `Stellar Wave`
2. **Apply to Work**: Comment on the issue you want to work on
3. **Get Assigned**: Wait for a maintainer to assign you
4. **Submit PR**: Create a pull request when ready

> **Important:** Only one contributor per issue. First to apply and get assigned
> gets the work.

## 🤝 Ways to Contribute

| Contribution Type | Description | Ideal For |
|------------------|-------------|-----------|
| 🐛 Bug Reports | Report issues with clear reproduction steps | All skill levels |
| 💡 Feature Suggestions | Propose new capabilities with use cases | Experienced users |
| 🔧 Bug Fixes | Submit PRs with tested fixes | Developers |
| 📖 Documentation | Improve guides, add examples, fix typos | Writers & developers |
| 🧪 Tests | Increase coverage, add edge cases | QA & developers |
| 👀 Code Reviews | Review pull requests for quality | Senior developers |
| 💬 Community Support | Help answer questions in discussions | All skill levels |

## 🌟 Good First Issues

New to ApexChainx? Looking for a place to start? Check out issues tagged with:

- **`good first issue`** - Perfect for first-time contributors, low complexity, well-documented
- **`help wanted`** - Issues that need extra attention
- **`Stellar Wave`** - Issues eligible for the [Stellar Wave Program](https://www.drips.network/wave/stellar)

[Browse all good first issues →](https://github.com/ApexChainx/ApexChainx-Contracts/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)

### How to Pick Your First Issue

1. **Browse the list** of good first issues
2. **Read the issue description** carefully to understand what's needed
3. **Comment on the issue** to let maintainers know you're interested
4. **Wait for assignment** before starting work to avoid duplicate efforts

## 🚀 Getting Started

### Prerequisites

**For Frontend (apexchainx-fe):**
- Node.js 18.x or higher
- npm or yarn
- Git
- Freighter wallet (for Stellar features)

**For Backend (apexchainx-be):**
- Python 3.9 or higher
- pip and virtualenv
- Git

**For Smart Contracts (apexchainx-contracts):**
- Rust and Cargo
- Soroban CLI
- Stellar CLI

### Fork and Clone

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/apexchainx-fe.git
   # or
   git clone https://github.com/YOUR_USERNAME/apexchainx-be.git
   # or
   git clone https://github.com/YOUR_USERNAME/apexchainx-contracts.git
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/ApexChainx/apexchainx-fe.git
   ```

### Setup Development Environment

**Frontend:**
```bash
cd apexchainx-fe
npm install
cp .env.example .env.local
# Edit .env.local with your config
npm run dev
```

**Backend:**
```bash
cd apexchainx-be
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
cp .env.example .env
# Edit .env with your config
uvicorn main:app --reload
```

**Smart Contracts:**
```bash
cd apexchainx-contracts

# 1. Install rustup (if not already installed)
#    https://rustup.rs

# 2. Install just (if not already installed)
#    brew install just  |  cargo install just  |  https://just.systems

# 3. Bootstrap the dev environment in one command:
#    - installs the pinned Rust 1.94.1 toolchain with rustfmt + clippy
#    - adds the wasm32-unknown-unknown cross-compilation target
#    - verifies cargo is on PATH
#    (idempotent: safe to re-run at any time)
just bootstrap

# 4. Verify your setup against the full local CI equivalent:
just ci
```

> **Pinned toolchain:** The project uses Rust `1.94.1` (see `rust-toolchain.toml`).
> `just bootstrap` installs this version automatically. You do not need to set
> it manually — `rustup` reads `rust-toolchain.toml` and selects the right
> version for every `cargo` command in this directory.

> **Session safety:** `just bootstrap` is idempotent. Running it again after a
> fresh shell or a `rustup update` is safe and will only download what is
> missing or outdated.

## 📝 Development Workflow

### Step 1: Create a Feature Branch

Always create a new branch for your work. Use a descriptive name:

```bash
git checkout -b feature/wallet-integration
git checkout -b fix/payment-bug
git checkout -b docs/stellar-guide
git checkout -b test/api-coverage
git checkout -b refactor/storage-layer
```

#### Branch Naming Convention

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feature/` | New features | `feature/wallet-integration` |
| `fix/` | Bug fixes | `fix/payment-timeout` |
| `docs/` | Documentation | `docs/stellar-guide` |
| `test/` | Test additions | `test/api-coverage` |
| `refactor/` | Code restructuring | `refactor/storage-layer` |

### Step 2: Make Your Changes

- Write clean, readable code following project conventions
- Add tests for new functionality
- Update documentation as needed
- Keep commits **focused and atomic** — one logical change per commit
- Update `CHANGELOG.md` for any interface-affecting changes

### Step 3: Run Tests

#### Smart Contracts

```bash
# Run full test suite
cd apexchainx_calculator
cargo test

# Run with linting
cargo clippy -- -D warnings

# Check formatting
cargo fmt -- --check

# Verify no-std compliance
cargo check --target wasm32-unknown-unknown --lib
```

#### Frontend

```bash
npm run test
npm run lint
npm run type-check
```

#### Backend

```bash
pytest
pytest --cov=app --cov-report=html
black app/
flake8 app/
mypy app/
```

### Step 4: Commit Using Conventional Commits

We follow [Conventional Commits](https://www.conventionalcommits.org/) for all
commit messages.

#### Format

```
<type>: <short description>

[optional body with additional context]

[optional footer referencing issues]
```

#### Commit Types

| Type | Usage | Example |
|------|-------|---------|
| `feat` | New feature | `feat: add wallet balance display` |
| `fix` | Bug fix | `fix: resolve payment timeout issue` |
| `docs` | Documentation | `docs: update stellar integration guide` |
| `style` | Formatting | `style: reformat config module` |
| `refactor` | Code restructuring | `refactor: extract storage layer` |
| `test` | Test additions | `test: add SLA boundary cases` |
| `chore` | Maintenance | `chore: update dependencies` |
| `perf` | Performance | `perf: optimize config lookup` |

#### Examples

```bash
git commit -m "feat: add wallet balance display"
git commit -m "fix: resolve payment timeout issue"
git commit -m "docs: update stellar integration guide"
git commit -m "test: add unit tests for SLA calculator"
```

### Step 5: Push and Open a Pull Request

```bash
git push origin feature/wallet-integration
```

Then open a pull request on GitHub with:

- **Clear title** following conventional commit format
- **Description** explaining what and why
- **Screenshots** (for UI changes)
- **Testing notes** (how you verified the changes)
- **Related issue**: `Closes #123` or `Fixes #456`

## 🎨 Code Style Guidelines

### Smart Contracts (Rust/Soroban)

#### Principles

- **Determinism first:** All computations must be deterministic — no floating point, no randomness
- **Gas efficiency:** Minimize storage writes, avoid unnecessary loops
- **Safety:** Use integer math only, validate all inputs, fail early
- **Documentation:** All public functions must have doc comments following the canonical comment policy

#### Style Rules

| Rule | Standard |
|------|----------|
| Naming | `snake_case` for functions/variables, `PascalCase` for types |
| Error handling | Custom error types via `#[contracterror]` |
| Imports | Group: std → external crates → internal modules |
| Formatting | `cargo fmt` (automated) |
| Linting | `cargo clippy -- -D warnings` (no warnings allowed) |
| Comments | Follow the canonical comment policy in [`CODING_STYLE.md`](CODING_STYLE.md) (see Part 2) |

#### Example

```rust
#[contractimpl]
impl SLAContract {
    /// Calculate SLA result for an outage.
    ///
    /// # Arguments
    /// * `outage_id` - Unique identifier for the outage event
    /// * `severity` - Severity level (Critical, High, Medium, Low)
    /// * `mttr_minutes` - Mean time to repair in minutes (0-525600)
    ///
    /// # Returns
    /// `SLAResult` containing SLA status, payment type, and rating
    pub fn calculate_sla(
        env: Env,
        outage_id: Symbol,
        severity: Severity,
        mttr_minutes: u32,
    ) -> SLAResult {
        // Implementation
    }
}
```

### Frontend (TypeScript/React)

| Rule | Standard |
|------|----------|
| Language | TypeScript for all new files |
| Components | Functional components with hooks |
| Styling | Tailwind CSS (no inline styles) |
| UI Library | shadcn/ui components when available |
| Reusability | Extract logic into custom hooks |
| Typing | TypeScript interfaces for all props |

### Backend (Python/FastAPI)

| Rule | Standard |
|------|----------|
| Style | PEP 8 |
| Typing | Type hints for all functions |
| Documentation | Docstrings for all public functions |
| I/O | async/await for all operations |
| Validation | Pydantic models for request/response |
| Architecture | Dependency injection for services |
| Configuration | Environment variables via `.env` |


## ✅ Pull Request Guidelines

### API Stability Consideration

Before modifying any public contract entrypoint, consult the **[API Stability Scorecard](docs/API_STABILITY_SCORECARD.md)** to determine whether your change is additive (safe) or breaking (requires a version bump and/or migration). The scorecard classifies every public function by stability tier (🔒 Frozen, ⚠️ Stable, 🔄 Evolving, 🛡️ Admin-Gated) and provides a change impact guide.
### Review Routing

Before opening a PR, consult the **[Module Ownership Map](docs/MODULE_OWNERSHIP.md)** to identify which review groups should be requested. High-risk modules (calculation, governance, storage version, event schema, config, cross-contract safety) have additional review requirements documented in the map.
### Before Submitting Checklist

#### Required Checks

- [ ] Code follows the project's style guidelines
- [ ] Self-review completed — read your own diff first
- [ ] Tests added/updated and all passing
- [ ] Documentation updated (README, docs/, inline comments)
- [ ] No `console.log`, `println!`, or `dbg!` statements left in code
- [ ] Environment variables documented in `.env.example`
- [ ] Breaking changes clearly documented in the PR description

#### Smart Contract Specific

- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` produces no warnings
- [ ] `cargo fmt -- --check` confirms formatting compliance
- [ ] `cargo machete` passes (no unused dependencies in `Cargo.toml`)
- [ ] `cargo +nightly udeps --all-targets` passes (no unused dependencies in code; requires nightly toolchain)
- [ ] `cargo check --target wasm32-unknown-unknown --lib` passes (no-std check)
- [ ] New public functions are added to the result schema or documented
- [ ] Any breaking change to `SLAResult` increments `RESULT_SCHEMA_VERSION`

### PR Description Template

```markdown
## Description

Brief description of the changes.

## Type of Change

- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issue

Closes #123

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing completed

## Screenshots (if applicable)

[Add screenshots here]

## Additional Notes

Any additional information for reviewers.
```

### For Stellar Wave Contributors

Include in your PR description:

- **Testnet transaction hashes** (for blockchain features)
- **Video/GIF** of feature working (for UI changes)
- **Performance metrics** (if relevant)
- **Time spent** on the issue (optional)

## 🧪 Testing Guidelines

### Smart Contract Tests

```bash
# Run full test suite
cd apexchainx_calculator
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_sla_boundary_conditions
```

### Frontend Tests

```bash
npm run test
npm run test:watch
npm run test:coverage
```

### Backend Tests

```bash
pytest
pytest tests/test_payment_service.py
pytest --cov=app --cov-report=html
```

## 🧹 Dependency Hygiene

The CI pipeline checks for unused dependencies to keep the codebase lean and
maintainable. Drift from unused dependencies silently increases build times,
attack surface, and maintenance burden.

### Why It Matters

- **`cargo machete`** detects dependencies declared in `Cargo.toml` that are no
  longer imported anywhere in the crate. Removing them reduces compile time and
  audit surface.
- **`cargo udeps`** detects dependencies that are available in the workspace but
  not actually used by the crate's code. This catches cases where a dependency
  was added for an experiment and never cleaned up.

Both checks are enforced in CI (see the **Unused dependency gate** step in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml)). A PR that introduces
unused dependencies will fail CI.

### How to Run Locally

```bash
# Install the tools (one-time)
cargo install cargo-machete --locked
cargo install cargo-udeps --locked

# Ensure the nightly toolchain is available (required by cargo-udeps)
rustup toolchain install nightly

# Check for unused dependencies
cd apexchainx_calculator
cargo machete
cargo +nightly udeps --all-targets
```

Or use the `just` recipes (see [`justfile`](justfile)):

```bash
just machete
just udeps
```

The full CI pipeline — including both hygiene checks — can be run with:

```bash
just ci
```

> **Tip:** Run `just machete` and `just udeps` before opening a PR to catch
> dependency issues early. The [PR checklist](#before-submitting-checklist)
> includes both checks under the *Smart Contract Specific* section.
>
> **Note:** `cargo-udeps` uses nightly-only compiler features, so it requires
> the nightly Rust toolchain. The `just udeps` recipe handles this automatically
> by invoking `cargo +nightly udeps`.

## 📚 Documentation Guidelines

| Principle | Practice |
|-----------|----------|
| **Clarity** | Use clear, concise language |
| **Examples** | Include runnable code examples |
| **Visuals** | Add diagrams for architecture, screenshots for UI |
| **Freshness** | Keep docs in sync with code changes |
| **Linking** | Cross-reference related documentation |
| **Formatting** | Use Markdown with consistent structure |

## 🔒 Security Guidelines

### Do's

- ✅ Use environment variables for all secrets
- ✅ Validate all inputs at the contract boundary
- ✅ Apply principle of least privilege to roles
- ✅ Keep dependencies updated via Dependabot/manual review
- ✅ Run cargo audit before merging dependency changes

### Don'ts

- ❌ Never commit API keys, private keys, or passwords
- ❌ Never trust user input without validation
- ❌ Never use unsafe code in smart contracts

---

## 📋 Repository Policies & Checklists

In addition to the inline checklists below, this repository maintains several
standalone policy documents and review templates:

| Document | Purpose | Audience |
|----------|---------|----------|
| [docs/PUBLIC_FUNCTION_DOC_POLICY.md](docs/PUBLIC_FUNCTION_DOC_POLICY.md) (SC-102) | Mandatory doc comments on all public items | All contributors |
| [docs/UPGRADE_REVIEW_CHECKLIST.md](docs/UPGRADE_REVIEW_CHECKLIST.md) (SC-103) | Admin-facing upgrade proposal review | Contract admins, multisig signers |
| [docs/SECURITY_REVIEW_TEMPLATE.md](docs/SECURITY_REVIEW_TEMPLATE.md) (SC-104) | Security review for new contract modules | Developers & security reviewers |
## SLAError Addition Workflow

Adding a new `SLAError` variant is a **contract interface change**, even when it
appears to be an internal detail.  Backend consumers that depend on numeric error
discriminants or call `get_failure_schema()` to build a lookup table must be able
to adapt without silent failures.

**Full guide:** [`docs/sla-error-additions-guide.md`](docs/sla-error-additions-guide.md)

### Quick rules

| Rule | Detail |
|------|--------|
| Append only | New variants go at the **end** of `SLAError`, never in the middle |
| Stable discriminants | Once shipped, `FooError = N` means `N` maps to `FooError` forever — never renumber or remove |
| Typed helper required | Every new variant needs a matching `is_<variant>` predicate in `error_responses.rs` |
| Catalogue update required | `get_failure_schema()` must include an entry for every variant |
| CHANGELOG required | Add an entry under `[Unreleased]` noting the new variant and discriminant |
| Tests required | Unit test for the error path + predicate smoke test + catalogue count assertion |
| Backend notification | If any existing label or discriminant changes, notify the `apexchainx-be` team |

See the [full guide](docs/sla-error-additions-guide.md) for step-by-step
instructions, deprecation protocol, and compatibility guarantees.

---
## 🐛 Reporting Bugs

| Field | Description | Required |
|-------|-------------|----------|
| Title | Clear, descriptive summary | ✅ |
| Steps to reproduce | Exact steps to trigger the bug | ✅ |
| Expected behavior | What should happen | ✅ |
| Actual behavior | What actually happens | ✅ |
| Screenshots | Visual evidence if applicable | Optional |
| Environment | OS, browser, versions | ✅ |
| Error messages | Full stack trace if available | ✅ |
| Stellar details | Network + tx hash if applicable | For Stellar issues |


## 💡 Suggesting Features

Use the GitHub issue template and include:

- **Clear title** describing the feature
- **Problem statement** (what problem does this solve?)
- **Proposed solution**
- **Alternative solutions** considered
- **Additional context** (mockups, examples, etc.)


## 📞 Getting Help

- **GitHub Issues**: For bugs and feature requests
- **Discord**: [Join our server] (link TBD)
- **Stellar Discord**: For Stellar-specific questions

## 📜 License

By contributing to ApexChainx, you agree that your contributions will be licensed under the MIT License.

## 🙏 Thank You!

Your contributions make ApexChainx better for everyone. We appreciate your time and effort!

---

## Contract Maintenance Policies

All contributors and reviewers must follow the maintenance policies documented in
[`docs/CONTRACT_MAINTENANCE_POLICY.md`](docs/CONTRACT_MAINTENANCE_POLICY.md).

### Code Comment Policy

Every Rust source file must follow the canonical code comment policy defined in
[`CODING_STYLE.md`](CODING_STYLE.md) (Part 2). The policy distinguishes between:

- **Invariants** (`// INVARIANT:`) — properties that must always be true for correctness
- **Public API notes** (`///` doc comments) — required for every `pub` item
- **Implementation details** (`//` inline) — non-obvious algorithm choices and workarounds

The policy defines **what must** be commented, **what should not** be commented,
and how to review comment compliance in PRs. See the complete policy in
`CODING_STYLE.md` for the full review checklist.

### Maintenance Policy Coverage

This covers:

- **[SC-500] `#[contracttype]` Compatibility Note Policy** (#279) — every public
  contract type change must include a compatibility note in the PR.
- **[SC-501] Response-Shape Stability Policy** (#283) — every `#[contracttype]`
  return type is assigned a stability tier (Stable, Versioned, Experimental).
- **[SC-502] Version Negotiation Protocol Note** (#284) — safe vs. breaking
  changes to `get_version_info()`. For the multi-contract negotiation protocol
  itself, see
  [`docs/VERSION_NEGOTIATION_CONTRIBUTOR_GUIDE.md`](docs/VERSION_NEGOTIATION_CONTRIBUTOR_GUIDE.md).
- **[SC-503] API Archetype Note** (#285) — three function archetypes: Read-Only,
  Mutating (Operator), Privileged (Admin).
- **[SC-504] Event Payload Size Check** (#286) — payload size assertions required
  for every event change.
- **[SC-505] Event Drift Review Note** (#287) — checklist for event name/payload
  changes. See also the [quick-reference checklist](docs/EVENT_DRIFT_CHECKLIST.md).
- **[SC-506] History Write Audit Check** (#288) — FIFO ordering, pruning, and
  idempotency invariants.
- **[SC-507] Telemetry Counters Policy** (#289) — additive-only counter fields,
  saturation handling.
- **[SC-508] Role-Change Incident Review Note** (#290) — admin/operator handoff
  safety decision table.
- **[SC-509] SLAError Addition Workflow** (#253) — step-by-step guide for adding,
  deprecating, or reviewing `SLAError` variants without breaking backend adapter
  logic. See [`docs/sla-error-additions-guide.md`](docs/sla-error-additions-guide.md).

### Release Summary Generator (#280)

Run `npx tsx tooling/release-summary.ts` to generate a maintainer-friendly
release summary from the `[Unreleased]` section of `CHANGELOG.md`. Use `--json`
for CI or `--check` for format validation.

### Devcontainer Setup (#281)

A `.devcontainer/` setup is provided for GitHub Codespaces and VS Code Dev
Containers. The devcontainer includes Rust, the `wasm32-unknown-unknown` target,
`just`, and Node.js. On first launch the devcontainer runs `just bootstrap`
automatically. Run `just ci` to verify your environment matches CI.

See [`.devcontainer/README.md`](.devcontainer/README.md) for detailed setup
instructions.

### Local Bootstrap (#257)

`just bootstrap` is the single, session-safe entry point for local contributors:

```bash
just bootstrap   # installs pinned toolchain, WASM target, verifies cargo
just ci          # runs the full local CI equivalent to confirm everything works
```

**What `just bootstrap` does:**

| Step | Action | Idempotent? |
|------|--------|-------------|
| 1 | Verifies `rustup` is installed and reachable | — (fails fast if missing) |
| 2 | Installs / updates pinned toolchain `1.94.1` with `rustfmt` and `clippy` components | ✓ |
| 3 | Adds `wasm32-unknown-unknown` cross-compilation target | ✓ |
| 4 | Verifies `cargo` is on `PATH` | — (fails fast if missing) |

**Pinned toolchain:** the `channel = "1.94.1"` entry in `rust-toolchain.toml`
ensures every `cargo` command in the repository uses the same compiler version
as CI. You do not need to set this manually — `rustup` reads the file
automatically.

**Session safety:** `just bootstrap` is safe to re-run after a fresh terminal,
after `rustup update`, or after a CI toolchain bump. It only performs work when
something is actually missing or outdated.

**Prerequisites** (must be installed manually — not handled by `just bootstrap`):

- `rustup` — https://rustup.rs
- `just` — `brew install just` · `cargo install just` · https://just.systems

---

## SC-098: Security Review Checklist for Privileged Changes

Use this checklist when reviewing PRs that touch governance, config, or storage.

### Authentication & Authorisation

- [ ] All privileged functions call `require_auth()` on the correct role (admin or operator)
- [ ] No function bypasses the role check under any code path
- [ ] Role assignments (admin, operator) can only be changed by the current admin
- [ ] Pause/unpause state is checked at the top of every write function

### Configuration Writes

- [ ] `set_config` only accepts valid severity symbols (critical / high / medium / low)
- [ ] `threshold_minutes`, `penalty_per_minute`, and `reward_base` are validated as non-zero positive values
- [ ] Config changes emit a versioned `cfg_upd` event with the new values
- [ ] After a config write the backend parity tests are re-run against the updated snapshot

### Storage Changes

- [ ] No new storage key is added without a corresponding version bump or migration path
- [ ] Persistent storage writes are minimised — avoid writes on read-only queries
- [ ] History pruning operations are admin-gated and emit a `pruned` event

### Pause Behaviour

- [ ] Contract-paused guard is present in all state-changing functions
- [ ] Pause state is correctly persisted and readable via `get_paused`
- [ ] Tests cover behaviour of every write function while paused

### General

- [ ] New public functions are added to the result schema or documented if they are read-only helpers
- [ ] Any breaking change to `SLAResult` increments `RESULT_SCHEMA_VERSION`
- [ ] CI passes: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `wasm32` build

---

---

## SC-099: Event-Topic & Payload Schema Contributor Safety Checklist

Use this checklist when reviewing PRs that add, remove, rename, or reorder event
topic constants or payload fields in `apexchainx_calculator/src/lib.rs` or
`apexchainx_calculator/src/event_schema.rs`.

Backend indexers and the `apexchainx-be` bridge depend on a stable,
deterministic event structure. A single field reorder or topic rename breaks
downstream consumers **silently** — no compilation error, no test failure,
just corrupted indexed data and broken settlement reconciliation.

### Event-Topic Constants

- [ ] **No topic name changed without a version bump.** Every event is identified by a `Symbol` constant (e.g. `EVENT_SLA_CALC`). Renaming the constant or changing its Symbol value is a **breaking change** and must increment the version symbol from `"v1"` to `"v2"` in both the constant definition and every emission site.
- [ ] **All topic constants are covered by the distinctness test** in `event_schema.rs` (`test_event_names_are_distinct`). If a new event constant is added, ensure it is appended to the `names` array in that test.
- [ ] **Topic index layout is unchanged.** The 3-topic layout (`topic[0]` = name, `topic[1]` = version, `topic[2]` = context) is a contract with backend consumers. Do not add, remove, or reorder topics. Any new metadata must go into the event **data payload**, not the topics array.
- [ ] **All emission sites are updated.** Every event constant is emitted in at least one place (search for `(EVENT_*, EVENT_VERSION,`). If a new event is added, at least one emission site must be included in the same PR.
- [ ] **The event schema doc comment in `lib.rs` (lines ~183-240) is updated.** The comment block lists every event's payload schema. New events must be documented there with the same format: `event_name  → (field: Type, ...)` and context description.

### Payload Schema Changes

- [ ] **Field additions go at the end.** Appending new fields to the end of a payload tuple is **not** breaking — old consumers ignore unrecognised trailing fields. Inserting, removing, or reordering fields **is** breaking.
- [ ] **Field type changes are breaking.** Changing a field's Soroban type (e.g. `u32` → `i128`, `Symbol` → `Address`) requires a version bump.
- [ ] **All payload emission sites emit the same tuple shape.** Search the codebase for the event name and verify every `env.events().publish(...)` call for that event emits a tuple with identical field count and types.
- [ ] **Topic stability tests pass.** Run `cargo test topic_stability_tests` and verify all assertion-based topic-structure checks pass.
- [ ] **The `event_schema.rs` doc comment is updated** to reflect the new/changed payload layout, including the type signature in the event catalog section.

### Versioning Protocol

- [ ] **Breaking changes bump `EVENT_VERSION`.** If any event's topic name, topic order, payload field count, or payload field type changes, the version Symbol must be incremented (`"v1"` → `"v2"`).
- [ ] **Additive-only changes do NOT bump the version.** Adding a new event constant (new name, not reusing an old one) or appending a new field to the end of an existing payload tuple does not require a version bump.
- [ ] **The Symbol Deprecation Protocol in `event_schema.rs` is followed.** If a symbol (status, payment type, rating, etc.) is being replaced, the old symbol must enter the deprecation lifecycle: introduction → coexistence → removal, as documented in `event_schema.rs` lines ~120-160.
- [ ] **`get_result_schema()` reflects the deprecation.** If a symbol was deprecated, `deprecated_symbols` in the returned `SLAResultSchema` must include an entry with the old/new mapping and `deprecated_at` version.

### Backend Integration Safety

- [ ] **No event is removed without a deprecation period.** Removing an event entirely (stopping emission) is a major breaking change. Coordinate with the `apexchainx-be` team and provide at least one release cycle of coexistence.
- [ ] **The `apexchainx-be` event schema parity tests still pass.** Coordinate with the backend team to re-run their contract-event snapshot tests after any topic or payload change.
- [ ] **CI passes:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `wasm32` build, and the new no-std lint script.

---

## SC-100: Release-Readiness Checklist for Contract Shape Changes

If your PR touches **storage keys, the storage schema version, event topic
constants, or event payload fields**, you must work through the dedicated
checklist before requesting a review.

**→ [`docs/CONTRACT_SHAPE_CHANGE_CHECKLIST.md`](docs/CONTRACT_SHAPE_CHANGE_CHECKLIST.md)**

The checklist covers:

| Section | What it checks |
|---------|----------------|
| **1. Storage schema** | New/renamed keys, `STORAGE_VERSION` bump, migration path, required tests |
| **2. Event schema** | Topic constants, payload field order, version bump, symbol deprecation, required tests |
| **3. Backend coordination** | Notifying `apexchainx-be`, filing deprecation follow-ups, updating `AUDIT_TRAIL.md` |
| **4. Release artifacts** | `CHANGELOG.md` entry with explicit version numbers |
| **5. CI gate** | `just ci` equivalent — all steps green before review |

> **When in doubt, open the checklist.** Storage and event-shape changes are
> invisible to the Rust compiler but can silently break backend indexers and
> settlement reconciliation in production.
## SC-100: Public Method Review Checklist

Use this checklist when reviewing PRs that add or modify public `#[contractimpl]` methods in `apexchainx_calculator/src/lib.rs`. A seemingly harmless method can break downstream assumptions if versioning, events, or state migrations are missed.

### 1. Event Schema
- [ ] **Topic Layout**: Any new event emission strictly follows the 3-topic layout (`name`, `version`, `context`).
- [ ] **Payload Rules**: If an existing event payload has changed, ensure fields were only appended, or the `EVENT_VERSION` was bumped. (See SC-099).
- [ ] **Size Bounds**: Event payloads do not exceed reasonable size limits (SC-504).

### 2. Versioning & Responses
- [ ] **Schema Updates**: If the method returns new status symbols or data structures, `RESULT_SCHEMA_VERSION` is incremented.
- [ ] **Stability Tier**: The return type's stability tier (Stable, Versioned, Experimental) is documented (SC-501).
- [ ] **Error Codes**: Any new errors are added to the `SLAError` enum and the failure schema.

### 3. State & Migration
- [ ] **Storage Keys**: If new storage keys are introduced, the `STORAGE_VERSION` is incremented, and `get_migration_state()` reflects `needs_migration`.
- [ ] **Determinism**: The method relies only on deterministic inputs (no unseeded randomness, no floats).
- [ ] **History & Pruning**: If the method writes to history, it adheres to the FIFO invariants (SC-506) and bounds `MAX_RECALCS_PER_OUTAGE`.

### 4. Authorisation & Safety
- [ ] **Archetype Defined**: The doc comment clearly defines the method's archetype (Read-Only, Mutating, Privileged).
- [ ] **Auth Checks**: Privileged and Mutating methods call `require_auth()` on the correct role.
- [ ] **Pause Guard**: State-mutating methods check the `PAUSED_KEY` at the top of the execution.
## SC-100: Storage-Key & Event-Topic Namespace Collision Pre-Merge Checklist

Use this checklist **before merging any PR** that introduces new storage keys
or event topic constants. Namespace collisions between storage keys or between
event topics are easy to introduce during development but difficult to diagnose
after deployment, especially when multiple contributors work on the same crate.

This checklist complements the automated collision-detection tests already in
the test suite (`test_storage_key_namespace_no_collisions` in `lib.rs` and
`test_event_names_are_distinct` in `event_schema.rs`). Manual human review is
still required because tests only cover the final state of a single branch —
they cannot catch collisions that arise during merge conflict resolution or
when two PRs independently introduce overlapping names.

### Reference: Current Storage Key Namespace (`apexchainx_calculator/src/lib.rs`)

All storage keys use 9-character-or-shorter `Symbol` constants defined with
`symbol_short!()`. The following are the currently registered keys:

| Constant | Symbol Value | Purpose |
|----------|-------------|---------|
| `ADMIN_KEY` | `"ADMIN"` | Admin address |
| `OPERATOR_KEY` | `"OPERATOR"` | Operator address |
| `PENDING_ADMIN_KEY` | `"PADMIN"` | Pending admin for two-step transfer |
| `PENDING_OP_KEY` | `"POP"` | Pending operator for two-step handoff |
| `CONFIG_KEY` | `"CONFIG"` | Per-severity SLA configuration map |
| `CUSTOM_CONFIG_KEY` | `"CUSTCFG"` | Admin-defined custom severity configs |
| `PAUSED_KEY` | `"PAUSED"` | Pause-state boolean flag |
| `PAUSE_INFO_KEY` | `"PAUSEINF"` | Pause metadata (reason, timestamp, caller) |
| `STATS_KEY` | `"STATS"` | Cumulative SLA statistics |
| `SEVERITY_CALC_COUNTS_KEY` | `"CALCCNT"` | Per-severity weekly calculation counter |
| `SEVERITY_VIOL_COUNTS_KEY` | `"VIOLCNT"` | Per-severity weekly violation counter |
| `LAST_CALCULATION_LEDGER_KEY` | `"CALCLDG"` | Per-severity last calculation ledger |
| `LAST_VIOLATION_LEDGER_KEY` | `"VIOLLDG"` | Per-severity last violation ledger |
| `HISTORY_KEY` | `"HIST"` | Ordered list of historical SLA results |
| `STORAGE_VERSION_KEY` | `"VER"` | Current on-chain storage schema version |
| `RETENTION_LIMIT_KEY` | `"RETLIM"` | Configurable retention limit override |
| `LAST_CFG_UPDATE_KEY` | (from config_metadata) | Ledger sequence of last config update |

### Reference: Current Event Topic Namespace (`apexchainx_calculator/src/event_schema.rs`)

All events follow a standardised 3-topic layout:
`topic[0]` = event name (Symbol), `topic[1]` = version (`"v1"`),
`topic[2]` = event-specific context.

The following event name constants are currently defined:

| Constant | Symbol Value | Payload Context |
|----------|-------------|----------------|
| `EVENT_SLA_CALC` | `"sla_calc"` | severity Symbol |
| `EVENT_SETTLE_INTENT` | `"set_int"` | severity Symbol |
| `EVENT_CONFIG_UPD` | `"cfg_upd"` | severity Symbol |
| `EVENT_PAUSED` | `"paused"` | caller Address |
| `EVENT_UNPAUSED` | `"unpause"` | caller Address |
| `EVENT_OP_SET` | `"op_set"` | caller Address |
| `EVENT_PRUNED` | `"pruned"` | caller Address |
| `EVENT_PRUNED_AGE` | `"pruned_a"` | caller Address |
| `EVENT_ADMIN_PROP` | `"adm_prop"` | caller Address |
| `EVENT_ADMIN_ACC` | `"adm_acc"` | caller Address |
| `EVENT_ADMIN_CAN` | `"adm_can"` | caller Address |
| `EVENT_ADMIN_REN` | `"adm_ren"` | caller Address |
| `EVENT_OP_PROP` | `"op_prop"` | caller Address |
| `EVENT_OP_ACC` | `"op_acc"` | caller Address |
| `EVENT_OP_CAN` | `"op_can"` | caller Address |
| `EVENT_CONFIG_FREEZE` | `"cfg_frz"` | caller Address |
| `EVENT_CONFIG_UNFREEZE` | `"cfg_unfrz"` | caller Address |
| `EVENT_STATS_SAT` | `"stats_sat"` | counter_name Symbol |
| `EVENT_MIGRATE_DONE` | `"migrate_done"` | caller Address |

### Pre-Merge Checklist

#### Storage-Key Collision Check

- [ ] Every new storage key constant uses `symbol_short!()` with a unique
      9-character-or-shorter Symbol value.
- [ ] The new Symbol value does **not** collide with any existing key in the
      table above or in any sub-module of the crate (search the entire crate
      with `grep -rn "symbol_short" apexchainx_calculator/src/`).
- [ ] The new key is added to the storage-keys namespace block in
      `apexchainx_calculator/src/lib.rs` (lines ~48-130), not scattered across
      sub-modules with inline `symbol_short!()` calls.
- [ ] The new key has a meaningful Rust constant name with the `_KEY` suffix
      convention (e.g. `MY_NEW_KEY`).
- [ ] The `test_storage_key_namespace_no_collisions` test in `lib.rs` passes.
- [ ] A merge-conflict scenario has been considered: if another open PR adds
      a key with the same Symbol value, this PR will need coordination with
      that contributor before merging.

#### Event-Topic Collision Check

- [ ] Every new event constant uses `symbol_short!()` with a unique
      9-character-or-shorter Symbol value.
- [ ] The new Symbol value does **not** collide with any existing event name
      in the table above.
- [ ] The new constant is added to `event_schema.rs` (not scattered in-line
      across other modules), with a doc comment explaining its purpose.
- [ ] The new constant is appended to the `names` array in the
      `test_event_names_are_distinct` test in `event_schema.rs`.
- [ ] The event schema doc comment in `event_schema.rs` includes the new event
      with its topic layout and payload schema documented.
- [ ] At least one emission site is included in the same PR.
- [ ] The 3-topic layout (`topic[0]=name, topic[1]=version, topic[2]=context`)
      is preserved for the new event.
- [ ] `cargo test` passes (the collision tests are part of the test suite).

#### Cross-Module Check

- [ ] The new Symbol value has been checked against **both** the storage-key
      AND event-topic namespaces. (Storage keys and event topics share the
      Soroban Symbol namespace — a collision between a storage key and an
      event name would not cause a compile error but would create confusing
      observability data.)
- [ ] The PR description explicitly calls out any new storage keys or event
      topics being introduced, with their Symbol values.
- [ ] If the PR adds both a new storage key AND a new event topic, the Symbol
      values are verified to be different from each other.

### Related

- [docs/RESERVED_KEYS_POLICY.md](docs/RESERVED_KEYS_POLICY.md) — Reserved key prefix conventions
- [docs/EVENT_TOPIC_COMPATIBILITY.md](docs/EVENT_TOPIC_COMPATIBILITY.md) — Event topic compatibility policy
- [SC-099: Event-Topic & Payload Schema Contributor Safety Checklist](#sc-099-event-topic--payload-schema-contributor-safety-checklist) — Broader event schema safety
- [SC-098: Security Review Checklist for Privileged Changes](#sc-098-security-review-checklist-for-privileged-changes) — Security review for privileged fn changes
---

**Happy coding! 🚀**
