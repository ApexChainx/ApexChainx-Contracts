# Upgrade, reentrancy and gas review

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `AccessControl` or `ProxyUpgrade` contract. The equivalents are:

- **AccessControl** - the role checks `require_admin` and `require_operator` in `apexchainx_calculator/src/lib.rs`, documented in `docs/AUTH_MODEL.md` and pinned by `auth_matrix_tests.rs`.
- **ProxyUpgrade** - `migrate()` and the `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3), documented in `docs/MIGRATION_STATE_CONSUMPTION.md`.

## Reentrancy (ProxyUpgrade #738, ReentrancyGuard #752, #763, #766, #768)

Reentrancy in the Solidity sense needs an outbound call that can call back before state is written. In this crate the only outbound call helper is `cross_contract_safety::safe_invoke_contract`, which wraps `try_invoke_contract` and translates failures into errors. The calculation, config, governance and `migrate()` paths do not call other contracts, so no guard is required. Any future outbound call must go through `safe_invoke_contract` and write state before calling out.

### Deployment script for ReentrancyGuard (#768)

`scripts/deploy-reentrancy-guard.sh` builds the release WASM package, deploys it to the network via the Stellar CLI, and initializes contract roles, verifying that the contract's reentrancy-safe architecture and compensation mechanics are active from genesis.

### Integer overflow/underflow in ReentrancyGuard (#763)

In Solidity, reentrancy guards often rely on integer state variables or invocation counters that can risk overflow or underflow if not bounded. In `apexchainx_calculator/src/cross_contract_safety.rs`, the call depth is strictly bounded by `MAX_CALL_DEPTH` (= 64) and checked via `can_push()` and `checked_remaining_capacity()`. Stack rollbacks on empty state safely no-op without underflow. Furthermore, call statuses use the strongly-typed `CrossContractCallStatus` enum, and all contract math uses `checked_*` or `saturating_*` arithmetic.

### Storage gap in ReentrancyGuard for upgrade safety (#766)

Solidity storage gaps (such as `uint256[50] private __gap`) reserve unassigned storage slots to prevent layout collisions between base and derived upgradeable contracts. On Soroban, contracts do not use positional storage slots; state is maintained in an instance key-value store indexed by distinct `Symbol` keys (e.g., `ADMIN`, `CONFIG`, `PAUSED`). Consequently, storage gaps are neither required nor applicable. Upgrade safety is preserved through `STORAGE_VERSION`, versioned `migrate()` arms, and the reserved key snapshot invariant in `storage_key_invariant_tests.rs`.

## Pausable unit tests (#770)

Pausable lifecycle management is provided through `pause()`, `unpause()`, `is_paused()`, and `get_pause_info()`. Comprehensive unit test coverage in `apexchainx_calculator/src/tests.rs` validates:
- Initial state is unpaused (`is_paused() == false`, `get_pause_info() == None`).
- Admin can pause and unpause; metadata (`reason`, `paused_at`, `paused_by`) is captured on pause and cleared on unpause.
- Strangers and operators cannot pause or unpause (rejected with `Unauthorized`).
- Idempotent unpausing and metadata replacement on consecutive pauses.
- Reason string length bounded by `MAX_REASON_LEN` (exceeding length returns `InvalidInput`).
- State-mutating calculation invocations (`calculate_sla`) are blocked while paused (`ContractPaused`) and resume after unpause.
- Read-only contract queries remain accessible while paused.

## Access control of the upgrade path and storage gaps (#744, #764)

`migrate(caller)` calls `caller.require_auth()`, loads `ADMIN_KEY` (`NotInitialized` if absent) and rejects any other caller with `Unauthorized`. It is idempotent at the current version, rejects a stored version newer than the binary (`VersionMismatch`), and publishes a `migrate_done` event with `(stored, current)`. Operators and strangers cannot migrate (see `auth_matrix_tests.rs`).

Regarding storage gaps (#764): Solidity upgradeable contracts reserve `uint256[50] __gap` slots to avoid variable collision across inheritance hierarchies. Soroban instance storage is addressed by symbolic keys (`Symbol`) rather than contiguous numerical slots. Layout collision cannot occur, and upgrade safety is enforced by `STORAGE_VERSION`, migration arms in `migrate()`, and the snapshot in `storage_key_invariant_tests.rs` (pinned in `proxy_upgrade_storage_gap_safety_tests`).

## Gas / resource use and Pausable optimization (#741, #752, #771)

- History is stored as per-entry sub-keys with a per-outage index (v3), so appends do not rewrite every retained entry.
- History length and config count are cached counters (`HISTLEN`, `CFGCNT`) instead of being recomputed on each read.
- `migrate()` derives the cached values once from the source of truth; read paths use the counters.
- Migration arms run in sequence in one invocation; each arm only touches the keys it introduces.
- **Pausable gas optimization (#771)**: `pause` validates input length bounds (`MAX_REASON_LEN`) immediately to fail fast without state lookups on invalid input, and reuses the instance storage handle rather than re-requesting it multiple times. Both `pause` and `unpause` execute well within the 160k instruction budget (verified by `test_pause_invalid_reason_fails_fast_gas` and `test_is_paused_read_gas_efficiency`).



