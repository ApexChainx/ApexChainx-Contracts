# ApexChainx System — Project Context

> **Purpose:** This document describes the high-level system architecture, repository landscape,
> and future contract roadmap for the ApexChainx platform.

## Table of Contents

- [Repository Architecture](#repository-architecture)
- [System Flow](#system-flow)
- [Architectural Rules](#architectural-rules)
- [SC-100: Future Contract Roadmap](#sc-100-future-contract-roadmap)

---

## Repository Architecture

The ApexChainx platform is composed of three repositories:

| Repository | Role | Technology |
|------------|------|------------|
| `apexchainx-fe` | Frontend application | React / TypeScript |
| `apexchainx-be` | Backend API and integration layer | Python / FastAPI |
| `apexchainx-contracts` | Soroban smart contracts (this repo) | Rust / Soroban SDK |

## System Flow

```
 User
  |
  v
┌─────────┐     ┌─────────┐     ┌──────────────┐
│   FE    │ ──→ │   BE    │ ──→ │  Contracts   │
│ (React) │ ←── │ (API)   │ ←── │  (Soroban)   │
└─────────┘     └─────────┘     └──────────────┘
```

## Architectural Rules

1. **Frontend never calls contracts directly** — all contract interactions go through the backend
2. **Backend is the exclusive bridge** — translates contract data to frontend-friendly responses
3. **Contracts are execution-layer only** — pure deterministic computation, no external dependencies

---

## Telemetry & Weekly Reset Semantics

The `apexchainx_calculator` contract maintains per-severity telemetry (`SeverityTelemetry`) tracking calculation counts, violation counts, and violation rates.

### Operator Posture & Reset Behavior

1. **Lazy On-Execution Evaluation**:
   - The contract checks the timestamp of the last calculation/violation for the invoked severity lane when `calculate_sla` is called.
   - If $\ge 7$ days ($604,800$ seconds) have passed since the recorded timestamp for that severity lane, the calculation and violation counters for that specific lane are reset to `0` before processing the current calculation.

2. **Per-Severity Isolation**:
   - Resets are evaluated per severity lane (`critical`, `high`, `medium`, `low`).
   - Activity in one lane does not reset or refresh timestamps for other lanes.

3. **Impact on Backend Consumers and Monitoring Dashboards**:
   - `get_severity_telemetry()` reflects stored counters. Inactive lanes retain their last updated telemetry state until the next invocation in that lane triggers a lazy reset.
   - Replays and duplicate resubmissions with identical inputs/configs do NOT update or reset telemetry counters.
   - Off-chain monitoring systems or backend consumers desiring continuous 7-day rolling window analytics should aggregate on-chain `EVENT_SLA_CALC` events or poll `get_severity_telemetry()` periodically alongside contract calls.

---

## SC-100: Future Contract Roadmap

This section documents the planned evolution of `apexchainx-contracts` based on
current backend integration needs and business requirements.

### Versioning Strategy

| Version | Scope | Timeline |
|---------|-------|----------|
| v1.0 | Single crate (`apexchainx_calculator`) | ✅ Current |
| v1.1 | Multi-contract version negotiation | ✅ Current |
| v2.0 | Payment escrow integration | Planned |
| v2.1 | Multi-party settlement | Planned |
| v3.0 | On-chain governance with timelocks | Planned |

### Current State

Only one contract crate exists in this repository:

| Crate | Status | Description | Key Features |
|-------|--------|-------------|--------------|
| `apexchainx_calculator` | **Production-ready** | SLA calculator contract | Config management, role-based auth, event emission, version negotiation, result schema |

### Planned Additions

The following crates are planned but **not yet implemented**. Do not import or
reference them until they appear in the repository.

| Crate | Status | Depends On | Description |
|-------|--------|------------|-------------|
| `payment_escrow` | Planned | `apexchainx_calculator` | Locks and conditionally releases Stellar token payments based on SLA results |
| `settlement` | Planned | `payment_escrow` | Splits shared outage costs between multiple parties |
| `governance` | Planned | — | On-chain admin config changes with time-locked execution |

### Integration Expectations

- The backend (`apexchainx-be`) currently integrates only with `apexchainx_calculator`
- New crates will be introduced incrementally
- Each new crate must expose a `get_result_schema()` equivalent for safe version pinning
- Frontend never calls contracts directly — all invocations go through the backend

### Contribution Guidelines for New Crates

1. **Open a tracking issue** before creating the crate directory
2. **Follow the established layout**: `src/lib.rs`, `src/tests.rs`, `Cargo.toml`
3. **Add to CI matrix** in `.github/workflows/`
4. **Export a result schema** function so the backend can detect breaking changes
5. **Include version negotiation** support for multi-contract compatibility
