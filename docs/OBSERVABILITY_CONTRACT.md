# Observability Contract for Backend Event Consumers & Indexers

> **Audience:** Backend operators (apexchainx-be), SRE, incident responders, and
> any team that monitors the `apexchainx_calculator` contract in production.
>
> **Purpose:** Define the canonical observability signals that a backend
> indexer should derive from contract events so that operations teams can
> distinguish normal operation from degradation, misconfiguration, or outage.
>
> **Related documents:**
> - [`AUDIT_TRAIL.md`](AUDIT_TRAIL.md) — full event reference with payload schemas and replay guidance
> - [`EVENT_TOPIC_COMPATIBILITY.md`](EVENT_TOPIC_COMPATIBILITY.md) — formal topic stability and deprecation policy
> - [`EVENT_COMPATIBILITY_POLICY.md`](EVENT_COMPATIBILITY_POLICY.md) — payload field ordering and compatibility rules

<p align="center">
  <img src="https://img.shields.io/badge/audience-operations-red" alt="Audience: Operations" />
  <img src="https://img.shields.io/badge/contract-apexchainx__calculator-blue" alt="Contract: apexchainx_calculator" />
  <img src="https://img.shields.io/badge/event_version-v1-success" alt="Event version: v1" />
</p>

---

## Table of Contents

