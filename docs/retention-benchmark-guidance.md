# MAX_HISTORY_SIZE & Retention-Polish Benchmark Guidance

> **Status:** Maintainership recommendation note  
> **References:** Issue #251, SC-062, SC-063, SC-013  
> **Last updated:** 2026-07-29  
> **Audience:** Contract admins, backend operators, deployment engineers

## Table of Contents

- [Overview](#overview)
- [Current Defaults](#current-defaults)
- [Why Retention Tuning Matters](#why-retention-tuning-matters)
- [Benchmark Evidence](#benchmark-evidence)
- [Tuning Strategy](#tuning-strategy)
- [Deployment-Scale Recommendations](#deployment-scale-recommendations)
- [Public Configuration Knobs](#public-configuration-knobs)
- [Monitoring & Alerting](#monitoring--alerting)
- [FAQ](#faq)

---

## Overview

The SLA Calculator contract retains a bounded on-chain history of every
`calculate_sla` result. This history is used for:

- **Backend reconciliation** — replaying and verifying SLA outcomes
- **Audit trails** — proving what was paid and why
- **Duplicate detection** — preventing double-submission of the same outage
- **Anti-spam accounting** — capping per-outage retained entries at `MAX_RECALCS_PER_OUTAGE` (16)

Retention comes at a cost: each entry incurs storage footprint and read/write
overhead during pruning. Poor defaults can create **storage or cost drift**
that is difficult to unwind after deployment.

---

## Current Defaults

| Constant | Value | Defined In | Notes |
|---|---|---|---|
| `MAX_HISTORY_SIZE` | **1000** | `apexchainx_calculator/src/lib.rs` | Hard upper bound; cannot be exceeded |
| `MAX_RECALCS_PER_OUTAGE` | **16** | `apexchainx_calculator/src/lib.rs` | Per-outage anti-spam cap |
| `RETENTION_LIMIT_KEY` | configurable | `apexchainx_calculator/src/history.rs` | Admin override via `set_retention_limit()` |
| Default retention limit | `MAX_HISTORY_SIZE` (1000) | `apexchainx_calculator/src/history.rs` | When `RETENTION_LIMIT_KEY` is unset |

The retention limit is surfaced as a public read via `get_retention_limit()`
and can be lowered (never raised above `MAX_HISTORY_SIZE`) by the admin
via `set_retention_limit()`.

---

## Why Retention Tuning Matters

### Storage Cost Drift

On Soroban, **instance storage** is the primary cost driver for history. Each
`SLAResult` entry contains:

- `outage_id` (Symbol — up to ~32 bytes)
- `status`, `payment_type`, `rating` (Symbol × 3)
- `mttr_minutes`, `threshold_minutes` (u32 × 2)
- `amount` (i128)
- `config_version_hash` (u64)
- `recorded_at` (u64)

A full history of 1000 entries consumes a material amount of instance storage.
For high-throughput deployments generating thousands of SLA calculations per
day, the default 1000-entry window fills quickly and must be pruned
aggressively.

### Read Cost on Pruning

Both `prune_history()` and `prune_history_by_age()` iterate over the
**entire** history (`O(n)`). At 1000 entries, a prune operation consumes
measurable CPU budget. The worst-case scenario — a full history plus a
retention limit close to `MAX_HISTORY_SIZE` — is the most expensive.

### Audit Window Trade-off

A larger history means a **longer auditable lookback window**. Backend
consumers that rely on `get_history_page()` or `get_history_by_outage()` for
reconciliation benefit from deeper history. The trade-off is **storage cost**
vs **audit completeness**.

---

## Benchmark Evidence

### Gas Profile — `calculate_sla` + History Growth

Based on the existing stress test (`test_stress_1000_calculations_mixed_severities`
in `apexchainx_calculator/src/tests.rs`):

| Metric | Value |
|---|---|
| Calls in benchmark | 1000 |
| Per-call CPU budget ceiling | < 50,000,000 instructions |
| History growth per call | +1 entry (append) |
| Full-history read cost (get_history) | O(n) — linear in entry count |

The per-call cost remains stable even as history grows because `calculate_sla`
performs a **single linear scan** of the history for duplicate detection
(anti-spam accounting), not a full rebuild. The primary cost is the scan,
which is `O(n)`.

### Gas Profile — Pruning Operations

From `test_prune_history_budget_is_reasonable` and
`test_prune_history_by_age_budget_is_reasonable`:

| Operation | History Size | CPU Budget Ceiling |
|---|---|---|
| `prune_history` | 20 entries → keep 5 | < 900,000 instructions |
| `prune_history_by_age` | 20 entries → prune by 500s age | < 900,000 instructions |

Linear extrapolation: at `MAX_HISTORY_SIZE` (1000 entries), a full prune could
consume up to ~45× the 20-entry budget, approaching ~40M instructions. This
is still within Soroban limits but should be **scheduled during low-traffic
periods**.

### Retention-Size Trade-offs (Simulated)

| Retention Limit | Lookback Window (est.) | Storage Footprint | Prune Cost (O) | Use Case |
|---|---|---|---|---|
| **100** (minimal) | ~1 hour at 100 calls/min | Very low | Low | High-throughput, cost-sensitive |
| **250** | ~2-3 hours at 100 calls/min | Low | Moderate | Balanced production default |
| **500** | ~5-8 hours at 100 calls/min | Moderate | Moderate | Standard audit window |
| **1000** (default) | ~16 hours at 100 calls/min | High | High | Full audit trail, low-throughput |

> **Note:** The lookback window depends on call frequency. A deployment with
> 10 calls/hour will retain days of history at any limit. A deployment with
> 1000 calls/hour will cycle through the window in under an hour at the
> default 1000-entry limit.

---

## Tuning Strategy

### Step 1 — Measure Your Call Rate

Use the existing `get_stats()` and `get_severity_telemetry()` endpoints to
determine your **average calls per hour**:

```
calls_per_hour = (weekly_calculations / 7) / 24
```

### Step 2 — Determine Your Required Audit Window

Ask: **How far back must auditors/reconcilers be able to look?**

- **Regulatory requirement?** Multiply required hours by `calls_per_hour`.
- **Operational requirement?** Consider the maximum time between backend
  reconciliation runs.
- **Debugging requirement?** Keep enough history to investigate patterns.

### Step 3 — Set the Retention Limit

```
retention_limit = MIN(audit_window_hours * calls_per_hour, MAX_HISTORY_SIZE)
```

Apply via admin call:

```
set_retention_limit(admin, retention_limit)
```

### Step 4 — Schedule Pruning

For **high-throughput deployments**, schedule `prune_history()` or
`prune_history_by_age()` calls:

- **By count:** `prune_history(admin, desired_keep_count)` — keeps the N
  most recent entries.
- **By age:** `prune_history_by_age(admin, min_age_seconds)` — removes
  entries older than the cutoff.

Recommendations:

| Deployment Scale | Pruning Strategy | Frequency |
|---|---|---|
| Low (< 10 calls/h) | `prune_history_by_age` (24h cutoff) | Daily |
| Medium (10–100 calls/h) | `prune_history_by_age` (6h cutoff) | Every 4 hours |
| High (> 100 calls/h) | `prune_history` (keep latest 250–500) | Hourly |

---

## Deployment-Scale Recommendations

### Small / Development Deployments

- **Retention limit:** 1000 (default) — no tuning needed
- **Pruning:** Optional; call `prune_history_by_age(admin, 86400)` weekly
- **Rationale:** Low volume means history grows slowly; the default is safe

### Medium / Staging Deployments

- **Retention limit:** 500
- **Pruning:** `prune_history_by_age(admin, 21600)` every 6 hours
- **Rationale:** Balances audit visibility with moderate storage costs

### Large / Production Deployments

- **Retention limit:** 250
- **Pruning:** `prune_history(admin, 200)` hourly OR
  `prune_history_by_age(admin, 3600)` hourly
- **Rationale:** Keeps storage costs predictable; backend reconciliation
  should fetch history more frequently

### High-Throughput / Multi-Tenant Deployments

- **Retention limit:** 100–250
- **Pruning:** `prune_history(admin, 100)` every 30 minutes
- **Rationale:** Storage costs dominate; audit trail can be offloaded to
  event indexers (all results are emitted as `sla_calc` events)

---

## Public Configuration Knobs

| Knob | Function | Range | Access |
|---|---|---|---|
| `set_retention_limit(limit)` | Override the default retention cap | 1 – `MAX_HISTORY_SIZE` (1000) | Admin only |
| `get_retention_limit()` | Read the current retention cap | — | Public (read-only) |
| `prune_history(keep_latest)` | Drop all but the N newest entries | 0 – history length | Admin only |
| `prune_history_by_age(min_age_seconds)` | Drop entries older than cutoff | 0 – u64::MAX | Admin only |
| `MAX_HISTORY_SIZE` | **Immutable** upper bound (compile-time constant) | 1000 | Requires contract upgrade |

### Important: `MAX_HISTORY_SIZE` is Immutable

`MAX_HISTORY_SIZE` (1000) is a compile-time constant and **cannot** be changed
by admin action. Raising it requires a **contract upgrade** with a new binary.
The admin can only **lower** the effective limit via `set_retention_limit()`.

---

## Monitoring & Alerting

### Metrics to Track

| Metric | Source | Alert Threshold |
|---|---|---|
| History length | `get_history().len()` | > 80% of retention limit |
| Prune frequency | Event log (`pruned` / `pruned_a` events) | < 1× per retention window |
| Per-call CPU | Budget instrumentation | > 80% of Soroban limit |
| Storage growth | Ledger inspection | Sustained upward trend |

### Recommended Alert Rules

1. **History near capacity:** Alert when `history.len() > 0.8 * retention_limit`
2. **Pruning starvation:** Alert when no prune event emitted in 2× the
   scheduled interval
3. **CPU budget warning:** Alert when per-call CPU exceeds 40M instructions

---

## FAQ

### Q: Can I set `set_retention_limit` to 0?

No. The minimum is 1 (enforced by `RetentionLimitOutOfRange` error).
Setting it to 1 effectively disables history retention while still allowing
duplicate detection on the single retained entry.

### Q: What happens when history exceeds the retention limit?

The `calculate_sla` function trims history **after** appending the new result.
It uses `min(MAX_HISTORY_SIZE, retention_limit)` as the effective cap.

### Q: Is `prune_history` or `prune_history_by_age` better?

- **`prune_history`** — deterministic, keeps exactly N entries. Best for
  predictable storage budgets.
- **`prune_history_by_age`** — time-based, keeps entries within a sliding
  window. Best for audit requirements based on time rather than count.

### Q: Will lowering the retention limit affect existing history?

No. `set_retention_limit` only changes the cap used for **future** trimming
during `calculate_sla`. Existing entries are not removed until the next
calculation or prune call.

### Q: Can I rely on event indexers instead of on-chain history?

Yes. Every `calculate_sla` result is emitted as a versioned `sla_calc` event
with the full result payload. If your backend indexes these events, you can
run with a very low retention limit (e.g., 50–100) and rely on the event
stream for historical queries.

---

## References

- [Configuration Validation Rules](./config-validation.md) — validation bounds
- [Audit Trail Documentation](./AUDIT_TRAIL.md) — audit event layout
- [Result Payload Hashing](./RESULT_PAYLOAD_HASHING.md) — backend parity
- [Event Compatibility Policy](./EVENT_COMPATIBILITY_POLICY.md) — event schema stability
- [Storage & Cost Baselines](./sc-w5-storage-and-cost-baselines.md) — cost model
