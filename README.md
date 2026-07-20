<p align="center">
  <img src="https://img.shields.io/badge/status-active-success.svg" alt="Status: Active">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT">
  <img src="https://img.shields.io/badge/version-0.1.0-blueviolet" alt="Version: 0.1.0">
  <img src="https://img.shields.io/badge/Soroban_SDK-21.0.0-important" alt="Soroban SDK: 21.0.0">
  <img src="https://img.shields.io/badge/rustc-stable-success" alt="Rust: stable">
  <img src="https://img.shields.io/badge/platform-Stellar_Network-000" alt="Platform: Stellar Network">
  <a href="https://codecov.io/gh/ApexChainx/ApexChainx-Contracts"><img src="https://codecov.io/gh/ApexChainx/ApexChainx-Contracts/branch/main/graph/badge.svg" alt="Coverage"></a>
</p>

# ApexChainx Smart Contracts

## Frequently Asked Questions

### What is ApexChainx?

ApexChainx is a smart contract platform built on the Stellar network for
deterministic SLA (Service Level Agreement) calculation, payment escrow,
and multi-party settlement.

### What blockchain does this use?

These contracts run on the **Stellar network** using the **Soroban** smart
contract platform.

### How is SLA calculated?

The contract takes severity level, measured MTTR (Mean Time To Repair), and
configured thresholds to determine whether SLA targets were met. Results include
status (met/violated), payment type (reward/penalty), and rating.

### Can I call contracts directly from the frontend?

**No.** All contract invocations must go through the backend API layer. The
frontend never interacts with contracts directly.

### How are contract upgrades handled?

The contract includes a version negotiation protocol (`get_version_info()`) that
allows backends to verify compatibility before deployment.

### Is the contract upgradeable?

No. The contract is not natively upgradeable. Upgrades require deploying a new
contract and migrating state through the backend.

> **Soroban-based SLA calculator and multi-contract coordination suite for the Stellar network.**

This repository is the execution-layer side of the 3-repo architecture:

- **apexchainx-fe** — Frontend application (React/TypeScript)
- **apexchainx-be** — Backend API and contract integration layer
- **apexchainx-contracts** — Soroban smart contracts (this repository)

## System Architecture

### Data Flow

```
  User
   |
   v
┌─────────┐     ┌─────────┐     ┌──────────────┐
│   FE    │ ──→ │   BE    │ ──→ │  Contracts   │
│ (React) │ ←── │ (API)   │ ←── │  (Soroban)   │
└─────────┘     └─────────┘     └──────────────┘
```

### Architectural Rules

1. **Frontend never calls contracts directly** — all contract invocations go through the backend API layer.
2. **Backend is the sole bridge** — responsible for translating contract results into frontend-friendly responses.
3. **Contracts are execution-layer only** — pure deterministic computation with no external dependencies.

## Overview

`apexchainx-contracts` contains the Soroban-side SLA logic for the ApexChainx platform.

At the current checked-in state, this repository contains one active contract crate:

| Crate | Description |
|-------|-------------|
| `apexchainx_calculator` | Deterministic SLA calculator with config management, statistics, and history |

This contract is responsible for deterministic SLA calculation and related contract-side state such as configuration, statistics, pause state, and calculation history.

## Technology Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Language | Rust | stable (edition 2021) |
| Framework | Soroban SDK | 21.0.0 |
| Blockchain | Stellar Network | — |
| Build System | Cargo + wasm32 target | — |
| Testing | Soroban testutils + cargo test | — |
| Standards | #![no_std], WASM-compatible | — |

**Main crate manifest:**

- `apexchainx_calculator/Cargo.toml`

## Contract API Reference

The active contract implementation is in:

- `apexchainx_calculator/src/lib.rs`
- `apexchainx_calculator/src/tests.rs` (test suite)

### Core Functions

| Function | Auth | Description |
|----------|------|-------------|
| `initialize(admin, operator)` | — (once) | One-time setup with role assignment |
| `set_config(caller, severity, threshold, penalty, reward)` | Admin | Update severity configuration |
| `get_config(severity)` | Public | Read a single severity config |
| `get_config_snapshot()` | Public | Ordered snapshot of all severity configs |
| `calculate_sla(caller, outage_id, severity, mttr)` | Operator | Execute SLA calculation (mutating) |
| `calculate_sla_view(outage_id, severity, mttr)` | Public | Simulate SLA calculation (read-only) |

### Governance Functions