- [1. Health Signal Model](#1-health-signal-model)
- [2. Canonical Event → Health Signal Mapping](#2-canonical-event--health-signal-mapping)
  - [2.1 SLA Calculation Health (`sla_calc`, `set_int`)](#21-sla-calculation-health-sla_calc-set_int)
  - [2.2 Configuration Health (`cfg_upd`)](#22-configuration-health-cfg_upd)
  - [2.3 Lifecycle Health (`paused`, `unpause`)](#23-lifecycle-health-paused-unpause)
  - [2.4 Governance Health (`adm_prop`, `adm_acc`, `adm_can`, `adm_ren`, `op_prop`, `op_acc`, `op_can`, `op_set`)](#24-governance-health-adm_prop-adm_acc-adm_can-adm_ren-op_prop-op_acc-op_can-op_set)
  - [2.5 Configuration-Fence Health (`cfg_frz`, `cfg_unfrz`)](#25-configuration-fence-health-cfg_frz-cfg_unfrz)
  - [2.6 Storage Maintenance Health (`pruned`, `pruned_a`)](#26-storage-maintenance-health-pruned-pruned_a)
  - [2.7 Saturation Signal (`stats_sat`)](#27-saturation-signal-stats_sat)
  - [2.8 Migration Boundary (`migrate_done`)](#28-migration-boundary-migrate_done)
- [3. Operational Alerting Rules](#3-operational-alerting-rules)
- [4. Startup Health Bootstrap](#4-startup-health-bootstrap)
- [5. Source of Truth](#5-source-of-truth)

---

## 1. Health Signal Model

Every contract event doubles as a **health signal** for the system. An
indexer should classify each observed event into one of four operational
states:

| State | Meaning | Example |
|-------|---------|---------|
| **Healthy** | Normal operational flow | Regular `sla_calc` / `set_int` pairs arriving at expected cadence |
| **Degraded** | Operating but with reduced fidelity | `stats_sat` fired — on-chain counter no longer reflects true totals |
| **Paused** | Intentional admin-initiated halt | `paused` observed, no `unpause` yet — all mutating calls blocked |
| **Anomaly** | Unexpected pattern requiring investigation | `sla_calc` stops arriving for > N minutes with no `paused` event |

An indexer that produces these four states for every monitored dimension
(sla-throughput, config-churn, governance-activity, counter-fidelity,
storage-health) satisfies this observability contract.

---

## 2. Canonical Event → Health Signal Mapping

### 2.1 SLA Calculation Health (`sla_calc`, `set_int`)

| Signal | Derivation |
|--------|-----------|
| **`sla_throughput_healthy`** | At least one `sla_calc` / `set_int` pair observed within the configured lookback window (default: 15 minutes). Absence with no `paused` event = `sla_throughput_degraded`. |
| **`sla_pair_ratio`** | Count of `sla_calc` events ÷ count of `set_int` events over the window. Must equal 1.0. Any deviation = `sla_pair_anomaly` (missed event in the pair). |
| **`sla_outage_id_replay_gap`** | Dedupe on `(outage_id, config_version_hash, recorded_at)` from `set_int`. A retried `calculate_sla` with unchanged config emits **no events** — the gap is expected, not anomalous. |
| **`sla_amount_drift`** | Compare `sla_calc.amount` against `set_int.amount`. Must match exactly. Mismatch = `sla_amount_anomaly`. |

**Healthy**: `sla_throughput_healthy == true`, `sla_pair_ratio == 1.0`, `sla_amount_drift == 0`.

**Degraded**: `stats_sat` has fired for `totcalc` → on-chain count under-reports.

**Anomaly**: `sla_pair_ratio ≠ 1.0` or throughput gap with no `paused`.

### 2.2 Configuration Health (`cfg_upd`)

| Signal | Derivation |
|--------|-----------|
| **`config_churn_rate`** | Count of `cfg_upd` events per hour. Elevated churn (e.g. > 10/hour) = `config_churn_elevated`. |
| **`config_severity_order`** | After each `cfg_upd`, validate `critical.penalty ≥ high.penalty ≥ medium.penalty ≥ low.penalty` from the next `get_config_snapshot()` call. Inversion = `config_order_anomaly` (contract-level bug). |
| **`config_cache_staleness`** | Timestamp of most recent `cfg_upd` vs. last `get_config_snapshot()` fetch. Staleness > 5 minutes after a `cfg_upd` = `config_cache_stale`. |

**Healthy**: `config_churn_rate` within normal band, `config_severity_order` valid, cache fresh.

**Anomaly**: `config_order_anomaly` or `config_cache_stale` after an observed `cfg_upd`.

### 2.3 Lifecycle Health (`paused`, `unpause`)

| Signal | Derivation |
|--------|-----------|
| **`pause_state`** | Most recent of `paused` or `unpause`. `paused` without matching `unpause` = contract is **paused**. |
| **`pause_duration`** | Elapsed time since most recent `paused` without `unpause`. Duration > configured threshold (e.g. 30 minutes) = `pause_prolonged`. |

**Healthy**: `pause_state == false`.

**Paused**: `pause_state == true`. Expected mute on `sla_calc`, `set_int`, `cfg_upd`.

**Anomaly**: `sla_calc` observed while `pause_state == true` (contract bug or ledger-replay error).

### 2.4 Governance Health (`adm_prop`, `adm_acc`, `adm_can`, `adm_ren`, `op_prop`, `op_acc`, `op_can`, `op_set`)

| Signal | Derivation |
|--------|-----------|
| **`governance_pending`** | `adm_prop` without matching `adm_acc`/`adm_can`, or `op_prop` without matching `op_acc`/`op_can`. |
| **`admin_renounced`** | `adm_ren` observed and no subsequent `adm_acc`. **Permanent** — no admin-gated function will ever succeed again. |
| **`governance_cycle_stuck`** | A pending proposal (`governance_pending == true`) that remains unresolved for > configured threshold (e.g. 60 minutes). |

**Healthy**: No pending proposals, `admin_renounced == false`.

**Degraded**: `governance_pending == true` but within threshold — normal two-step in progress.

**Anomaly**: `governance_cycle_stuck` or `admin_renounced == true` (terminal, requires redeploy).

### 2.5 Configuration-Fence Health (`cfg_frz`, `cfg_unfrz`)

| Signal | Derivation |
|--------|-----------|
| **`config_frozen`** | Most recent of `cfg_frz` or `cfg_unfrz`. `cfg_frz` without matching `cfg_unfrz` = config is **frozen**. |
| **`cfg_upd_during_freeze`** | Any `cfg_upd` observed while `config_frozen == true` = `freeze_violation` (contract bug). |

**Healthy**: `config_frozen == false`.

**Paused**: `config_frozen == true`. All `set_config` calls will panic with `ConfigFrozen`.

### 2.6 Storage Maintenance Health (`pruned`, `pruned_a`)

| Signal | Derivation |
|--------|-----------|
| **`prune_rate`** | Count of `pruned` + `pruned_a` events per day. Elevated rate = admin is actively compacting history. |
| **`post_prune_retention`** | After each prune event, `kept_count` from the payload. Dropping below a configured floor (e.g. 10 entries) = `retention_low`. |

**Healthy**: `prune_rate` within normal band, `retention_low == false`.

**Degraded**: `retention_low` — history window is shrinking.

### 2.7 Saturation Signal (`stats_sat`)

| Signal | Derivation |
|--------|-----------|
| **`counter_saturated`** | Set `true` for the affected counter (`totcalc`, `totviol`, `totrew`, `totpen`) on first `stats_sat` observation. **Never resets** — permanent degradation for that counter. |
| **`cumulative_saturation_drift`** | Running sum of `(attempted_increment - previous_value)` from every `stats_sat` payload across the contract lifetime. |

**Healthy**: `counter_saturated == false` for all four counters.

**Degraded**: `counter_saturated == true` for any counter. On-chain total is a **lower bound** — true cumulative is only available from the off-chain aggregator.

**Anomaly**: Repeated `stats_sat` for the same counter after it already saturated (indicates the counter is still being incremented at the ceiling).

### 2.8 Migration Boundary (`migrate_done`)

| Signal | Derivation |
|--------|-----------|
| **`migration_in_progress`** | `migrate_done` observed within the last checkpoint window. Treat as a **hard upgrade boundary**. |
| **`post_migration_staleness`** | `migrate_done` observed but startup bootstrap (re-read schemas, re-pin symbols) has not been re-executed. |

**Healthy**: No recent `migrate_done`, or bootstrap has been re-executed.

**Anomaly**: `post_migration_staleness` — indexer is consuming events against stale schema assumptions.

---

## 3. Operational Alerting Rules

Backend operators should configure alerts on the following conditions,
derived exclusively from the health signals above:

| Priority | Alert | Condition | Action |
|----------|-------|-----------|--------|
| **P0 — Critical** | `admin_renounced == true` | Permanent loss of admin capability | Escalate to contract owner; prepare redeploy |
| **P0 — Critical** | `freeze_violation` | `cfg_upd` observed during config freeze | Escalate to contract maintainers — contract-level bug |
| **P1 — High** | `sla_throughput_degraded` with `pause_state == false` | No `sla_calc` for > 15 min, not paused | Investigate operator activity; check ledger health |
| **P1 — High** | `sla_pair_anomaly` | `sla_calc` / `set_int` ratio ≠ 1.0 | Check for partial ledger replay or missed events |
| **P1 — High** | `counter_saturated` fires for first time | On-chain counter fidelity lost for affected field | Mark dashboards; begin off-chain accumulation |
| **P2 — Medium** | `pause_prolonged` | Contract paused for > 30 minutes | Confirm admin intent; notify stakeholders |
| **P2 — Medium** | `governance_cycle_stuck` | Pending proposal unresolved > 60 minutes | Contact admin; verify two-step ceremony |
| **P2 — Medium** | `config_order_anomaly` | Severity penalty inversion detected | Escalate to contract maintainers |
| **P3 — Low** | `config_churn_elevated` | Unusually high `cfg_upd` rate | Review admin activity; rule out automation loop |
| **P3 — Low** | `retention_low` | History retention below configured floor | Review prune policy; consider raising retention cap |

---

## 4. Startup Health Bootstrap

An indexer initializing or recovering should execute this sequence to
establish baseline health state:

1. **Pin symbol vocabulary** — call `get_result_schema()`, `get_failure_schema()`,
   and `get_config_snapshot()`. Cache all symbol strings.
2. **Resolve pause state** — call `get_pause_info()`. If paused, initialize
   `pause_state = true`.
3. **Resolve freeze state** — call `get_config_snapshot()` and inspect the
   freeze flag. Initialize `config_frozen` accordingly.
4. **Resolve governance state** — call `get_pending_admin()` and
   `get_pending_operator()`. If `Some(_)`, initialize `governance_pending = true`.
5. **Resolve counter saturation** — call `get_stats()`. Compare each counter
   against its type ceiling. If any counter is at ceiling, initialize
   `counter_saturated = true` for that counter — the event may have been
   missed before the indexer started.
6. **Begin event streaming** — consume events from the last processed ledger.
   Apply the health signal derivations in Section 2 for each observed event.

---

## 5. Source of Truth

| Resource | File | Role |
|----------|------|------|
| Canonical event catalogue (rustdoc) | [`apexchainx_calculator/src/event_schema.rs`](../apexchainx_calculator/src/event_schema.rs) | Event names, payload schemas, versioning rules |
| Correlation ID logic | [`apexchainx_calculator/src/event_correlation.rs`](../apexchainx_calculator/src/event_correlation.rs) | Deterministic trace-ID generation (SC-W5-079) |
| Event ordering invariants | [`apexchainx_calculator/src/event_ordering_tests.rs`](../apexchainx_calculator/src/event_ordering_tests.rs) | SC-W5-042 ordering guarantees |
| Topic stability contract | [`apexchainx_calculator/src/topic_stability_tests.rs`](../apexchainx_calculator/src/topic_stability_tests.rs) | SC-W5-043 topic invariants |
| Full event audit trail | [`docs/AUDIT_TRAIL.md`](AUDIT_TRAIL.md) | Per-event payload reference, replay playbook |
| Project architecture | [`docs/PROJECT_CONTEXT.md`](PROJECT_CONTEXT.md) | System flow, future roadmap |

If this document disagrees with the source files, **the source files are correct**.
