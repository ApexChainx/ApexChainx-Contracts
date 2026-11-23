# SLAError Failure Taxonomy

> **Audience:** Backend integration engineers, operator tooling developers,
> and contract auditors.
>
> **Purpose:** Provide a formal failure taxonomy for every `SLAError` code
> that can surface to backend consumers. Each error is classified by
> category, severity, consumer impact, and recovery strategy.
>
> Backends can retrieve the machine-readable catalogue at runtime via
> `get_failure_schema()`, which returns a versioned `FailureSchema` struct
> mapping each code to a label and description. This document provides the
> **human-readable** companion with taxonomy, root causes, and runbook steps.

---

## Taxonomy Categories

| Category | Code Range | Description |
|----------|------------|-------------|
| **Initialization** | 1–2 | Contract lifecycle errors before normal operation |
| **Authorization** | 3 | Caller role/permission errors |
| **Configuration** | 4, 8–11, 16, 18 | Severity config, parameter bounds, and freeze state |
| **Migration** | 5 | Storage version mismatch |
| **Pause** | 6 | Contract paused state |
| **Governance** | 7 | Transfer/handoff lifecycle |
| **Validation** | 12, 17 | Input parameter validation |
| **Duplicate Detection** | 13 | Idempotency and duplicate outage prevention |
| **Arithmetic** | 14–15 | Computation overflow or invalid results |
| **Resource Limits** | 19 | Anti-spam and storage capacity limits |

---

## Error Code Reference

### Initialization Errors

#### Code 1: `AlreadyInitialized`

| Field | Value |
|-------|-------|
| **Category** | Initialization |
| **Severity** | Error |
| **Recovery** | None — the contract is already initialized. No action needed. |
| **Consumer Impact** | The caller attempted to initialize an already-initialized contract. This is a programming error in the caller. |
| **Emitted By** | `initialize()` |
| **Recovery Strategy** | Do not call `initialize()` a second time. Check if the contract is initialized by calling `get_admin()` first. |
| **Runbook** | N/A — this should never reach production monitoring. Fix the caller. |

#### Code 2: `NotInitialized`

| Field | Value |
|-------|-------|
| **Category** | Initialization |
| **Severity** | Critical |
| **Recovery** | Admin must call `initialize(admin, operator)` |
| **Consumer Impact** | No contract operations are possible until the contract is initialized. |
| **Emitted By** | `initialize()`, and any function that reads storage before initialization |
| **Recovery Strategy** | Deploy the contract and call `initialize()` with valid admin and operator addresses. |
| **Runbook** | 1. Verify contract is deployed on the expected network. 2. Have the admin address call `initialize(admin, operator)`. 3. Call `get_admin()` to confirm initialization succeeded. |

---

### Authorization Errors

#### Code 3: `Unauthorized`

| Field | Value |
|-------|-------|
| **Category** | Authorization |
| **Severity** | Error |
| **Recovery** | Caller must use the correct wallet/account with the required role |
| **Consumer Impact** | The transaction is rejected; no state change occurs. |
| **Emitted By** | Any admin-gated or operator-gated function |
| **Recovery Strategy** | Verify the caller address matches the stored admin or operator address. Use `get_admin()` and `get_operator()` to retrieve the expected addresses. |
| **Runbook** | 1. Check which role is required for the attempted operation. 2. Call `get_admin()` or `get_operator()` to get the expected address. 3. Ensure the correct wallet is signing the transaction. |

---

### Configuration Errors

#### Code 4: `ConfigNotFound`

| Field | Value |
|-------|-------|
| **Category** | Configuration |
| **Severity** | Error |
| **Recovery** | Admin must configure the severity level |
| **Consumer Impact** | SLA calculation for the requested severity cannot proceed. |
| **Emitted By** | `calculate_sla`, `get_config`, `load_config` |
| **Recovery Strategy** | Call `set_config(severity, threshold, penalty, reward)` to configure the missing severity, then retry. |
| **Runbook** | 1. Identify which severity is missing. 2. Call `set_config()` with valid parameters. 3. Retry the operation. |

