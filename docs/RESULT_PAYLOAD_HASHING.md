# Policy: Historical Result Payload Hashing for Backend Parity Checking

> **Version:** 1.0.0  
> **Last Updated:** 2026-07-29  
> **Applies To:** `apexchainx_calculator` Soroban smart contract  
> **References:** Issue #203, SC-W5-046 (#95 replay_calculate_sla), config_version_hash

---

## 1. Purpose

This document defines the **formal policy** for how historical SLA result payloads are hashed and how backend consumers can verify parity between on-chain results and off-chain recomputations. This ensures backend systems can independently verify that the contract's deterministic calculations match their own records.

---

## 2. The Config Version Hash

### 2.1 What It Is

Every `SLAResult` includes a `config_version_hash: u64` field. This hash is a deterministic fingerprint of the **exact severity configuration** used to compute that result.

### 2.2 Hash Algorithm

The contract uses a **polynomial rolling hash**:

```
H = ((s1 * P^3 + s2 * P^2 + s3 * P + s4) mod M) where P = 31, M = 2^64
s1 = critical severity config hash
s2 = high severity config hash  
s3 = medium severity config hash
s4 = low severity config hash
```

Each severity's config hash is:
```
severity_hash = threshold_minutes * P^2 + penalty_per_minute * P + reward_base
```

Where `P = 31` (a small prime chosen to fit within the overflow semantics of `wrapping_mul` for `u64`).

### 2.3 Properties

| Property | Guarantee |
|----------|-----------|
| **Determinism** | Same config → same hash, every time |
| **Collision resistance** | Different config values produce different hashes (verified by tests) |
| **Field-order sensitivity** | Changing threshold vs. penalty vs. reward yields different hashes |
| **Severity isolation** | Changing one severity does not affect others' contribution |
| **Stable across reads** | Repeated reads of unchanged config → identical hash |

---

## 3. The SLAResult Payload

### 3.1 Fields

| Field | Type | Description |
|-------|------|-------------|
| `outage_id` | `Symbol` | Unique outage identifier |
| `status` | `Symbol` | `"met"` or `"viol"` |
| `mttr_minutes` | `u32` | Measured time to repair |
| `threshold_minutes` | `u32` | Applied SLA threshold |
| `amount` | `i128` | Financial outcome (positive = reward, negative = penalty) |
| `payment_type` | `Symbol` | `"rew"` or `"pen"` |
| `rating` | `Symbol` | `"top"`, `"excel"`, `"good"`, or `"poor"` |
| `config_version_hash` | `u64` | Hash of the config used for evaluation |
| `recorded_at` | `u64` | Ledger timestamp at calculation time |

### 3.2 What Is Hashed

The `config_version_hash` captures **only the configuration parameters**:

- `threshold_minutes` for each canonical severity
- `penalty_per_minute` for each canonical severity  
- `reward_base` for each canonical severity

The result fields (`outage_id`, `status`, `mttr_minutes`, `amount`, etc.) are **NOT** included in the config hash. They are the output, not the input.

---

## 4. Backend Parity Verification

### 4.1 Workflow

```
┌──────────────────────────────────────────────────────────────┐
│                    BACKEND PARITY CHECK                       │
├──────────────────────────────────────────────────────────────┤
│ 1. Fetch on-chain result via get_history() or event listener │
│ 2. Extract config_version_hash from result                   │
│ 3. Fetch current config via get_config_snapshot()            │
│ 4. Recompute config_version_hash locally                     │
│ 5. Compare: local_hash == result.config_version_hash?        │
│    ├─ Yes → config was not tampered with                     │
│    └─ No  → config changed between calculation and check     │
│                                                              │
│ 6. Recompute the result via replay_calculate_sla()           │
│    ├─ Same result → calculation is consistent                │
│    └─ Different   → alert! investigate discrepancy           │
└──────────────────────────────────────────────────────────────┘
```

### 4.2 Replay Verification

The contract exposes `replay_calculate_sla(outage_id, severity, mttr_minutes, recorded_at_ledger)` which:

1. Takes the same inputs as the original `calculate_sla` call
2. Uses the **current** config (not historical)
3. Returns `(SLAResult, config_version_hash)`
4. Does NOT mutate state or emit events

Use this for reconciliation when you have the original inputs but want to verify the output:

```python
# Backend parity check
on_chain_result = contract.get_history()[0]
replay_result, replay_hash = contract.replay_calculate_sla(
    on_chain_result.outage_id,
    severity,
    on_chain_result.mttr_minutes,
    on_chain_result.recorded_at
)

assert replay_result.status == on_chain_result.status
assert replay_result.amount == on_chain_result.amount
assert replay_result.rating == on_chain_result.rating
# Note: config_version_hash may differ if config was updated since
```

### 4.3 Off-Chain Hash Recomputation

Backend consumers can independently recompute the config version hash to verify it:

```python
def compute_config_version_hash(config_snapshot):
    """
    Recompute the config version hash from a get_config_snapshot() response.
    Must match the contract's compute_config_version_hash() exactly.
    """
    P = 31
    MOD = 2**64
    
    def severity_hash(threshold, penalty, reward):
        return (threshold * P * P + penalty * P + reward) % MOD
    
    result = 0
    for i, entry in enumerate(config_snapshot.entries):
        sh = severity_hash(
            entry.config.threshold_minutes,
            entry.config.penalty_per_minute,
            entry.config.reward_base
        )
        # Polynomial rolling: severity[i] * P^(n-i-1)
        power = len(config_snapshot.entries) - i - 1
        result = (result + sh * (P ** power)) % MOD
    
    return result
```

---

## 5. Idempotency Guarantees

### 5.1 Duplicate Outage Handling

The contract's idempotency policy for `calculate_sla`:

- **Same inputs, same config** → Returns the original result (idempotent, no state change)
- **Same inputs, different config** → New calculation replaces old entry
- **Same outage_id, different inputs** → Returns `DuplicateOutageInput` error

### 5.2 Backend Expectations

- Backends should **deduplicate by `outage_id`** in their event streams
- When a `DuplicateOutageInput` error is received, the backend should compare the existing result against the attempted submission
- The `config_version_hash` is the key discriminator for determining whether a re-submission is truly a duplicate or a legitimate re-evaluation

---

## 6. Future Enhancements

| Feature | Status | Description |
|---------|--------|-------------|
| Per-ledger config snapshots | Planned | Store config at each `calculate_sla` call for exact historical replay |
| Result payload hash | Planned | Hash the full result (not just config) for tamper-evident history |
| Merkle tree over history | Planned | Efficient proof of inclusion for individual results |

---

## 7. Testing

Run the config hash tests:

```bash
cargo test --package apexchainx_calculator -- config_version_hash
```

Key test cases:
- `test_config_version_hash_is_deterministic` — same config → same hash
- `test_config_version_hash_changes_on_update` — config change → hash change
- `test_config_version_hash_collision_resistance` — additive sum collision resistance
- `test_config_version_hash_field_order_sensitivity` — per-field sensitivity
- `test_config_version_hash_severity_isolation` — per-severity isolation
- `test_backend_replay_exact_threshold_outcome_is_deterministic_before_config_change` — replay parity

---

## 8. Change Log

| Date | Version | Description |
|------|---------|-------------|
| 2026-07-29 | 1.0.0 | Initial formal policy; codifies config_version_hash algorithm, replay workflow, and backend parity verification |
