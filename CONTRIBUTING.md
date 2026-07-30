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
- [Documentation Guidelines](#documentation-guidelines)
- [Security Guidelines](#security-guidelines)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)
- [Getting Help](#getting-help)

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
# Install Soroban CLI if you haven't
cargo install --locked soroban-cli
# Build contracts
make build
# Run tests
make test
```

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
- **Documentation:** All public functions must have doc comments

#### Style Rules

| Rule | Standard |
|------|----------|
| Naming | `snake_case` for functions/variables, `PascalCase` for types |
| Error handling | Custom error types via `#[contracterror]` |
| Imports | Group: std → external crates → internal modules |
| Formatting | `cargo fmt` (automated) |
| Linting | `cargo clippy -- -D warnings` (no warnings allowed) |

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
This covers:

- **[SC-500] `#[contracttype]` Compatibility Note Policy** (#279) — every public
  contract type change must include a compatibility note in the PR.
- **[SC-501] Response-Shape Stability Policy** (#283) — every `#[contracttype]`
  return type is assigned a stability tier (Stable, Versioned, Experimental).
- **[SC-502] Version Negotiation Protocol Note** (#284) — safe vs. breaking
  changes to `get_version_info()`.
- **[SC-503] API Archetype Note** (#285) — three function archetypes: Read-Only,
  Mutating (Operator), Privileged (Admin).
- **[SC-504] Event Payload Size Check** (#286) — payload size assertions required
  for every event change.
- **[SC-505] Event Drift Review Note** (#287) — checklist for event name/payload
  changes.
- **[SC-506] History Write Audit Check** (#288) — FIFO ordering, pruning, and
  idempotency invariants.
- **[SC-507] Telemetry Counters Policy** (#289) — additive-only counter fields,
  saturation handling.
- **[SC-508] Role-Change Incident Review Note** (#290) — admin/operator handoff
  safety decision table.

### Release Summary Generator (#280)

Run `npx tsx tooling/release-summary.ts` to generate a maintainer-friendly
release summary from the `[Unreleased]` section of `CHANGELOG.md`. Use `--json`
for CI or `--check` for format validation.

### Devcontainer Setup (#281)

A `.devcontainer/` setup is provided for GitHub Codespaces and VS Code Dev
Containers. The devcontainer includes Rust, the `wasm32-unknown-unknown` target,
`just`, and Node.js. Run `just bootstrap` to set up your local environment.

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

**Happy coding! 🚀**