#### Code 8: `InvalidThreshold`

| Field | Value |
|-------|-------|
| **Category** | Configuration |
| **Severity** | Error |
| **Recovery** | Provide a threshold within the valid range for the given severity |
| **Consumer Impact** | Configuration update is rejected. |
| **Emitted By** | `set_config`, `set_custom_severity` |
| **Recovery Strategy** | Check severity-specific threshold limits in the contract documentation. Common ranges: critical (1–60), high (1–120), medium (1–240), low (1–480). |
| **Runbook** | 1. Check the severity-specific threshold bounds. 2. Provide a threshold within the valid range. 3. Retry. |

#### Code 9: `InvalidPenalty`

| Field | Value |
|-------|-------|
| **Category** | Configuration |
| **Severity** | Error |
| **Recovery** | Provide a penalty within the valid range and severity ordering constraints |
| **Consumer Impact** | Configuration update is rejected. |
| **Emitted By** | `set_config`, `set_custom_severity` |
| **Recovery Strategy** | Ensure penalty respects cross-severity ordering (critical ≥ high ≥ medium) and severity-specific caps. |
| **Runbook** | 1. Read current config for all severities using `get_config_snapshot()`. 2. Ensure new penalty respects the ordering constraint. 3. Retry. |

#### Code 10: `InvalidReward`

| Field | Value |
|-------|-------|
| **Category** | Configuration |
| **Severity** | Error |
| **Recovery** | Provide a reward base that is materially greater than the penalty rate |
| **Consumer Impact** | Configuration update is rejected. |
| **Emitted By** | `set_config`, `set_custom_severity` |
| **Recovery Strategy** | Ensure `reward_base > penalty_per_minute * 1.5` (reward must materially exceed penalty). |
| **Runbook** | 1. Review the penalty and reward relationship. 2. Provide a reward base that satisfies the consistency constraint. 3. Retry. |

#### Code 11: `InvalidSeverity`

| Field | Value |
|-------|-------|
| **Category** | Configuration |
| **Severity** | Error |
| **Recovery** | Use a supported severity symbol |
| **Consumer Impact** | Operation is rejected. |
| **Emitted By** | `set_config`, `calculate_sla`, `set_custom_severity` |
| **Recovery Strategy** | Use one of the canonical severities (`critical`, `high`, `medium`, `low`) or a previously registered custom severity. |
| **Runbook** | 1. Check supported severities with `get_config_snapshot()`. 2. Use a valid severity symbol. 3. Retry. |

#### Code 16: `ConfigFrozen`

| Field | Value |
|-------|-------|
| **Category** | Configuration |
| **Severity** | Warning |
| **Recovery** | Admin must unfreeze the configuration |
| **Consumer Impact** | Configuration changes are blocked. SLA calculations and read operations continue to work. |
| **Emitted By** | `set_config`, `set_custom_severity`, `remove_custom_severity` |
| **Recovery Strategy** | Admin calls `unfreeze_config()` to re-enable configuration changes. |
| **Runbook** | 1. Confirm the configuration is frozen by calling `is_config_frozen()`. 2. Admin calls `unfreeze_config()`. 3. Retry the configuration change. |

#### Code 18: `SeverityNotInSet`

| Field | Value |
|-------|-------|
| **Category** | Configuration |
| **Severity** | Error |
| **Recovery** | Register the custom severity first |
| **Consumer Impact** | Custom severity operation is rejected. |
| **Emitted By** | `remove_custom_severity`, `get_custom_severity` |
| **Recovery Strategy** | Call `set_custom_severity()` to register the severity before attempting to remove or read it. |
| **Runbook** | 1. Verify the severity is registered using `get_custom_severity()`. 2. If not registered, call `set_custom_severity()`. 3. Retry. |

---

### Migration Errors

#### Code 5: `VersionMismatch`

