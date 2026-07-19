# ApexChainx Smart Contracts — Codex Context

> **Technical deep-dive:** Architecture, constraints, event conventions, and
> integration guidance for the ApexChainx Soroban smart contract ecosystem.

<p align="center">
  <img src="https://img.shields.io/badge/last_updated-2026-green" alt="Last Updated: 2026">
  <img src="https://img.shields.io/badge/status-maintained-success" alt="Status: Maintained">
  <img src="https://img.shields.io/badge/audience-developers-blue" alt="Audience: Developers">
</p>
> integration guidance for the ApexChainx Soroban smart contract ecosystem.

## Table of Contents

- [Overview](#overview)
- [Technology Stack](#technology-stack)
- [Core Contracts](#core-contracts)
- [Architecture](#architecture)
- [Constraints & Design Principles](#constraints--design-principles)
- [Critical Logic: SLA Calculation](#critical-logic-sla-calculation)
- [Risk Assessment](#risk-assessment)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Cross-Repo Dependencies](#cross-repo-dependencies)
- [Backend-Facing Result Schema](#backend-facing-result-schema)
- [Event Conventions](#event-conventions)
- [SC-097: Event Replay & Recovery](#sc-097-event-replay--recovery)

---

## Overview

This repository contains Soroban smart contracts that power the ApexChainx
platform. These contracts execute on the **Stellar network** and are invoked
exclusively through the backend API layer.

### Primary Responsibilities

| Function | Description |
|----------|-------------|
| SLA Calculation | Deterministic penalty/reward computation based on service metrics |
| Payment Escrow | Lock and conditionally release Stellar token payments |
| Multi-Party Settlement | Split shared outage costs between multiple parties |

### Invocation Model

```
Backend API → Contract Invocation → Result Processing → Payment Execution
```

**Key constraint:** Contracts are never called directly by the frontend.
All interactions go through the backend bridge.

---

## Core Contracts

### 1. SLA Calculator (Production-Ready)

**Status:** ✅ Implemented and tested

The SLA calculator is the primary contract in this repository. It handles
deterministic SLA computation, configuration management, and event emission.

#### Responsibilities

| Responsibility | Details |
|---------------|---------|
| SLA Computation | Calculate penalty or reward based on severity and MTTR |
| Configuration | Store and manage severity-based SLA parameters |
| Governance | Admin/operator role management with two-step transfers |
| Events | Versioned lifecycle events for backend consumers |
| History | On-chain calculation history with pruning |

#### Key Functions

| Function | Auth | Purpose |
|----------|------|---------|
| `initialize` | — | One-time setup with admin/operator roles |
| `calculate_sla` | Operator | Full SLA computation (mutating) |
| `calculate_sla_view` | Public | Read-only SLA simulation |
| `set_config` | Admin | Update severity configuration |
| `get_config_snapshot` | Public | Ordered config export for backend |

#### Critical Constraints

- ✅ Deterministic — same inputs always produce the same output
- ✅ Self-contained — no external state dependencies
- ✅ Backend-parity — must match backend SLA logic exactly

---

### 2. Payment Escrow (Planned)

**Status:** 📋 Not yet implemented

Future contract for locking and conditionally releasing Stellar token payments
based on SLA results.

#### Planned Responsibilities

| Operation | Description |
|-----------|-------------|
| `create_escrow` | Lock funds in escrow with SLA conditions |
| `release_escrow` | Release funds when SLA conditions are met |
| `refund_escrow` | Return funds when SLA conditions are violated |

---

### 3. Multi-Party Settlement (Planned)

**Status:** 📋 Not yet implemented

Future contract for splitting shared outage costs between multiple parties.

#### Planned Responsibilities

| Operation | Description |
|-----------|-------------|
| `create_settlement` | Initiate a multi-party cost split |
| `execute_settlement` | Process and distribute payments |

---

## Architecture

### Design Principles

| Principle | Description |
|-----------|-------------|
| **Stateless** | Contracts minimize on-chain state; configuration is the primary persisted data |
| **Deterministic** | Same inputs always produce identical outputs — no randomness |
| **Backend-Mediated** | All contract invocations flow through the backend API layer |
| **Network-Validated** | Stellar consensus validates all contract executions |

### Execution Flow

```
Backend  ──→  Contract Invocation  ──→  Result Processing  ──→  Payment Execution
    ↑                                                                     |
    └─────────────────────  Event Stream Replay  ←─────────────────────────┘
```

---

## Constraints & Design Principles

### Determinism

All contract computations must be **fully deterministic**. This is non-negotiable
because:
- Backend and contract logic must produce identical results
- Event replay depends on reproducible outcomes
- Audit trails require verifiable computation

### Integer Math

```
❌ Floating point:  NOT ALLOWED
✅ Integer math:    ALWAYS REQUIRED
```

No floating point operations. All calculations use integer arithmetic with
appropriate precision scaling.

### Gas Optimization

| Strategy | Rationale |
|----------|-----------|
| Minimize storage writes | Each write consumes significant gas |
| Avoid loops over unbounded data | Gas costs scale with iteration count |
| Use view functions for reads | Read-only calls have no gas cost |
| Batch operations where possible | Reduce per-operation overhead |

### Idempotency

Contracts must be idempotent where applicable:
- Re-processing the same SLA calculation returns the same result
- Duplicate event consumption does not produce errors
- Configuration updates are idempotent for same parameters

### Input Validation

All function inputs are validated at the contract boundary before any state
changes occur:
- Severity levels are checked against supported values
- Thresholds and penalties are validated as positive integers
- Addresses are verified for format correctness
- Bounds checking on all numeric parameters

---

## Critical Logic: SLA Calculation

### Computation Model

The SLA calculation is the core deterministic function that determines whether
service targets were met and computes the corresponding financial outcome.

#### Input Parameters

| Parameter | Type | Description | Source |
|-----------|------|-------------|--------|
| `severity` | `Severity` | Incident severity level | Contract caller |
| `mttr_minutes` | `u32` | Measured time to repair (minutes) | Contract caller |
| `threshold_config` | `Config` | Severity-specific threshold parameters | On-chain storage |

#### Output Values

| Field | Type | Description | Possible Values |
|-------|------|-------------|-----------------|
| `status` | `SLAStatus` | Whether SLA target was met | `met`, `violated` |
| `amount` | `i64` | Financial outcome (positive or negative) | Signed integer |
| `payment_type` | `PaymentType` | Type of financial outcome | `reward`, `penalty` |
| `rating` | `Rating` | Performance rating | `top`, `excel`, `good`, `poor` |

#### Determinism Requirement

The SLA computation must produce **exactly identical results** when executed in
the contract and in the backend. This is enforced through:

1. **Golden test vectors** shared between contract and backend
2. **CI parity checks** that compare contract output against backend expectations
3. **Integer-only math** eliminating floating-point divergence

---

## Risk Assessment

### Risk Matrix

| Category | Risk | Severity | Mitigation |
|----------|------|----------|------------|
| **SLA Logic** | Backend-contract mismatch | High | Golden test vectors, parity CI checks |
| **SLA Logic** | Integer precision errors | Medium | Use only integer math, test boundary conditions |
| **SLA Logic** | Edge cases (boundary MTTR) | Medium | Comprehensive boundary test suite |
| **Payments** | Double execution | High | Idempotency keys, outage_id deduplication |
| **Payments** | Missing authorization | Critical | require_auth() on all privileged functions |
| **Payments** | Wrong recipient | High | Address validation, two-step confirmation |
| **Security** | Admin privilege misuse | Critical | Two-step transfers, audit events, renounce safety |
| **Security** | Initialization errors | High | One-time init guard, verify-after-init |
| **Security** | Unauthorized config changes | High | Role-based access control, event emission |
| **Gas** | Unnecessary storage writes | Medium | Use view functions for reads, batch writes |
| **Gas** | Inefficient loops | Medium | Bound data structures, paginate history |
| **Gas** | Repeated computation | Low | Cache results where possible |

---

## Coding Standards

### Mandatory Rules

| Rule | Rationale |
|------|-----------|
| Integer math only | Floating point is non-deterministic and gas-expensive |
| Validate all inputs | Prevent invalid state transitions |
| Emit events for state changes | Enable backend audit and replay |
| Keep functions small | Improve auditability and testability |
| Avoid unnecessary state writes | Minimize gas costs |
| Use require_auth() for privileged ops | Enforce role-based access control |

### Code Organization

```
contract_name/
├── Cargo.toml
└── src/
    ├── lib.rs           # Contract entry point, storage keys, error types
    ├── tests.rs         # Integration tests
    ├── version_negotiation.rs  # Multi-contract versioning
    ├── storage_version.rs      # Schema versioning
    ├── event_schema.rs         # Canonical event definitions
    └── ...              # Additional domain modules
```

---

## Testing Requirements

- unit tests for each function
- edge case tests
- integration tests with backend expectations
- deterministic output validation

---

## Cross-Repo Dependencies

- apexchainx-be → invokes contracts
- apexchainx-fe → displays results

Important:

- contract logic must never diverge from backend expectations
- API response structure depends on contract output
- result symbol mappings are versioned through the contract-facing schema

## Backend-Facing Result Schema

The SLA calculator now exposes an explicit result schema contract so the backend
does not have to infer symbol meanings implicitly.

Current schema version:

- schema label: `v1`
- schema version: `1`

Current symbol mappings:

- status met -> `met`
- status violated -> `viol`
- payment reward -> `rew`
- payment penalty -> `pen`
- rating exceptional -> `top`
- rating excellent -> `excel`
- rating good -> `good`
- rating poor -> `poor`

Compatibility rule:\n\n- additive read-only contract helpers are preferred over changing the shape of\n  `SLAResult`\n- **Versioning**: Breaking ABI/symbol changes → MAJOR bump (v2.0.0), update `schema_version` in `get_result_schema()`.\n- Backend: Pin contract ID/version, regenerate parity tests from snapshots post-release.\n\n**Backend Dependency Expectations**:\n- Match `calculate_sla_view()` exactly with local logic.\n- Consume `test_snapshots/tests/*.json` for golden vectors.\n- Monitor git tags `vX.Y.Z` for releases.\n\n## Event Conventions

For the canonical reference on Soroban event commit timing, event semantics, and
full event catalog details, see [docs/AUDIT_TRAIL.md](AUDIT_TRAIL.md). This
context document keeps only the repository-specific conventions and points
readers to that authoritative explanation instead of restating the full catalog.

The local event topic convention remains:

- topic 0 -> event name
- topic 1 -> event version
- topic 2 -> event-specific context such as severity or caller

---

## SC-097: Event Replay & Recovery Guidance

### Event Consumption Model

Backend consumers should treat the SLA calculator's on-chain events as a
**supplementary audit trail**, not as the primary source of truth. The
canonical state is always the most recent `calculate_sla` result stored
on-chain and retrieved via direct contract reads.

### Event Replay Assumptions

| Assumption | Implication |
|------------|-------------|
| Versioned topics | Check `topic[1]` before deserialising to avoid version mismatches |
| Non-guaranteed presence | Events may not exist for every ledger — handle gaps gracefully |
| Idempotent replay | Use `outage_id` as deduplication key for re-processing |

### Missed-Event Recovery Procedure

1. **Detect gap** — Compare last processed ledger sequence against current `getLatestLedger`
2. **Replay** — Use `getEvents` with explicit `startLedger` to fetch missed events in order
3. **Cross-check** — Verify replayed results against `calculate_sla_view` for the same `outage_id`
4. **Alert** — Log any discrepancy between event payload and on-chain state as potential double-execution risk

### Canonical State vs Event-Stream Interpretation

| Operation | Recommended Source | Reasoning |
|-----------|-------------------|-----------|
| Current SLA result | `calculate_sla_view` | Most recent on-chain state |
| Historical audit | Event stream replay | Chronological event log |
| Config at point-in-time | `cfg_upd` events + `get_config` | Config change events + current state |
| Payment amounts | Event payload `amount` field | Signed integer in event |

---

## Documentation Goals

This document serves as a technical reference for:

| Goal | Audience |
|------|----------|
| Improve contract correctness | Developers, auditors |
| Ensure payment security | Backend operators, security reviewers |
| Optimize gas usage | Contract developers |
| Guarantee deterministic behavior | QA engineers, backend integrators |
