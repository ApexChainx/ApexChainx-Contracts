# Configuration Validation Rules

> **Reference:** Validation rules enforced by the `set_config` function in the
> `apexchainx_calculator` contract, designed to prevent admin-side misuse and
> ensure runtime safety.

## Table of Contents

- [Overview](#overview)
- [Supported Severities](#supported-severities)
- [Validation Rules](#validation-rules)
- [Error Handling](#error-handling)
- [Default Configuration Values](#default-configuration-values)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Implementation Notes](#implementation-notes)

---

## Overview

The `apexchainx_calculator` contract validates all configuration updates to
prevent admin-side misuse and unexpected runtime behavior. Invalid configuration
writes fail deterministically with specific error codes, ensuring that:

1. No partial state changes occur — validation runs before any storage writes
2. Error codes are specific — each validation failure maps to a unique error
3. Behavior is deterministic — same inputs always produce same outcome

## Overview

The SLA Calculator contract validates all configuration updates to prevent admin-side misuse and unexpected runtime behavior. Invalid configuration writes will fail deterministically with specific error codes.

## Supported Severities

The contract supports exactly four severity levels, each with distinct
validation parameters:

| Severity | Priority | Typical Response Window | Default Threshold |
|----------|----------|------------------------|------------------|
| `critical` | 🔴 Highest | < 15 minutes | 15 min |
| `high` | 🟠 Important | < 30 minutes | 30 min |
| `medium` | 🟡 Standard | < 60 minutes | 60 min |
| `low` | 🟢 Low priority | < 120 minutes | 120 min |

## Validation Rules

### General Rules (Apply to All Severities)

| Parameter | Valid Range | Purpose | Error on Violation |
|-----------|-------------|---------|-------------------|
| `threshold_minutes` | 1 – 1,440 (24 hours) | Prevents zero or unrealistic thresholds | `InvalidThreshold` (code 8) |
| `penalty_per_minute` | 1 – 10,000 | Ensures penalties are positive and bounded | `InvalidPenalty` (code 9) |
| `reward_base` | 1 – 100,000 | Ensures rewards are positive and bounded | `InvalidReward` (code 10) |

### Severity-Specific Rules

| Severity | Max Threshold | Min Penalty/Min | Rationale |
|----------|--------------|-----------------|-----------|
| `critical` | 60 minutes | 50 units | Short response window, significant penalty for failures |
| `high` | 120 minutes | 25 units | Moderate response window with meaningful penalties |
| `medium` | 240 minutes (4h) | 10 units | Longer response window, moderate penalty floor |
| `low` | 1,440 minutes (24h) | Max 100 units | Lowest priority, penalties capped to prevent over-punishment |

### Cross-Parameter Consistency Rules

| Rule | Condition | Error on Violation | Rationale |
|------|-----------|-------------------|-----------|
| Reward exceeds penalty | `penalty_per_minute × 1.5 < reward_base` | `InvalidReward` (code 10) | Ensures meeting SLA targets is always financially beneficial compared to paying penalties for minor threshold overruns |

### Cross-Severity Consistency Rules

| Rule | Condition | Error on Violation | Rationale |
|------|-----------|-------------------|-----------|
| Penalty severity ordering | `critical.penalty >= high.penalty >= medium.penalty >= low.penalty` | `InvalidPenalty` (code 9) | Maintains logical severity progression — a higher-severity outage must never carry a lower penalty than a lower-severity one |

### Rule Enforcement Order

1. **General parameter bounds** are validated first (range checks)
2. **Severity-specific constraints** are validated second (severity-dependent limits)
3. **Cross-parameter consistency** is validated third (penalty × 1.5 < reward for same severity)
4. **Cross-severity consistency** is validated last (higher severity penalties ≥ lower severity penalties)

## Error Handling

### Error Reference

| Error Code | Name | Trigger Condition | Recovery |
|------------|------|-------------------|----------|
| 8 | `InvalidThreshold` | Threshold outside valid range or severity-specific limit | Adjust to valid range (1–1440, severity-dependent) |
| 9 | `InvalidPenalty` | Penalty per minute outside valid range or violates cross-severity ordering | Adjust to valid range (1–10,000, severity-dependent) or align with adjacent severity levels |
| 10 | `InvalidReward` | Reward base outside valid range or violates cross-parameter consistency | Adjust to valid range (1–100,000) or ensure penalty × 1.5 < reward |
| 11 | `InvalidSeverity` | Severity not in supported set | Use one of: critical, high, medium, low |

### Deterministic Failure Guarantees

| Property | Guarantee |
|----------|-----------|
| Reproducibility | Same invalid parameters always produce the same error |
| State safety | No partial state changes — validation occurs before any storage writes |
| Error specificity | Each error code maps to exactly one validation condition |
| Gas efficiency | Failed validations do not consume gas beyond the validation check |

### Error Flow

```
Input Parameters
       ↓
[General Range Validation]  ←── Errors 8, 9, 10
       ↓
[Severity-Specific Validation]  ←── Error 8, 9
       ↓
[Severity Existence Check]  ←── Error 11
       ↓
[Cross-Parameter Consistency]  ←── Error 10
       ↓
[Cross-Severity Consistency]  ←── Error 9
       ↓
[Event Emission on Success]  ←── Config saved
```

## Default Configuration Values

The contract initializes with the following validated defaults:

| Severity | Threshold (min) | Penalty/Min (units) | Reward Base (units) | Annual Impact Estimate |
|----------|----------------|---------------------|--------------------|----------------------|
| `critical` | 15 | 100 | 750 | ~$270,000 |
| `high` | 30 | 50 | 750 | ~$135,000 |
| `medium` | 60 | 25 | 750 | ~$67,500 |
| `low` | 120 | 10 | 600 | ~$27,000 |

> **Note:** Annual impact assumes consistent incident rates and is for
> illustration only. Actual impact depends on incident frequency and duration.

## Best Practices for Backend Operators

### 1. Gradual Configuration Changes

```
❌ Bad:  threshold_minutes: 30 → 5 (drastic jump)
✅ Good: threshold_minutes: 30 → 25 → 20 → 15 (incremental)
```

- Make incremental changes rather than drastic jumps
- Test new configurations in a staging environment first
- Use `calculate_sla_view` to preview the impact of changes

### 2. Severity Consistency

| Rule | Rationale |
|------|-----------|
| Maintain logical severity progression | Higher severity → lower threshold, higher penalty |
| Avoid inversion | Critical should always be stricter than high |
| Proportional scaling | Penalty ratios should reflect severity tiers |

### 3. Economic Considerations

- Consider the total economic impact of penalties and rewards
- Ensure penalty structures incentivize the desired behavior
- Balance rewards against operational costs
- Audit economic impact quarterly based on incident data

### 4. Monitoring

- Monitor SLA calculation results after configuration changes
- Watch for unexpected patterns in violation rates
- Track reward-to-penalty ratios over time
- Set up alerts for anomalous configuration changes

### 5. Pre-Commit Validation

Use `calculate_sla_view` to test configurations before applying:

```rust
// Preview the effect of a new threshold
let result = calculate_sla_view(
    outage_id,
    severity::critical,
    mttr_minutes,  // Try different MTTR values
);
// Verify edge cases (threshold boundaries) work as expected
```

### 6. Change Management Checklist

- [ ] Test new config with `calculate_sla_view`
- [ ] Verify severity progression is maintained
- [ ] Check economic impact is within expected bounds
- [ ] Deploy during low-traffic period
- [ ] Monitor violation rates for 24h post-change

## Examples

### Valid Configurations

```rust
// Critical: aggressive response with high penalty
set_config(admin, critical, 30, 150, 1000);

// High: balanced response with moderate penalty
set_config(admin, high, 45, 75, 800);

// Medium: standard response with reasonable penalty
set_config(admin, medium, 90, 30, 600);

// Low: relaxed response with minimal penalty
set_config(admin, low, 180, 15, 500);
```

### Invalid Configurations

```rust
// ERROR: threshold too high for critical (max 60)
set_config(admin, critical, 120, 100, 750);  // → InvalidThreshold

// ERROR: penalty too low for high (min 25)
set_config(admin, high, 30, 10, 750);         // → InvalidPenalty

// ERROR: negative reward not allowed
set_config(admin, medium, 60, 25, -100);      // → InvalidReward

// ERROR: unsupported severity level
set_config(admin, urgent, 15, 100, 750);      // → InvalidSeverity

// ERROR: reward too low relative to penalty (need penalty × 1.5 < reward)
set_config(admin, critical, 15, 100, 100);    // → InvalidReward (100×1.5=150 ≮ 100)

// ERROR: cross-severity penalty inversion (high penalty < medium penalty)
// First ensure medium has a lower penalty, then try setting high below it
set_config(admin, medium, 60, 50, 750);       // medium penalty = 50
set_config(admin, high, 30, 25, 750);         // → InvalidPenalty (high 25 < medium 50)
```

## Implementation Notes

| Property | Detail |
|----------|--------|
| Validation timing | Occurs before any state changes — no partial updates |
| Enforcement level | All rules enforced at the contract level |
| Success events | Successful config updates emit versioned `cfg_upd` events |
| Failure behavior | Failed validations do not emit events or consume extra gas |
| Determinism | Same invalid inputs always produce same error codes |

## See Also

- **[Severity Compatibility Matrix](./severity-compatibility-matrix.md)** — Canonical vs. custom severity families, additive vs. breaking change classification, release planning checklist.
- **[Event Ordering Guarantees](./event-ordering-guarantees.md)** — Deterministic event sequencing contract for backend consumers.
- **[CONTRACT_API_COMPATIBILITY.md](./CONTRACT_API_COMPATIBILITY.md)** — API surface stability and deprecation policy.
---

## Custom Severity Configuration (`set_custom_severity`)

> **Reference:** The `set_custom_severity`, `get_custom_severity`, and
> `remove_custom_severity` methods in `apexchainx_calculator/src/config.rs` and
> `apexchainx_calculator/src/lib.rs`.

### Overview

Custom severity support allows the admin to register arbitrary severity levels
beyond the four canonical ones (`critical`, `high`, `medium`, `low`). This is
useful for workloads that need additional grading tiers without altering the
canonical configuration.

### Validation Differences from `set_config`

| Aspect | `set_config` | `set_custom_severity` |
|--------|-------------|----------------------|
| Severity check | Must be canonical (critical/high/medium/low) | Must NOT be canonical |
| Parameter bounds | General + per-severity + cross-parameter + cross-severity | General bounds only (`validate_general_bounds`) |
| Per-severity limits | `critical` max 60 min, `low` penalty cap 100, etc. | None — only the universal ranges apply |
| Cross-severity ordering | Enforces penalty progression (critical ≥ high ≥ medium) | Not enforced |
| Metadata recording | Config version hash, `get_last_config_update` updated | Not tracked in canonical version hash or metadata |

### General Bounds Applied

| Parameter | Valid Range |
|-----------|-------------|
| `threshold_minutes` | 1 – 1,440 (24 hours) |
| `penalty_per_minute` | 1 – 10,000 |
| `reward_base` | 1 – 100,000 |
| Cross-parameter consistency | `penalty_per_minute × 1.5 < reward_base` |

### Budget Estimate

`set_custom_severity` performs a single storage read-modify-write on the
`CUSTOM_CONFIG_KEY` map and emits one event. Based on regression coverage
(`test_set_custom_severity_budget_is_reasonable` in `tests.rs`):

| Metric | Bound |
|--------|-------|
| CPU instructions | < 150,000 |
| Storage entries | Grows by 1 per unique severity (never shrinks unless `remove_custom_severity` is called) |

### Storage and Compatibility Notes

- Custom severity entries are stored in a separate `CUSTOM_CONFIG_KEY` map,
  independent of the canonical `CONFIG_KEY` map.
- They do **not** appear in `get_config_snapshot`, `get_config_version_hash`, or
  `get_last_config_update` — those APIs reflect canonical severities only.
- To inspect registered custom severities, use `get_custom_config_snapshot`.
- Removing a severity with `remove_custom_severity` clears its entry from the
  map and emits a `cfg_upd` event with zeroed payload.
- Repeated calls to `set_custom_severity` for the same severity overwrite the
  previous values (last-write-wins).
- `calculate_sla` and `calculate_sla_view` resolve custom severities through
  `load_config`, which falls back to the custom map when the severity is not
  canonical.

### When to Use

- Additional grading tiers beyond the four canonical levels are needed.
- The admin is aware that custom entries are not covered by the config version
  hash or canonical snapshot.
- Storage growth is acceptable — each distinct custom severity occupies one
  entry in the contract's instance storage.

### Public Methods Reference

| Method | Signature | Description |
|--------|-----------|-------------|
| `set_custom_severity` | `(env, caller, severity, threshold_minutes, penalty_per_minute, reward_base)` | Register or update a custom severity |
| `get_custom_severity` | `(env, severity)` | Read a registered custom severity |
| `remove_custom_severity` | `(env, caller, severity)` | Unregister a custom severity |
| `get_custom_config_snapshot` | `(env)` | List all registered custom severities |
| Zero-threshold safety | `threshold_minutes = 0` is rejected with `InvalidThreshold` (code 8) before reaching storage. This prevents a division in `compute_result`'s performance-ratio calculation from ever operating on a zero denominator |
| Cross-severity hardening | `validate_cross_severity_penalty_ordering` uses safe `.ok_or(InvalidSeverity)` lookups on the canonical severity list rather than panicking `.unwrap()` calls, so any unexpected invariant violation surfaces as a deterministic error instead of an unrecoverable host trap |