| Field | Value |
|-------|-------|
| **Category** | Migration |
| **Severity** | Critical |
| **Recovery** | Admin must call `migrate()` OR backend must be upgraded |
| **Consumer Impact** | All versioned operations are blocked until migration is resolved. |
| **Emitted By** | `check_version()` (internal), `migrate()` |
| **Recovery Strategy** | 1. Call `get_migration_state()` to determine direction. 2. If `stored < expected` → admin calls `migrate()`. 3. If `stored > expected` → backend binary must be upgraded. |
| **Runbook** | See [Migration State Consumption Guide](MIGRATION_STATE_CONSUMPTION.md) for the full startup handshake protocol. |

---

### Pause Errors

#### Code 6: `ContractPaused`

| Field | Value |
|-------|-------|
| **Category** | Pause |
| **Severity** | Warning |
| **Recovery** | Admin must call `unpause()` |
| **Consumer Impact** | State-changing operations (SLA calculation, config changes) are blocked. Read-only operations continue to work. |
| **Emitted By** | All state-changing functions |
| **Recovery Strategy** | Admin calls `unpause()` to resume normal operations. Read `get_pause_info()` to understand why the contract was paused. |
| **Runbook** | 1. Call `get_pause_info()` to get the pause reason and timestamp. 2. Investigate why the contract was paused. 3. If safe, admin calls `unpause()`. 4. Verify with `is_paused()`. |

---

### Governance Errors

#### Code 7: `NoPendingTransfer`

| Field | Value |
|-------|-------|
| **Category** | Governance |
| **Severity** | Error |
| **Recovery** | Initiate a transfer proposal first |
| **Consumer Impact** | The accept/cancel transfer operation is rejected. |
| **Emitted By** | `accept_admin`, `cancel_admin_proposal`, `accept_operator`, `cancel_operator_proposal` |
| **Recovery Strategy** | The admin must call `propose_admin()` or `propose_operator()` first to create a pending transfer. |
| **Runbook** | 1. Call `get_pending_admin()` or `get_pending_operator()` to check current state. 2. If none exists, admin must initiate a proposal first. 3. Retry after proposal. |

---

### Validation Errors

#### Code 12: `RetentionLimitOutOfRange`

| Field | Value |
|-------|-------|
| **Category** | Validation |
| **Severity** | Error |
| **Recovery** | Provide a retention limit between 1 and `MAX_HISTORY_SIZE` (1000) |
| **Consumer Impact** | Retention limit update is rejected. |
| **Emitted By** | `set_retention_limit` |
| **Recovery Strategy** | Provide a value between 1 and 1000 (inclusive). |
| **Runbook** | 1. Check the current retention limit with `get_retention_limit()`. 2. Provide a value in the valid range. 3. Retry. |

#### Code 17: `InvalidInput`

| Field | Value |
|-------|-------|
| **Category** | Validation |
| **Severity** | Error |
| **Recovery** | Provide valid input parameters within documented constraints |
| **Consumer Impact** | The operation is rejected. |
| **Emitted By** | `pause` (reason too long) |
| **Recovery Strategy** | Check the documented constraints for the input parameter. For `pause()`, ensure the reason string does not exceed `MAX_REASON_LEN` (256 bytes). |
| **Runbook** | 1. Review the input parameters against documented constraints. 2. Adjust and retry. |

---

### Duplicate Detection Errors

#### Code 13: `DuplicateOutageInput`

| Field | Value |
|-------|-------|
| **Category** | Duplicate Detection |
| **Severity** | Error |
| **Recovery** | Correct the input or wait for a config change |
| **Consumer Impact** | The SLA calculation is rejected because the same `outage_id` was previously submitted with different inputs under the same configuration. |
| **Emitted By** | `calculate_sla` |
| **Event** | `dup_input` — emitted with the stored `SLAResult` before the error is returned |
| **Recovery Strategy** | 1. Check whether the submitted `mttr_minutes` or severity level was entered incorrectly. 2. If the previous calculation was incorrect, admin must call `prune_history()` to remove the conflicting entry. 3. Alternatively, wait for a config update (changes the version hash and allows a fresh entry). 4. If the intent is genuinely a re-evaluation with different data under the same config, use a new unique `outage_id`. |
| **Runbook** | 1. Read the `dup_input` event from the rejected transaction to obtain the stored `SLAResult` (no follow-up `get_latest_by_outage` call is required). 2. If the previous entry is erroneous, admin calls `prune_history()`. 3. Re-submit with corrected data. |