| Function | Auth | Description |
|----------|------|-------------|
| `propose_admin(caller, new_admin)` | Admin | Initiate two-step admin transfer |
| `accept_admin(caller)` | Proposed | Complete admin transfer |
| `cancel_admin_proposal(caller)` | Admin | Cancel pending admin proposal |
| `propose_operator(caller, new_operator)` | Admin | Initiate operator rotation |
| `accept_operator(caller)` | Proposed | Complete operator rotation |
| `cancel_operator_proposal(caller)` | Admin | Cancel pending operator proposal |
| `renounce_admin(caller)` | Admin | Irreversibly remove admin authority |

### State & Utility Functions

| Function | Auth | Description |
|----------|------|-------------|
| `pause(caller, reason)` | Admin | Pause contract with metadata |
| `unpause(caller)` | Admin | Resume contract operations |
| `get_paused()` | Public | Check pause state |
| `get_pause_info()` | Public | Get pause metadata |
| `get_stats()` | Public | Read cumulative SLA statistics |
| `get_history()` | Public | Paginated SLA calculation history |
| `prune_history(caller)` | Admin | Compact on-chain history storage |
| `get_result_schema()` | Public | Versioned result schema descriptor |
| `get_config_version_hash()` | Public | Deterministic config fingerprint |
| `get_version_info()` | Public | Version negotiation metadata |

## Project Structure

```text
apexchainx-contracts/
├── .github/
│   └── workflows/
│       ├── ci.yml                 # CI pipeline: fmt, clippy, test, wasm build
│       ├── release-hash.yml       # Release artifact SHA-256 manifest
│       └── security.yml           # cargo-audit dependency scanning
├── apexchainx_calculator/         # Core contract crate
│   ├── Cargo.toml
│   ├── test_snapshots/
│   │   └── tests/                 # Golden test vectors for backend parity
│   ├── src/
│   │   ├── lib.rs                 # Contract entry point & storage
│   │   ├── tests.rs               # Integration test suite
│   │   ├── version_negotiation.rs # Multi-contract versioning protocol
│   │   ├── storage_version.rs     # Storage schema versioning
│   │   ├── event_schema.rs        # Canonical event definitions
│   │   ├── event_correlation.rs   # Event grouping and correlation
│   │   ├── topic_stability_tests.rs
│   │   ├── payload_versioning_tests.rs
│   │   └── ...                    # Additional test and utility modules
│   └── src/
├── artifacts/                     # Test artifacts and snapshots
├── docs/                          # Project documentation
│   ├── CODEX_CONTEXT.md
│   ├── PROJECT_CONTEXT.md
│   ├── config-validation.md
│   └── sc-w5-storage-and-cost-baselines.md
├── offchain/                      # Off-chain helper scripts
├── scripts/                       # Build, test, and utility scripts
├── tests/                         # TypeScript/integration test suites
├── tooling/                       # Release, CI, and security tooling
├── tools/                         # Analytical and utility tools
├── ts/                            # TypeScript helpers for governance, config
├── Cargo.toml                     # Workspace manifest
├── CONTRIBUTING.md
├── CHANGELOG.md
├── README.md
├── SECURITY.md
├── TODO.md
└── pers-store/                    # Persistent storage test fixtures
```

## What Is Actually In This Repo

Only the SLA calculator contract is currently checked in.

That means this repo does not currently contain:

- `payment_escrow`
- `multi_party_settlement`
- deployment scripts
- a top-level Cargo workspace

If those are planned, they are future work rather than part of the present repository state.

## Local Setup

### Prerequisites

- Rust toolchain
- Cargo
- optional: Soroban CLI for deployment workflows

### Environment Configuration

To configure the environment variables for this project, copy the template `.env.example` to `.env`:

```bash
cp .env.example .env
```

And then edit the `.env` file with your specific configurations. This file is ignored by Git to prevent committing secrets:
- `SOROBAN_RPC_URL`: The RPC endpoint for the Stellar/Soroban network.
- `SOROBAN_NETWORK_PASSPHRASE`: Passphrase of the target network.
- `DEPLOYER_SECRET_KEY`: Private key/secret key for deploying contracts.
- `CONTRACT_ID`: The ID of the deployed smart contract.


