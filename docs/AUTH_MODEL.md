# SLA Contract Authorization Model (#634)

This document is the authoritative reference for *who may call each endpoint*
on the SLA calculator contract. The enforcement lives in the contract methods
(`require_admin`, `require_operator`, `require_auth()`) and is **pinned by
tests** in `apexchainx_calculator/src/auth_matrix_tests.rs` — the matrix test
encodes exactly the model below so future refactors cannot silently change
authorization.

## Tiers

| Tier | Meaning | Auth enforced |
|------|---------|---------------|
| `operator-mutating` | Writes state; must be initiated by the current operator | `require_auth()` + operator-role match |
| `admin` | Writes/prunes administrative state | `require_auth()` + admin-role match |
| `addr` | Self-authenticating; the pending/revealed address itself must sign | `require_auth()` + caller == stored slot |
| `public-view` | Read-only, no auth, **live** computation over current config | none |
| `public-replay` | Read-only, no auth, deterministic **replay** of a decision | none |

## Per-endpoint model

### Calculation surface

| Method | Tier | Notes |
|--------|------|-------|
| `calculate_sla` | operator-mutating | The only mutating evaluation path. Writes history, stats, telemetry; emits `sla_calc`/`set_int`. Operator-only (#28). |
| `calculate_sla_view` | public-view | Live read-only evaluation over the **current** config. No storage writes, no events. `auth_matrix_tests` asserts strangers can call it. |
| `replay_calculate_sla` | public-replay | Deterministic replay of what `calculate_sla` *would* have produced, without writing state or emitting events. **No operator authorization and no pause check** — deliberately public (read-only audit helper, #95). `auth_matrix_tests` asserts strangers can call it and that it has no side-effects. |

> **Why replay is public:** replay is a pure function over current config. It
> never touches storage, so authorizing it would add a signature requirement
> with zero security benefit for a read. It is kept distinct from
> `calculate_sla_view` because view computes a **live** evaluation of the
> current instance while replay deterministically re-derives the decision a
> *mutating* submission would have stored.

The `get_public_api()` descriptor reflects these tiers in its `auth` column
(`calculate_sla`=`operator`, `calculate_sla_view`=`none`,
`replay_calculate_sla`=`none`).

### Lifecycle / governance surface

| Method | Tier |
|--------|------|
| `accept_admin` / `accept_operator` | `addr` (the pending candidate self-authenticates; #426) |
| `cancel_admin_proposal` / `cancel_operator_proposal` | admin |
| `propose_admin` / `propose_operator` | admin |
| `renounce_admin` | admin |
| `set_operator` | admin (legacy single-step break-glass) |

### Config & admin surface

| Method | Tier |
|--------|------|
| `set_config`, `set_custom_severity`, `remove_custom_severity` | admin |
| `freeze_config` / `unfreeze_config` | admin |
| `set_retention_limit`, `prune_history`, `prune_history_by_age` | admin |
| `pause` / `unpause` | admin |
| `migrate` | admin |
| `initialize` | `multi` (both the admin **and** operator addresses must authorize; #425) |
| all `get_*` / `list_*` / `healthcheck` / `is_*` queries | public-view (`none`) |
| `get_public_api` | public-view (`none`) — deliberately callable pre-migration |

## Rules for future refactors

1. **No silent auth changes.** If a method's tier changes, update this doc,
   the `CANONICAL_PUBLIC_METHODS` manifest pins in `tests.rs`, the
   `get_public_api()` descriptor, and the auth matrix test **in the same
   commit**.
2. **`replay_calculate_sla` must stay read-only.** Because it is unauthenticated,
   it must never gain write side-effects. `auth_matrix_tests` contains a
   no-side-effects test for it.
3. **Public ≠ uninstrumented.** Public read views are still version-checked
   (`check_version`) and must surface `NotInitialized` consistently with other
   methods.