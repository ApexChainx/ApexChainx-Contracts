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

### What stops an operator from spamming the same outage ID?

`calculate_sla` is idempotent: resubmitting an outage with an unchanged config
hash and identical inputs returns the stored result and writes nothing — no
history entry, no statistics, no telemetry, no events — so retries are safe and
cannot skew reported violation rates. Resubmitting the *same* outage with
different inputs is rejected (`DuplicateOutageInput`), and a config change opens
a new stored generation for that outage, capped at 16 retained entries
(`OutageRecalcLimit`) so one outage cannot crowd others out of the retention
window. Admin pruning frees that headroom again.

### Is the contract upgradeable?

No. The contract is not natively upgradeable. Upgrades require deploying a new
contract and migrating state through the backend.

> **Soroban-based SLA calculator and multi-contract coordination suite for the Stellar network.**

This repository is the execution-layer side of the 3-repo architecture.

## Related Repositories

| Repository | Description |
|------------|-------------|
| [apexchainx-fe](https://github.com/ApexChainx/apexchainx-fe) | Frontend application (React/TypeScript) |
| [apexchainx-be](https://github.com/ApexChainx/apexchainx-be) | Backend API and contract bridge |

## Development Setup

### Quick Start with Dev Container (#281)

A [dev container](.devcontainer/) is provided for GitHub Codespaces and VS Code:

```bash
# Open in Codespaces or VS Code — the devcontainer auto-configures:
# - Rust toolchain + wasm32-unknown-unknown target
# - just command runner
# - Node.js + npx for tooling scripts
```

### Local Setup

```bash
# Bootstrap the dev environment
just bootstrap

# Run the full CI pipeline locally
just ci
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed setup instructions.

## Security & Supply Chain

- **[Release Artifact Provenance Policy](docs/RELEASE_PROVENANCE_POLICY.md)** — Guidelines for WASM output checksums and snapshot check-ins.
- **Dependency auditing:** `cargo audit` runs on CI for every push
- **WASM integrity:** Release artifacts include SHA-256 manifests
- **Reproducible builds:** Local builds can be verified against CI-generated manifests
