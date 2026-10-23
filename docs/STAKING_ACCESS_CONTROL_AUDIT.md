# Staking Access Control Audit

> **Issue:** [#812 — Audit Staking access control modifiers](https://github.com/ApexChainx/ApexChainx-Contracts/issues/812)
> **Area:** `Staking` (`apexchainx_staking`)
> **Priority:** P0 - Critical

---

## Overview

This document audits every access-control boundary in the
`apexchainx_staking` contract (`apexchainx_staking/src/lib.rs`).
The contract runs on the Stellar/Soroban platform and does **not** use
OpenZeppelin or any EVM-based `AccessControl` primitives.
All role enforcement is done via Soroban's built-in `Address::require_auth()`.

---

## Role Model

| Role | Who holds it | How it is stored |
|------|-------------|-----------------|
| **Admin** | Single address set at `initialize` | Instance storage (`ADMIN` key) |

There is no operator role in the current staking contract.
All privileged operations are admin-only.

---

## Entrypoint Access Control Matrix

| Method | Auth Required | Caller Check | Notes |
|--------|--------------|-------------|-------|
| `initialize(admin, reward_rate)` | `admin.require_auth()` | Admin address | One-time; fails with `AlreadyInitialized` if called again |
| `stake(staker, amount)` | `staker.require_auth()` | Staker address | Any address may stake |
| `unstake(staker, amount)` | `staker.require_auth()` | Staker address | Only the staker can withdraw their own tokens |
| `get_stake(staker)` | None | — | Read-only; no auth required |
| `total_staked()` | None | — | Read-only; no auth required |
| `reward_rate()` | None | — | Read-only; no auth required |
| `get_admin()` | None | — | Read-only; no auth required |
| `set_reward_rate(new_rate)` | `admin.require_auth()` | Admin address | Admin-only; fails with `Unauthorized` otherwise |

---

## Auth Implementation Review

### `initialize`

```rust
admin.require_auth();
```

- Called once. Stores `admin` in instance storage under `KEY_ADMIN`.
- Guard: `AlreadyInitialized` error if `KEY_ADMIN` already exists — prevents
  an attacker from re-initializing after deployment.
- **Finding:** Correct. No additional guard needed.

### `stake`

```rust
staker.require_auth();
```

- Staker must sign their own stake transaction. Prevents third-party forced
  staking on behalf of another address without their consent.
- **Finding:** Correct.

### `unstake`

```rust
staker.require_auth();
```

- Only the owning address can withdraw their own staked tokens.
- **Finding:** Correct.

### `set_reward_rate`

```rust
let admin: Address = env.storage().instance().get(&KEY_ADMIN)
    .ok_or(StakingError::Unauthorized)?;
admin.require_auth();
```

- Reads the stored admin and requires their authorization.
- Fails with `Unauthorized` if no admin is stored (contract not initialized).
- **Finding:** Correct. The admin address is verified against on-chain storage
  before auth is checked, which prevents spoofing.

---

## Error Code Review

| Code | Name | Description | Auth-Relevant |
|------|------|-------------|---------------|
| 1 | `Unauthorized` | Caller is not the admin | Yes |
| 2 | `AlreadyInitialized` | Contract already initialized | Yes — prevents re-init attack |
| 3 | `InvalidAmount` | Amount must be positive | No |
| 4 | `NoStake` | Staker has no active position | No |
| 5 | `InsufficientStake` | Unstake amount exceeds balance | No |
| 6 | `InvalidRate` | Rate must be non-negative | No |

---

## Security Findings

### ✅ No Critical Vulnerabilities Found

All entrypoints consistently enforce `require_auth()` before any state mutation.
No method allows unauthorized state changes.

### ℹ️ Observations & Recommendations

1. **No admin transfer mechanism** — The current contract has no way to change
   the admin address after initialization. If the admin key is lost, the
   `set_reward_rate` function becomes permanently inaccessible.
   _Recommendation:_ Add a two-step `propose_admin` / `accept_admin` flow
   (similar to `apexchainx_calculator`) in a future issue.

2. **No emergency pause** — The contract has no pause mechanism. If a
   vulnerability is discovered, there is no way to halt staking/unstaking
   without deploying a new contract.
   _Recommendation:_ Add a `pause` / `unpause` mechanism gated by the admin
   role in a future issue.

3. **Single-admin model** — Only one admin address is supported. Multi-sig or
   threshold-based admin would improve operational resilience.
   _Recommendation:_ Evaluate multi-sig admin for production deployments.

4. **Read-only methods are correctly unauthenticated** — `get_stake`,
   `total_staked`, `reward_rate`, and `get_admin` require no auth and do not
   write state. This is correct and matches the established pattern in
   `apexchainx_calculator`.

---

## Test Coverage for Access Control

The following existing tests in `apexchainx_staking/src/lib.rs` provide
access-control coverage:

| Test | What it covers |
|------|----------------|
| `test_initialize_sets_admin_and_rate` | Admin set at init |
| `test_double_initialize_fails` | Re-init guard |
| `test_set_reward_rate_updates_rate` | Admin can set rate |
| `test_invalid_rate_rejected` | Rate validation |
| `test_stake_invalid_amount_rejected` | Amount validation |
| `test_unstake_insufficient_fails` | Unstake guard |

**Gap:** No test verifies that a non-admin caller is rejected by
`set_reward_rate`. A dedicated `test_unauthorized_set_reward_rate_rejected`
test should be added (tracked under issue #812 acceptance criteria).

---

## Related Documentation

- [`docs/AUTH_MODEL.md`](AUTH_MODEL.md) — Auth model for `apexchainx_calculator`
  (reference pattern)
- [`docs/ACCESS_CONTROL_REVIEW.md`](ACCESS_CONTROL_REVIEW.md) — Calculator
  access control review
- [`docs/ACCESS_CONTROL_AUDIT.md`](ACCESS_CONTROL_AUDIT.md) — Prior audit
  (calculator-focused)
- [`apexchainx_staking/README.md`](../apexchainx_staking/README.md) — Staking
  contract overview and API table

---

## Audit Sign-Off

| Item | Status |
|------|--------|
| All state-mutating entrypoints gated by `require_auth` | ✅ |
| No unauthorized state-change path exists | ✅ |
| Re-initialization protected | ✅ |
| Staker self-custody enforced | ✅ |
| Admin transfer mechanism | ⚠️ Missing — recommended for future issue |
| Emergency pause | ⚠️ Missing — recommended for future issue |
| Unauthorized-caller test for `set_reward_rate` | ⚠️ Missing — should be added |
