# ReentrancyGuard, ProxyUpgrade and Pausable Audit

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `ReentrancyGuard`, `ProxyUpgrade` or `Pausable` contract. The equivalents in this Soroban crate are:

- **ReentrancyGuard** - outbound cross-contract call isolation via `cross_contract_safety::CrossContractSafety`, role authentication checks (`require_admin` / `require_operator`), and the Checks-Effects-Interactions pattern.
- **ProxyUpgrade** - `migrate()` and `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3 -> v4), documented in `docs/MIGRATION_STATE_CONSUMPTION.md` and `docs/UPGRADE_PLAYBOOK.md`.
- **Pausable** - `pause()`, `unpause()`, `is_paused()`, and `get_pause_info()`, gated by `require_admin` and stored in instance storage (`PAUSED_KEY` / `PAUSE_INFO_KEY`).

## Audit of ReentrancyGuard access control modifiers (#761)

Every state-changing method strictly validates the caller's authority before executing logic or mutating instance storage:
- Admin-gated functions (`pause`, `unpause`, `set_config`, `freeze_config`, `unfreeze_config`, `prune_history`, `propose_admin`, `propose_operator`, `renounce_admin`, `migrate`) call `require_admin`.
- Calculation entrypoints (`calculate_sla`) require operator authorization via `require_operator`.
- Read-only queries (`is_paused`, `get_stats`, `get_config`, `get_migration_state`) are un-gated public views.
- Because authorization checks run before any state modification or external invocation, unauthorized callers cannot initiate operations that could participate in reentrant call flows. Pinned by `auth_matrix_tests.rs` (`reentrancy_guard_access_control_tests`).

## Refactoring ReentrancyGuard to OpenZeppelin primitives (#762)

OpenZeppelin is a Solidity framework. On Soroban:
- The standard building blocks are `Address::require_auth()`, typed `SLAError` enums, and `CrossContractSafety`.
- Reentrancy protection on Soroban is achieved by ensuring all internal state mutations occur before any outbound contract invocation (Checks-Effects-Interactions) and by using `CrossContractSafety` call stack tracking and compensation rollbacks.
- Outbound calls dispatched through `cross_contract_safety` track recursion depth and handle errors deterministically, ensuring that reentrancy or dispatch failures cannot corrupt storage. Pinned by `cross_contract_safety.rs` (`reentrancy_guard_primitives_tests`).

## Add storage gap to ProxyUpgrade for upgrade safety (#764)

Solidity storage gaps reserve sequential EVM storage slots (`uint256[50] __gap`) in upgradeable contracts to prevent slot collision when base contracts add variables.
- On Soroban, instance storage is key-value based (addressed by symbolic keys / `Symbol`) rather than contiguous sequential slots. There are no variable offset shifts or slot collisions.
- Upgrade safety is ensured by `STORAGE_VERSION`, explicit schema migrations in `migrate()`, and the reserved-key snapshot in `storage_key_invariant_tests.rs`.
- Pinned by `schema_migration_tests.rs` (`proxy_upgrade_storage_gap_safety_tests`), which verifies that upgrades preserve pre-existing key-value state without accidental overwriting.

## Optimize gas usage in Pausable (#771)

The pause mechanism has been optimized for gas and instruction efficiency:
- `pause` checks input length bounds (`MAX_REASON_LEN`) immediately, allowing invalid calls to fail fast before storage lookups or auth verification.
- Reuses the `env.storage().instance()` handle to avoid duplicate host environment lookups.
- `unpause` removes `PAUSE_INFO_KEY` and sets `PAUSED_KEY` to `false` via the cached storage handle.
- View functions like `is_paused` perform a single instance-storage read without extraneous overhead.
- Verified by automated budget tests (`test_pause_invalid_reason_fails_fast_gas` and `test_is_paused_read_gas_efficiency` in `apexchainx_calculator/src/tests.rs`).