### Run Tests\n\n```bash\ncd apexchainx_calculator\ncargo test\n```\n\n### Test Vector Artifacts for Backend Parity\n\nRun `cargo test` to generate/update canonical SLA test vectors as JSON snapshots:\n\n```\napexchainx_calculator/test_snapshots/tests/*.json\n```\n\n**Key Vectors**:\n- `test_backend_parity_threshold_boundary_cases.*.json`: SLA met/viol boundaries\n- `test_backend_parity_reward_tier_cases.*.json`: Reward tiers (top/excel/good)\n- `test_stress_1000_calculations_mixed_severities.*.json`: Performance aggregates\n- `test_config_snapshot_is_deterministic_and_complete.*.json`: Full config\n\n**Backend Usage**:\n1. Consume snapshots for parity tests: Input (severity/mttr) → match contract `calculate_sla_view`\n2. Use `get_config_snapshot()` + `get_result_schema()` for schema validation.\n3. Maintenance: `cargo test` after SLA changes → snapshots auto-update.\n\nVectors ensure contract/backend parity without manual duplication.\n\n### Build The Contract\n\n```bash\ncd apexchainx_calculator\ncargo build\n```\n\n### Build WASM\n\n```bash\ncd apexchainx_calculator\ncargo build --target wasm32-unknown-unknown --release\n```\n\nExpected artifact:\n\n- `apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm`\n\n## Deploy-Oriented Workflow

The current repository does not ship deployment scripts, but the existing crate
is ready for a manual Soroban deployment flow.

### 1. Build the release WASM

```bash
cd apexchainx_calculator
cargo build --target wasm32-unknown-unknown --release
```

### 2. Deploy the contract

Example:

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm \
  --source-account <source-account> \
  --network <network-name>
```

Save the returned contract ID for later invocation.

### 3. Initialize the contract

The current `initialize` function accepts:

- `admin: Address`
- `operator: Address`

Example:

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- initialize \
  --admin <admin-address> \
  --operator <operator-address>
```

### 4. Read contract state

Useful follow-up calls after deployment:

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- get_config \
  --severity critical
```

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- get_stats
```

## Artifact Guidance

For this repository, the main artifact contributors and operators should expect is:

- release WASM for deployment:
  `apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm`

Optional local outputs include:

- debug build artifacts under `apexchainx_calculator/target/debug`
- test binaries under `apexchainx_calculator/target/debug/deps`

## Release Artifact Hash Manifest (SC-003)

Every CI run and release tag produces a `manifest.sha256` file alongside the
WASM artifact. The manifest contains the SHA-256 hash of `apexchainx_calculator.wasm`
in standard `sha256sum` format:

```
<sha256hex>  apexchainx_calculator.wasm
```

### Verify a local build matches the recorded manifest

```bash
# 1. Build the release WASM locally
cd apexchainx_calculator
cargo build --release --target wasm32-unknown-unknown

# 2. Download manifest.sha256 from the corresponding CI run or GitHub Release

# 3. Copy the WASM next to the manifest and verify
cp target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm .
sha256sum -c manifest.sha256
# Expected output: apexchainx_calculator.wasm: OK
```

### Generate a manifest locally

```bash
cd apexchainx_calculator
cargo build --release --target wasm32-unknown-unknown
sha256sum target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm \
  | awk '{print $1 "  apexchainx_calculator.wasm"}' > manifest.sha256
cat manifest.sha256
```

The `release-hash` workflow (`.github/workflows/release-hash.yml`) runs
automatically on every push to `main`, every PR, and every `v*` tag. On tag
pushes the manifest and WASM are attached to the GitHub Release.

## Build Verification

### Current Status

| Check | Status |
|-------|--------|
| `cargo test` | ✅ Passing |
| Crate compilation | ✅ Clean |
| Test suite bindings | ✅ Wired and functional |

### no-std Compliance

Soroban contracts execute inside a WASM sandbox with no operating system and no
Rust standard library. The crate is declared `#![no_std]` to enforce this at
the source level. However, `cargo test` on the host re-enables `std` via the test
harness — so a stray `use std::vec::Vec` or `println!` would compile fine in
tests yet fail at deployment.

#### Compliance Check

The CI pipeline includes a dedicated **no-std compliance check**:

```bash
cargo check --target wasm32-unknown-unknown --lib
```

This compiles only the library crate (not the test harness) for the
`wasm32-unknown-unknown` target (no `std`). Any accidental `std` import
surfaces as a compile error before it can reach a deployed contract.

### Test Coverage

The current test suite covers the following domains:

| Domain | Description |
|--------|-------------|
| Authorization | Role-based access control, admin/operator permissions |
| SLA Logic | Reward and penalty calculation with boundary conditions |
| Pause/Unpause | Contract lifecycle management |
| Statistics | Cumulative tracking of SLA calculations |
| Audit Parity | Read-only calculation mode matches mutating mode |
| History | Event recording, retrieval, and pruning operations |

## Backend Integration

