# Storage-Version Upgrade Playbook

> **Issue:** [#247](https://github.com/ApexChainx/ApexChainx-Contracts/issues/247)
> **Status:** Active
> **Applies to:** `apexchainx_calculator` contract crate

This document is the operator playbook for executing a storage-version upgrade
on the `apexchainx_calculator` Soroban contract. It covers preflight checks,
migration execution, post-upgrade verification, and rollback expectations for
any upgrade that changes the on-chain storage schema.

---

## Table of Contents

- [1. When to Use This Playbook](#1-when-to-use-this-playbook)
- [2. Roles](#2-roles)
- [3. Preflight Checklist](#3-preflight-checklist)
- [4. Execution: Running the Migration](#4-execution-running-the-migration)
- [5. Post-Upgrade Verification](#5-post-upgrade-verification)
- [6. Rollback](#6-rollback)
- [7. Code References](#7-code-references)
- [8. Upgrade History Log](#8-upgrade-history-log)

---

## 1. When to Use This Playbook

Follow this playbook whenever:

- The `STORAGE_VERSION` constant in [`apexchainx_calculator/src/lib.rs`](../apexchainx_calculator/src/lib.rs) has been **incremented** in
  a new contract binary.
- A new migration arm has been added to the `migrate()` function.
- A new storage key has been introduced that existing deployments do not yet
  have in their on-chain state.
- The `RESULT_SCHEMA_VERSION` has changed (breaking change to `SLAResult`).

**You do not need this playbook for:**

- Additive changes that do **not** require a storage schema migration (e.g.,
  adding a new read-only view function).
- Configuration value changes via `set_config` — those are regular admin
  operations, not storage upgrades.

---

## 2. Roles

| Role | Responsibility |
|------|---------------|
| **Admin** | Calls `migrate()`, pauses/unpauses the contract, verifies migration |
| **Backend Operator** | Verifies version handshake via `get_version_info()`, monitors events |
| **Release Engineer** | Deploys the new WASM binary, coordinates upgrade window |

All migration operations require the **admin** key. The operator role cannot
execute migration.

---

## 3. Preflight Checklist

Complete these steps **before** deploying the new contract binary.

### 3.1 Confirm the New Binary Is Correct

```bash
# Verify the WASM checksum matches the release manifest
sha256sum -c <(curl -sL https://github.com/ApexChainx/ApexChainx-Contracts/releases/latest/download/manifest.sha256)

# Confirm the binary reports the expected storage version
# The contract's get_version_info() will report expected_version = STORAGE_VERSION
```

### 3.2 Take a Pre-Upgrade Snapshot

Record the current on-chain state before any migration:

```bash
# Call these read-only views from the backend or Soroban CLI:
#   get_config_snapshot()    — capture all severity configs
#   get_version_info()       — record current storage_version
#   get_stats()              — snapshot cumulative statistics
#   get_full_audit_state()   — complete audit snapshot
```

Save these values. They serve as the **rollback reference** if the migration
must be reverted.

### 3.3 Pause the Contract

```bash
# Admin calls:
pause(reason="Storage upgrade to v{N}. Pre-upgrade snapshot captured.")
```

This blocks `calculate_sla` and all state-mutating operations while the
migration is in progress. Read-only views (`get_version_info`, `get_config`,
`healthcheck`, etc.) remain available.

Verify the pause:

```bash
is_paused()   # must return true
get_pause_info()  # confirm reason and timestamp
```

### 3.4 Verify Backend Readiness

The backend should call `get_version_info()` against the **new** contract
binary and confirm:

- `storage_version` is the **old** version (pre-migration, e.g., `1`)
- `result_schema_version` is unchanged or matches the new schema version
- `needs_migration` is `true`
- `is_paused` is `true`
- `contract_name` is `"sla_calc"`

If `needs_migration` is `false`, the storage version is already current —
skip to verification.

### 3.5 Confirm No In-Flight Operations

Ensure no `calculate_sla` calls are in progress. The pause in step 3.3
prevents new calls; wait for any in-flight transactions to finalise.

---

## 4. Execution: Running the Migration

### 4.1 Call `migrate()`

The admin calls the `migrate` entrypoint:

```rust
// Contract entrypoint signature:
pub fn migrate(env: Env, caller: Address) -> Result<(), SLAError>
```

**What `migrate()` does:**

1. Reads the current `stored_version` from on-chain storage (key: `VER`)
2. If `stored_version == STORAGE_VERSION`, returns `Ok(())` (idempotent no-op)
3. If `stored_version > STORAGE_VERSION`, returns `VersionMismatch` (binary is
   older than state — this is a rollback attempt; see §6)
4. Applies each migration step sequentially (`v0→v1`, `v1→v2`, …)
5. Each step reads old state, writes new state, then bumps the version
6. After all steps, verifies `current == STORAGE_VERSION`
7. Emits a `migrate_done` event with `(old_version, new_version)`

**Current migration path (as of v1):**

| Step | From | To | Action |
|------|------|----|--------|
| v0→v1 | `0` | `1` | Calls `init_missing_storage_defaults()` to populate any missing keys (PAUSED, STATS, history, config, custom config) and stamps `STORAGE_VERSION = 1` |

Future upgrades will add new arms (e.g., `v1→v2`) as commented placeholders
in `lib.rs`.

### 4.2 Expected Errors

| Error | Meaning | Action |
|-------|---------|--------|
| `NotInitialized` | Contract was never deployed | Verify the contract address is correct |
| `Unauthorized` | Caller is not the admin | Ensure the admin key is used |
| `VersionMismatch` | `stored > STORAGE_VERSION` | See §6 Rollback |

### 4.3 Idempotency

Calling `migrate()` when the storage version is already current is safe:
it returns `Ok(())` immediately without mutating state. You can re-invoke it
after a successful migration without side effects.

---

## 5. Post-Upgrade Verification

### 5.1 Confirm Migration State

```bash
# Call get_migration_state() (returns StorageVersionInfo):
# {
#   "stored_version": <new_version>,
#   "expected_version": <new_version>,
#   "needs_migration": false
# }
```

Or use the combined version handshake:

```bash
# Call get_version_info() (returns VersionInfo):
# {
#   "storage_version": <new_version>,
#   "result_schema_version": <unchanged_or_new>,
#   "needs_migration": false,
#   "is_paused": true,       # still paused from preflight
#   "contract_name": "sla_calc"
# }
```

`needs_migration` must be `false`.

### 5.2 Verify Backend Handshake

The backend should re-call `get_version_info()` and confirm:

- `storage_version` matches the new expected version
- `result_schema_version` is compatible with the backend's parser
- `needs_migration == false`

### 5.3 Verify Configuration Integrity

```bash
# Confirm the pre-upgrade config snapshot matches the post-upgrade snapshot:
get_config_snapshot()
get_config_version_hash()
get_full_audit_state()
```

The `config_version_hash` should be identical to the pre-upgrade value
(migration does not modify configuration).

### 5.4 Smoke Test a Read-Only Calculation

```bash
# Call calculate_sla_view() with known inputs and confirm the result:
calculate_sla_view(outage_id="smoke-test", severity="critical", mttr_minutes=10)
```

This verifies the calculation path works with the new storage schema.

### 5.5 Unpause the Contract

```bash
unpause()
```

Verify:

```bash
is_paused()   # must return false
```

### 5.6 Submit a Canary Calculation

Submit a single `calculate_sla` with a unique `outage_id` (e.g.,
`"upgrade-canary-<timestamp>"`) to confirm the full mutating path works
end-to-end:

```bash
calculate_sla(outage_id="upgrade-canary-<ts>", severity="low", mttr_minutes=5)
```

Check the emitted `sla_calc` event and verify the result is appended to
history (`get_history()`).

### 5.7 Monitor Events

Watch for the `migrate_done` event emitted during migration, and confirm no
unexpected `stats_sat` or error events follow during the canary calculation.

---

## 6. Rollback

### 6.1 When Rollback Is Needed

Rollback is required if:

- `migrate()` returns `VersionMismatch` because the stored version is newer
  than the binary
- Post-upgrade verification reveals data corruption or unexpected state
- The new binary has a critical bug discovered during smoke testing

### 6.2 Rollback Procedure

1. **Keep the contract paused.** The pause is the circuit breaker — do not
   unpause.

2. **Re-deploy the previous WASM binary** (the version matching the
   pre-upgrade `STORAGE_VERSION`). Use the published release manifest to
   verify the correct binary:
   ```bash
   sha256sum -c manifest.sha256
   ```

3. **Call `get_migration_state()`** to confirm the old binary sees
   `needs_migration == false` (the stored version matches the old binary's
   expected version). If `needs_migration` is still `true`, the wrong binary
   was deployed.

4. **Verify state integrity** against the pre-upgrade snapshot captured in
   §3.2:
   ```bash
   get_config_snapshot()
   get_stats()
   get_full_audit_state()
   ```

5. **Unpause** and resume normal operations.

### 6.3 Rollback Invariant

> **Storage versions are monotonic.** The `STORAGE_VERSION` constant in code
> must never be lower than the version stamped in on-chain storage. The
> migration harness rejects `stored > STORAGE_VERSION` with
> `VersionMismatch`. This invariant is enforced by
> [`ts/upgradeGuardTests.ts`](../ts/upgradeGuardTests.ts) (see
> `assertRollbackInvariant`).

**If a rollback is needed**, the deployed binary must be the **exact same
binary** that was running before the upgrade attempt. Downgrading to an
arbitrary older binary is not supported unless its `STORAGE_VERSION` matches
the currently stored version exactly.

### 6.4 Data Written During Migration

The current migration path (v0→v1) is additive only — it initialises missing
keys to deterministic defaults. It does **not** delete or overwrite existing
data. If the migration introduced new storage keys, those keys will remain
populated after rollback but are harmless (the old binary does not read them).

Future migration arms that transform or delete existing data must document the
reversibility of each step in the upgrade history log (§8).

---

## 7. Code References

| Concept | Location | Description |
|---------|----------|-------------|
| `STORAGE_VERSION` constant | [`apexchainx_calculator/src/lib.rs`](../apexchainx_calculator/src/lib.rs) | The version this binary expects (currently `1`) |
| `STORAGE_VERSION_KEY` | `lib.rs` | On-chain storage key `VER` |
| `migrate()` entrypoint | `lib.rs` | Admin-gated migration harness with sequential step application |
| `init_missing_storage_defaults()` | `lib.rs` | Idempotent initialisation of missing keys for v0→v1 migration |
| `StorageVersionInfo` struct | `lib.rs` | Return type of `get_migration_state()` |
| `VersionInfo` struct | `lib.rs` | Combined version handshake for backend startup |
| `HealthcheckResult` struct | `lib.rs` | Read-only readiness probe for load balancers |
| `check_version()` guard | `lib.rs` | Enforced on all mutating and most read-only entrypoints |
| `read_storage_version()` | [`apexchainx_calculator/src/storage_version.rs`](../apexchainx_calculator/src/storage_version.rs) | Reads on-chain version, returns `None` if uninitialised |
| `is_migration_complete()` | `storage_version.rs` | Checks the `MIGRATED` flag |
| `build_negotiation_info()` | [`apexchainx_calculator/src/version_negotiation.rs`](../apexchainx_calculator/src/version_negotiation.rs) | Builds `VersionNegotiationInfo` for multi-contract compatibility checks |
| `negotiate_contract_versions()` | `version_negotiation.rs` | Cross-contract version compatibility negotiation |
| `classifyVersion()` | [`ts/upgradeGuardTests.ts`](../ts/upgradeGuardTests.ts) | TypeScript reference implementation of version classification |
| `assertRollbackInvariant()` | `upgradeGuardTests.ts` | Enforces monotonic version progression |
| Cross-Contract Deployment Checklist | [`docs/CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md`](CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md) | Pre/post-deployment compatibility verification |
| Contract API Compatibility | [`docs/CONTRACT_API_COMPATIBILITY.md`](CONTRACT_API_COMPATIBILITY.md) | Stable API signature specification for backend consumers |

---

## 8. Upgrade History Log

| Date | From | To | Binary Change | Migration Steps | Reversible? |
|------|------|----|---------------|-----------------|-------------|
| Initial | — | v1 | Initial deployment (`initialize`) | N/A (fresh deploy) | N/A |
| (template) | v1 | v2 | (placeholder) | (to be documented) | (to be assessed) |

**Instructions for future upgrades:**

When adding a new migration arm in `lib.rs`, also:

1. Increment `STORAGE_VERSION`
2. Add a migration arm `if current == <old> { ... }` in `migrate()`
3. Document the step in this table
4. Update `classifyVersion()` in `ts/upgradeGuardTests.ts` if the version
   classification logic changed
5. Run the upgrade guard tests:
   ```bash
   npx ts-node ts/upgradeGuardTests.ts
   ```
6. Update `docs/CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md` if the migration
   affects cross-contract interfaces