---

### Arithmetic Errors

#### Code 14: `InvalidPenaltyAmount`

| Field | Value |
|-------|-------|
| **Category** | Arithmetic |
| **Severity** | Error |
| **Recovery** | This indicates a computational error; check input values |
| **Consumer Impact** | The SLA calculation is rejected. |
| **Emitted By** | `calculate_sla` (internal computation) |
| **Recovery Strategy** | Verify the input MTTR and threshold values. This error indicates the computed penalty overflowed or produced an unexpected result. Reduce the overtime minutes or contact the admin to adjust config parameters. |
| **Runbook** | 1. Review the input values (mttr_minutes, severity, threshold). 2. If inputs are correct, check severity config with `get_config()`. 3. Retry with corrected values. |

#### Code 15: `InvalidRewardAmount`

| Field | Value |
|-------|-------|
| **Category** | Arithmetic |
| **Severity** | Error |
| **Recovery** | This indicates a computational error; check input values |
| **Consumer Impact** | The SLA calculation is rejected. |
| **Emitted By** | `calculate_sla` (internal computation) |
| **Recovery Strategy** | Verify the input MTTR and threshold values. This error indicates the computed reward produced an unexpected result (e.g., zero or negative when it should be positive). |
| **Runbook** | 1. Review the input values (mttr_minutes, severity, threshold). 2. If inputs are correct, check severity config with `get_config()`. 3. Retry with corrected values. |

---

### Resource Limits Errors

#### Code 19: `OutageRecalcLimit`

| Field | Value |
|-------|-------|
| **Category** | Resource Limits |
| **Severity** | Error |
| **Recovery** | Wait for pruning or use a different outage |
| **Consumer Impact** | The SLA calculation for this `outage_id` is rejected because it already occupies `MAX_RECALCS_PER_OUTAGE` (16) retained history entries. |
| **Emitted By** | `calculate_sla` |
| **Recovery Strategy** | 1. Use a new unique `outage_id` for the same real-world outage (config changes between submissions count as fresh entries). 2. Ask admin to prune old entries via `prune_history()` or `prune_history_by_age()` to free capacity for this outage. |
| **Runbook** | 1. Call `get_history_by_outage(outage_id)` to see the current entries. 2. If legitimate, use a new outage ID. 3. If historical entries are stale, admin calls `prune_history()`. |

---

### Exposure Errors

#### Code 22: `ExposureOverflow`

| Field | Value |
|-------|-------|
| **Category** | Arithmetic (read-only view) |
| **Severity** | Error |
| **Recovery** | Adjust severity configs so aggregate totals fit in i128 |
| **Consumer Impact** | The `get_economic_exposure` view cannot return aggregate totals because they overflow `i128`. Per-severity breakdown entries computed before the overflow are not returned. |
| **Emitted By** | `get_economic_exposure` (read-only, no event emitted) |
| **Recovery Strategy** | 1. Review per-severity `reward_base` and `penalty_per_minute` values via `get_config_snapshot()`. 2. Reduce configs so that the sums of `reward_base * 2` (4 severities) and `penalty_per_minute` (4 severities) each fit within `i128`. With current bounds (`reward_base ≤ 100 000`, `penalty_per_minute ≤ 10 000`), maximum totals are 800 000 and 40 000 respectively, so this error is unreachable under normal operation. |
| **Runbook** | 1. Call `get_economic_exposure()` — if it returns `ExposureOverflow`, call `get_config_snapshot()` to inspect per-severity configs. 2. Identify which aggregate is overflowing by checking the magnitudes. 3. Admin adjusts configs via `set_config()` to reduce the totals. 4. Re-call `get_economic_exposure()`. |