The backend repository (`apexchainx-be`) invokes this contract and translates
contract results into backend API responses. This relationship imposes several
critical constraints.

### Integration Contract

| Requirement | Rationale |
|-------------|----------|
| SLA logic alignment | Contract and backend must produce identical results for identical inputs |
| Deterministic encoding | Result encoding must be stable across invocations |
| Single source of truth | API consumers see only what the backend returns |
| Snapshot-style reads | Config reads should prefer explicit snapshot views for stable ordering |

### Backend Dependencies

- Match `calculate_sla_view()` exactly with local business logic
- Consume `test_snapshots/tests/*.json` as golden vectors
- Monitor git tags (`vX.Y.Z`) for contract releases
- Use `get_config_snapshot()` + `get_result_schema()` for schema validation

## Current Limitations

This repository is stable at the crate level but the overall contract layer is still narrow:

- Only one contract crate exists (`apexchainx_calculator`)
- No deployment automation checked in
- No broader contract workspace with escrow or settlement modules
- Cross-repo contract invocation is handled by the backend

## Governance Model

The contract implements a role-based governance model with two-step transfers for
security and an irreversible renounce mechanism.

### Role Architecture

| Role | Authority | Set By |
|------|-----------|--------|
| **Admin** | Config updates, governance, pause, prune | `initialize()` |
| **Operator** | SLA calculation execution | Admin |

### Admin Transfer (Two-Step)

Admin authority is transferred via a two-step flow to prevent accidental reassignment:

1. **Propose:** Current admin calls `propose_admin(caller, new_admin)` — stores a pending proposal
2. **Accept:** Proposed admin calls `accept_admin(caller)` — atomically promotes caller to admin

The old admin retains authority until `accept_admin` succeeds. `get_pending_admin()`
is queryable at any time.

**Cancellation:** To cancel a stale or mistaken proposal, the current admin calls
`cancel_admin_proposal(caller)`. This clears the pending proposal without changing
the active admin. A fresh proposal can be issued immediately.

### Operator Handoff (Two-Step)

Operator rotation follows the same pattern:

1. **Propose:** Admin calls `propose_operator(caller, new_operator)`
2. **Accept:** New operator calls `accept_operator(caller)` to activate

`get_pending_operator()` exposes the pending state for governance visibility.

**Cancellation:** `cancel_operator_proposal(caller)` clears a pending operator
proposal. The active operator is unchanged.

### Admin Renounce

`renounce_admin(caller)` permanently removes admin authority. This operation is
**irreversible**:

- All admin-gated functions (`set_config`, `pause`, `unpause`, `set_operator`, `prune_history`) are permanently locked
- Any pending admin proposal is cleared atomically
- No recovery path exists by design

> **⚠️ Warning:** A renounced contract should be treated as immutable. If recovery
> is needed, redeploy and reinitialize the contract.

### Pause Metadata

`pause(caller, reason)` stores a `PauseInfo` struct containing:

- `reason`: Human-readable explanation string
- Timestamp: Ledger timestamp at pause time

`get_pause_info()` returns this metadata while the contract is paused.
`unpause()` clears it. This gives backend operators operational context without
requiring off-chain state tracking.

## Code of Conduct

This project adheres to a code of conduct that all contributors are expected to follow. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## Security Considerations

Please refer to our [Security Policy](SECURITY.md) for information on supported versions, our disclosure policy, and how to report a vulnerability.

### Smart Contract Security

- **Deterministic execution:** All calculations use integer math only — no floating point
- **Input validation:** All function inputs are validated before state changes
- **Access control:** Privileged operations require `require_auth()` with role verification
- **Reentrancy protection:** Contract design prevents reentrant calls
- **Pause mechanism:** Admin can pause the contract in case of emergency

### Operational Security

- **Two-step transfers:** Admin and operator role changes require confirmation
- **Renounce safety:** Admin renounce is irreversible — use with caution
- **Event audit trail:** All state changes emit versioned events for backend consumers
- **Deterministic failure:** Same invalid inputs always produce same errors

### Supply Chain Security

- **Dependency auditing:** `cargo audit` runs on CI for every push
- **WASM integrity:** Release artifacts include SHA-256 manifests
- **Reproducible builds:** Local builds can be verified against CI-generated manifests

## Related Repositories

| Repository | Description |
|------------|-------------|
| [apexchainx-fe](https://github.com/ApexChainx/apexchainx-fe) | Frontend application (React/TypeScript) |
| [apexchainx-be](https://github.com/ApexChainx/apexchainx-be) | Backend API and contract bridge |
