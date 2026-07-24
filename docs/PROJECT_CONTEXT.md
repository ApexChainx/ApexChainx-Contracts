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
| `apexchainx-contracts` | Soroban smart contracts (this repo) | Rust / Soroban SDK 21.1.0 |

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