---

## Consumer Guidance: Handling Errors at the Backend

### At Startup

```python
# Load the failure schema once at startup
failure_schema = contract.get_failure_schema()
failure_map = {code.code: code for code in failure_schema.codes}
logger.info(f"Loaded {len(failure_map)} failure codes, schema version: {failure_schema.version}")
```

### In Request Handlers

```python
def handle_calculate_sla(outage_id, severity, mttr):
    try:
        result = contract.calculate_sla(outage_id, severity, mttr)
        return result
    except ContractError as e:
        code = extract_error_code(e)
        if code == 6:  # ContractPaused
            pause_info = contract.get_pause_info()
            logger.warning(f"Contract paused: {pause_info.reason}")
            return retry_after_unpause()
        elif code == 13:  # DuplicateOutageInput
            # The rejection transaction emitted a dup_input event carrying the
            # stored result, so no follow-up contract read is required.
            existing = extract_dup_input_event(transaction.events, outage_id)
            logger.warning(f"Duplicate outage {outage_id}: {existing}")
            return existing  # Return the previously stored result
        elif code == 19:  # OutageRecalcLimit
            logger.error(f"Outage {outage_id} exceeded recalc limit")
            raise
        else:
            logger.error(f"Unexpected error code {code}: {failure_map[code].description}")
            raise
```

### Error Recovery Decision Matrix

| Error Code | Retry Safe? | Idempotent? | Backend Action |
|------------|-------------|-------------|----------------|
| 1 AlreadyInitialized | No | N/A | Fix caller logic |
| 2 NotInitialized | No | N/A | Initialize contract |
| 3 Unauthorized | No | N/A | Switch wallet |
| 4 ConfigNotFound | No | N/A | Configure severity |
| 5 VersionMismatch | After migration | N/A | Run handshake |
| 6 ContractPaused | After unpause | Eventual | Wait for unpause |
| 7 NoPendingTransfer | No | N/A | Initiate proposal |
| 8–11 Config | No | N/A | Fix config params |
| 12 RetentionLimit | No | N/A | Fix limit value |
| 13 DuplicateOutageInput | After prune | N/A | Resolve conflict |
| 14–15 Arithmetic | With corrected input | N/A | Fix inputs |
| 16 ConfigFrozen | After unfreeze | Eventual | Wait for unfreeze |
| 17 InvalidInput | With corrected input | N/A | Fix input |
| 18 SeverityNotInSet | After registration | N/A | Register severity |
| 19 OutageRecalcLimit | After prune | N/A | Prune or new ID |
| 20 ProposalExpired | No | N/A | Re-initiate proposal |
| 21 AdminRenounced | No | N/A | Use operator path |
| 22 ExposureOverflow | Adjust configs | N/A | Reduce config magnitudes |

---

## Taxonomy Versioning

The failure taxonomy is versioned via the `FailureSchema.version` field
(currently `"v1"`). When new error codes are added:

1. The new code is appended to the `SLAError` enum in `lib.rs`
2. A new entry is added to the `get_failure_schema()` function
3. This document is updated with the new taxonomy entry
4. The schema version remains `"v1"` — adding codes is additive and not
   breaking. Breaking changes (removing or renumbering codes) would require
   a version bump to `"v2"`.

---

## Related Documents

- [Compatibility Tracking Matrix](COMPATIBILITY_TRACKING_MATRIX.md) — Event-schema and storage-version tracking
- [Contract Maintenance Policy](CONTRACT_MAINTENANCE_POLICY.md) — SC-500–SC-508 policies
- [Migration State Consumption Guide](MIGRATION_STATE_CONSUMPTION.md) — `get_migration_state()` consumption
- [Event Topic Compatibility](EVENT_TOPIC_COMPATIBILITY.md) — Event schema versioning
