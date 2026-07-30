# Contract Lifecycle State Diagram

> State-transition reference for the `apexchainx_calculator` contract.  
> Resolves [#256](https://github.com/ApexChainx/ApexChainx-Contracts/issues/256).  
> All transitions are sourced directly from
> `apexchainx_calculator/src/lib.rs`, `governance.rs`, and `config_freeze.rs`.

---

## Table of Contents

1. [Overview](#overview)
2. [Top-level lifecycle](#top-level-lifecycle)
3. [Pause / Unpause flow](#pause--unpause-flow)
4. [Storage migration flow](#storage-migration-flow)
5. [Config-freeze flow](#config-freeze-flow)
6. [Admin transfer (two-step)](#admin-transfer-two-step)
7. [Operator handoff (two-step)](#operator-handoff-two-step)
8. [Combined orthogonal state matrix](#combined-orthogonal-state-matrix)
9. [Invariants and guard table](#invariants-and-guard-table)

---

## Overview

The contract has **four independent boolean axes** that combine to determine
which operations are permitted at any moment:

| Axis | Storage key | Default | Blocks |
|------|-------------|---------|--------|
| Initialized | `ADMIN` present | false | All versioned calls |
| Paused | `PAUSED` | false | `calculate_sla`, state-changing ops |
| Version-matched | `VER == STORAGE_VERSION` | true after init | All versioned calls |
| Config frozen | `FREEZE` | false | `set_config`, `set_custom_severity`, `remove_custom_severity` |

The diagrams below model each axis independently, then the matrix section
shows their interactions.

---

## Top-level lifecycle

```mermaid
stateDiagram-v2
    [*] --> Uninitialized : contract deployed

    Uninitialized --> Active : initialize(admin, operator)\n[stamps VER=1, sets ADMIN, OPERATOR, PAUSED=false]

    Active --> Paused : pause(caller, reason)\n[admin only]
    Paused --> Active : unpause(caller)\n[admin only]

    Active --> NeedsMigration : contract binary upgraded\n(STORAGE_VERSION bumped)
    NeedsMigration --> Active : migrate(admin)\n[applies v0→v1→… steps]

    Active --> ConfigFrozen : freeze_config(admin)
    ConfigFrozen --> Active : unfreeze_config(admin)

    Active --> AdminRenounced : renounce_admin(admin)\n[irreversible — admin key removed]
    Paused --> AdminRenounced : renounce_admin(admin)\n[irreversible]
```

> **Note:** `AdminRenounced` is a terminal state for governance.
> `pause`, `set_config`, and admin-transfer functions are permanently locked once
> `renounce_admin` is called because all require the `ADMIN` key to be present.

---

## Pause / Unpause flow

```mermaid
stateDiagram-v2
    [*] --> Running : initialize() succeeds\n(PAUSED = false)

    Running --> Paused : pause(caller, reason)\n• admin only\n• reason ≤ 256 bytes\n• stores PauseInfo{reason, timestamp, paused_by}\n• emits paused event

    Paused --> Running : unpause(caller)\n• admin only\n• clears PauseInfo\n• emits unpause event

    Running --> Running : calculate_sla ✓\nset_config ✓\nset_operator ✓\n(all state-changing ops allowed)

    Paused --> Paused : calculate_sla ✗ → ContractPaused\nset_config ✗ → ContractPaused\nis_paused() ✓\nget_pause_info() ✓\n(read-only ops still work)
```

---

## Storage migration flow

```mermaid
stateDiagram-v2
    [*] --> VersionCurrent : initialize()\n(writes VER = STORAGE_VERSION = 1)

    VersionCurrent --> VersionMismatch : contract binary upgraded\n(new binary has STORAGE_VERSION = N+1)\n(on-chain VER still = N)

    VersionMismatch --> VersionCurrent : admin calls migrate()\n• applies each step: v0→v1→v1→v2…\n• idempotent when already current\n• emits migrate_done event

    VersionMismatch --> VersionMismatch : any versioned endpoint called\nreturns VersionMismatch error\n(get_version_info / healthcheck bypass this guard)

    VersionCurrent --> VersionCurrent : normal operation\ncheck_version() passes
```

**Key bypass functions** (callable even in `VersionMismatch` state):

| Function | Why it bypasses |
|----------|----------------|
| `get_version_info()` | Backend needs to read version before deciding to call `migrate` |
| `get_migration_state()` | Read-only diagnostic |
| `healthcheck()` | Load-balancer probe — must always respond |
| `migrate()` | The function that fixes the mismatch |

---

## Config-freeze flow

```mermaid
stateDiagram-v2
    [*] --> Thawed : initialize()\n(FREEZE key absent → defaults to false)

    Thawed --> Frozen : freeze_config(admin)\n• admin only\n• sets FREEZE = true\n• emits cfg_frz event

    Frozen --> Thawed : unfreeze_config(admin)\n• admin only\n• sets FREEZE = false\n• emits cfg_unfrz event

    Thawed --> Thawed : set_config ✓\nset_custom_severity ✓\nremove_custom_severity ✓

    Frozen --> Frozen : set_config ✗ → ConfigFrozen\nset_custom_severity ✗ → ConfigFrozen\nremove_custom_severity ✗ → ConfigFrozen\nget_config ✓\nget_config_snapshot ✓\n(reads are always allowed)
```

---

## Admin transfer (two-step)

```mermaid
stateDiagram-v2
    [*] --> AdminSet : initialize()\n(ADMIN = original_admin)

    AdminSet --> PendingTransfer : propose_admin(caller, new_admin)\n• current admin only\n• stores PADMIN = new_admin\n• emits adm_prop event

    PendingTransfer --> AdminSet : accept_admin(new_admin)\n• must be called by proposed new_admin\n• ADMIN ← new_admin\n• clears PADMIN\n• emits adm_acc event

    PendingTransfer --> AdminSet : cancel_admin_proposal(caller)\n• current admin only\n• clears PADMIN\n• emits adm_can event

    AdminSet --> Renounced : renounce_admin(admin)\n• removes ADMIN key\n• clears any pending PADMIN\n• emits adm_ren event\n• IRREVERSIBLE

    Renounced --> Renounced : all admin-gated calls\npermanently return Unauthorized
```

---

## Operator handoff (two-step)

```mermaid
stateDiagram-v2
    [*] --> OperatorSet : initialize()\n(OPERATOR = original_operator)

    OperatorSet --> PendingHandoff : propose_operator(admin, new_operator)\n• admin only\n• stores POP = new_operator\n• emits op_prop event

    PendingHandoff --> OperatorSet : accept_operator(new_operator)\n• must be called by proposed new_operator\n• OPERATOR ← new_operator\n• clears POP\n• emits op_acc event

    PendingHandoff --> OperatorSet : cancel_operator_proposal(admin)\n• admin only\n• clears POP\n• emits op_can event

    OperatorSet --> OperatorSet : set_operator(admin, new_operator)\n• direct (single-step) replacement\n• admin only\n• emits op_set event
```

> **Note:** `set_operator` is a direct single-step replacement (legacy path).
> The two-step `propose_operator` / `accept_operator` flow is preferred for
> operational safety because it requires the incoming operator to confirm.

---

## Combined orthogonal state matrix

The four axes (initialized, version-matched, paused, config-frozen) are
independent but **stack**: a call fails at the first guard it hits.

Guard evaluation order inside `calculate_sla` and `set_config`:

1. `check_version()` → fails with `VersionMismatch` or `NotInitialized`
2. `require_not_paused()` (calculate_sla) → fails with `ContractPaused`
3. `require_admin()` / `require_operator()` → fails with `Unauthorized`
4. `require_not_frozen()` (set_config) → fails with `ConfigFrozen`

```
                        │ Not Init │ NeedsMigr. │ Running │ Paused │
────────────────────────┼──────────┼────────────┼─────────┼────────┤
 initialize()           │    ✓     │     ✓      │    ✗    │   ✗    │
 migrate()              │    ✗     │     ✓      │   nop   │   ✓    │
 calculate_sla()        │    ✗     │     ✗      │    ✓    │   ✗    │
 set_config()           │    ✗     │     ✗      │  ✓/✗*   │   ✗    │
 pause() / unpause()    │    ✗     │     ✗      │    ✓    │  ✓/✓   │
 freeze/unfreeze()      │    ✗     │     ✗      │  ✓/✓*   │   ✓    │
 get_version_info()     │    ✗     │     ✓      │    ✓    │   ✓    │
 healthcheck()          │    ✓     │     ✓      │    ✓    │   ✓    │
 get_result_schema()    │    ✗     │     ✗      │    ✓    │   ✓    │

* depends on ConfigFrozen axis (✗ when frozen, ✓ when thawed)
```

---

## Invariants and guard table

| Invariant | Where enforced |
|-----------|---------------|
| `initialize()` is called exactly once | `ADMIN_KEY` presence check at start of `initialize()` |
| All versioned endpoints require `VER == STORAGE_VERSION` | `check_version()` called at the top of every public function (except bypass list) |
| `calculate_sla` is blocked while paused | `require_not_paused()` called before operator check |
| `set_config` is blocked while config is frozen | `require_not_frozen()` called after admin check |
| Admin transfer requires proposee to accept | `PENDING_ADMIN_KEY` must be present and caller must match |
| Operator handoff requires proposee to accept | `PENDING_OP_KEY` must be present and caller must match |
| `renounce_admin` is irreversible | `ADMIN_KEY` is removed; no path to re-set it without `initialize()` |
| Custom severities cannot shadow canonical ones | `is_canonical_severity()` check in `set_custom_severity()` |
| History capped at `MAX_HISTORY_SIZE` (1000) or configurable limit | FIFO trim after every `calculate_sla` write |
| One outage capped at `MAX_RECALCS_PER_OUTAGE` (16) retained entries | Scan + count inside `calculate_sla` |

---

## Source references

| Transition / guard | Implementation location |
|--------------------|------------------------|
| `initialize()` | `lib.rs::SLACalculatorContract::initialize` |
| `pause()` / `unpause()` | `lib.rs::SLACalculatorContract::pause` / `unpause` |
| `freeze_config()` / `unfreeze_config()` | `config_freeze.rs::freeze_config` / `unfreeze_config` |
| `migrate()` | `lib.rs::SLACalculatorContract::migrate` |
| `check_version()` | `lib.rs::SLACalculatorContract::check_version` |
| `propose_admin()` / `accept_admin()` | `governance.rs::propose_admin` / `accept_admin` |
| `renounce_admin()` | `governance.rs::renounce_admin` |
| `propose_operator()` / `accept_operator()` | `governance.rs::propose_operator` / `accept_operator` |
| `set_operator()` | `governance.rs::set_operator` |
