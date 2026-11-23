# Staking Contract

The `apexchainx_staking` crate implements a token-staking contract on the
Stellar network using the [Soroban](https://soroban.stellar.org/) smart-contract
platform.

## Overview

Stakers record token amounts in the contract and accrue an estimated reward
based on the number of ledger sequences elapsed since the stake was last
modified. An admin address controls the reward rate and contract lifecycle.

## Public API

| Method | Auth required | Description |
|--------|---------------|-------------|
| `initialize(admin, reward_rate)` | `admin` | One-time setup — sets admin, rate, and zero total. |
| `stake(staker, amount)` | `staker` | Add a positive amount to the balance and reset its `staked_at` ledger. |
| `unstake(staker, amount)` | `staker` | Remove a positive amount up to the current balance; a full withdrawal removes the record. |
| `get_stake(staker)` | none | Return amount, last-modified ledger, and estimated pending reward; errors if no record exists. |
| `total_staked()` | none | Return the aggregate recorded stake, or zero before initialization. |
| `reward_rate()` | none | Return the current reward rate, or zero before initialization. |
| `get_admin()` | none | Return the configured admin; panics before initialization. |
| `set_reward_rate(new_rate)` | `admin` | Set a non-negative reward rate. |

The Rust API documentation on each public contract method describes its
arguments, authorization, return value, errors, and emitted events.

## Gas Optimization Strategy (issue #806)

| Technique | Where applied |
|-----------|--------------|
| Instance storage for hot-path globals | `ADMIN`, `RATE`, `TOTAL` |
| Single persistent read per staker | `stake`, `unstake`, `get_stake` |
| Storage key removed on zero balance | `unstake` full withdrawal |
| `symbol_short!` for all keys | All storage reads/writes |
| No redundant writes | Rate/total only written when changed |

## Reward Calculation

```
pending_reward = amount × reward_rate × (current_ledger − staked_at)
```

The reward estimate is returned by `get_stake` and is **not** written to chain —
it is computed on-the-fly from the on-chain inputs, keeping the staker record
compact.

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 1 | `Unauthorized` | Caller is not the admin. |
| 2 | `AlreadyInitialized` | Contract already initialized. |
| 3 | `InvalidAmount` | Amount must be positive. |
| 4 | `NoStake` | Staker has no active position. |
| 5 | `InsufficientStake` | Unstake amount exceeds balance. |
| 6 | `InvalidRate` | Rate must be non-negative. |

## Running Tests

```bash
cargo test -p apexchainx_staking
```

## Event Emissions (issue #807)

Every state-changing operation emits a Soroban event so that off-chain
indexers can reconstruct the full staking history without polling storage.

| Operation | Topic\[0\] | Topic\[1\] | Data payload |
|-----------|-----------|-----------|--------------|
| `initialize` | `"staking"` | `"init"` | `admin: Address` |
| `stake` | `"staking"` | `"staked"` | `StakeEventData { staker, amount, new_total }` |
| `unstake` | `"staking"` | `"unstaked"` | `StakeEventData { staker, amount, new_total }` |
| `set_reward_rate` | `"staking"` | `"rate_set"` | `new_rate: i128` |

Events are **not** emitted on error paths — a failed call produces no events.
