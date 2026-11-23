# Storage Footprint & Rent Estimation Policy

> **Audience:** Smart contract maintainers, node operators, and backend system administrators.

## Overview

Soroban smart contracts incur on-chain storage rent costs based on the number of stored entries and total byte footprint. To allow proactive monitoring and prevent out-of-funds storage eviction, `apexchainx_calculator` exposes deterministic functions to estimate storage size and per-ledger rent costs.

## Public API Functions

### 1. `get_storage_footprint_estimate(env: Env) -> Result<u64, SLAError>`

Returns the estimated total byte footprint of the contract's instance storage layout.

#### Calculation Formula

$$\text{Footprint (bytes)} = B_{\text{base}} + (N_{\text{history}} \times B_{\text{history}}) + (N_{\text{custom}} \times B_{\text{custom}})$$

Where:
- $B_{\text{base}} = 1,024\text{ bytes}$ — Base instance storage overhead (Admin, Operator, Pause state, Config, Stats).
- $N_{\text{history}}$ — Number of retained history entries (`Vec<SLAResult>`).
- $B_{\text{history}} = 120\text{ bytes}$ — Estimated byte size per `SLAResult` tuple (including XDR/Soroban metadata).
- $N_{\text{custom}}$ — Number of registered custom severities (`Map<Symbol, SLAConfig>`).
- $B_{\text{custom}} = 150\text{ bytes}$ — Estimated byte size per custom severity entry.

### 2. `get_rent_estimate(env: Env) -> Result<i128, SLAError>`

Returns the estimated storage rent cost in stroops per ledger based on current byte footprint.

#### Calculation Formula

$$\text{Rent Estimate (stroops/ledger)} = \left\lfloor \frac{\text{Footprint}}{10} \right\rfloor + 1$$

## Admin Guidance & History Pruning

When `get_storage_footprint_estimate` approaches budget limits or when rent estimates rise:
1. Admins should call `prune_history` or `prune_history_by_age` to trim old `SLAResult` entries.
2. Verify footprint reduction by re-querying `get_storage_footprint_estimate()`.
