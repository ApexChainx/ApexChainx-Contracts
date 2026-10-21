# ReentrancyGuard and Pausable review

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `ReentrancyGuard` or `Pausable` inheritance hierarchy. The equivalents are:

- **ReentrancyGuard** - `cross_contract_safety::CrossContractSafety` and `safe_invoke_contract` for cross-contract outbound calls with compensation rollbacks, plus Checks-Effects-Interactions pattern across all state-modifying entrypoints in `apexchainx_calculator/src/lib.rs`.
- **Pausable** - `pause()`, `unpause()`, `is_paused()`, and `get_pause_info()` in `apexchainx_calculator/src/lib.rs`, enforced by `require_not_paused` in `calculation.rs`.

## Deployment script (#768)

`scripts/deploy-reentrancy-guard.sh` builds the contract wasm, deploys it to the network via Stellar CLI, and initializes the contract roles with reentrancy protection verified.

## Storage gap for upgrade safety (#766)

Solidity storage gaps reserve storage slots in upgradeable base contracts (e.g. `uint256[50] private __gap`) to prevent memory/slot misalignment during upgrade inheritance. On Soroban, contract storage is an instance-scoped key-value map keyed by explicit `Symbol` identifiers (such as `ADMIN`, `CONFIG`, `PAUSED`) rather than sequential slot offsets. Therefore, storage gaps are neither necessary nor applicable. Upgrade safety is enforced deterministically by `STORAGE_VERSION`, versioned `migrate()` arms, and the strict reserved key invariant asserted in `storage_key_invariant_tests.rs`.

## Integer overflow/underflow risks (#763)

In Solidity, custom reentrancy guards often use integer nonces or call counters that may be vulnerable to overflow/underflow if unchecked. In this crate:
- Cross-contract invocation depth is tracked via `CrossContractSafety::depth()`, bounded by `MAX_CALL_DEPTH` (= 64) with checked remaining capacity.
- Status values use a typed enum (`CrossContractCallStatus`) rather than raw integers.
- All calculator and governance math uses `checked_*` / `saturating_*` arithmetic, preventing any overflow or underflow vulnerabilities.

## Unit tests for Pausable (#770)

`pausable_unit_tests` in `apexchainx_calculator/src/tests.rs` provides comprehensive coverage for:
- Initial unpaused state (`is_paused() == false`, `get_pause_info() == None`).
- Admin can pause with metadata (reason, timestamp, caller) and emit `paused` event.
- Stranger and operator cannot pause or unpause (`Unauthorized`).
- Admin can unpause, clearing `PAUSE_INFO_KEY` and emitting `unpause` event.
- Idempotent pause/unpause behavior and reason string length bounds validation (`MAX_REASON_LEN`).
- Calculation calls (`calculate_sla`) are blocked while paused (`ContractPaused`) and resume after unpause.
