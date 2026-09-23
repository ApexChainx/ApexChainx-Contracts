# Severity Curves

> **Issue:** [#610](https://github.com/ApexChainx/ApexChainx-Contracts/issues/610)
> **Status:** Active
> **Applies to:** `set_config` / custom-severity validation in `apexchainx_calculator`

Single, authoritative description of the admissible configuration region per
severity. The numbers below are the enforcing constants in
`SLACalculator::validate_config`, `validate_general_bounds`, and the two
cross-severity ordering validators — not aspirational values. Boundary fixtures
(in `severity_curve_tests.rs`) pin these edges so the region cannot drift
silently.

---

## 1. General bounds (all severities, incl. custom)

| Parameter | Min | Max |
| --- | --- | --- |
| `threshold_minutes` | 1 | 1440 |
| `penalty_per_minute` | 1 | 10000 |
| `reward_base` | 1 | 100000 |

Cross-parameter rule: a bonus must materially exceed the penalty —

```
penalty_per_minute * 3 < reward_base * 2      (reward_base > 1.5 * penalty_per_minute)
```

## 2. Per-severity region (canonical severities)

| Severity | `threshold_minutes` | `penalty_per_minute` |
| --- | --- | --- |
| critical | ≤ 60 | ≥ 50 |
| high | ≤ 120 | ≥ 25 |
| medium | ≤ 240 | ≥ 10 |
| low | ≤ 1440 (general cap) | ≤ 100 |

## 3. Cross-severity ordering

- **Thresholds:** `critical ≤ high ≤ medium ≤ low` (adjacent pairing enforced).
- **Penalties:** `critical ≥ high ≥ medium` enforced on adjacent pairs.
  The **low** severity is *exempt* from the upper-direction penalty check: its
  per-severity cap (`100`) may legitimately exceed medium's floor (`≥ 10`) by
  design.

## 4. Boundary fixtures

`src/severity_curve_tests.rs` encodes the boundary as `±1` pairs at every edge:

- every general-bounds min/max (`1/0`, `1440/1441`, `10000/10001`, `100000/100001`);
- each per-severity cap (`critical` 60, `high` 120, `medium` 240) and floor
  (`critical` 50, `high` 25, `medium` 10, `low` 100) accepts `value` and rejects `value ∓ 1`;
- the reward ratio boundary is pinned on both sides;
- cross-severity ordering accepts in-order and rejects inverted configurations.

Any future change to the acceptable region must update this file and the docs in
the same commit.