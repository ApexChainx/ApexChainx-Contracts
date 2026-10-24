# Issue #777 — Pausable Storage Gap for Upgrade Safety

## Resolution

This issue is **resolved without adding a storage gap**. The request for a
storage gap in the Pausable logic is not applicable to Soroban smart contracts.
This document explains why and describes the equivalent safety mechanism that
is in place.

## Why Storage Gaps Do Not Apply to Soroban

Storage gaps are an Ethereum/EVM pattern. In Solidity, upgradeable contracts
use `delegatecall` and store state in sequentially numbered slots. If a base
contract does not reserve empty slots (e.g. `uint256[50] __gap`), adding a new
state variable in an upgraded version collides with derived contract state that
starts at the same slot offset.

**Soroban uses an explicit key-value storage model.** Every storage entry is
addressed by a named `Symbol` key, not by a slot offset. There is no concept
of "the next available slot" because entries are never adjacent. Adding a new
storage entry in a future upgrade simply introduces a new named key — it cannot
collide with any existing key.

See [docs/STORAGE_GAP_RATIONALE.md](STORAGE_GAP_RATIONALE.md) for the
project-wide rationale.

## Pausable Storage Keys

The Pausable logic in `apexchainx_calculator/src/metadata.rs` uses exactly two
storage keys, both defined as constants in `src/lib.rs`:

| Constant         | Symbol value | Purpose                                      |
|------------------|--------------|----------------------------------------------|
| `PAUSED_KEY`     | `"PAUSED"`   | Boolean flag: `true` when contract is paused |
| `PAUSE_INFO_KEY` | `"PAUSEINF"` | `PauseInfo` struct: reason, timestamp, caller|

These key names are **stable and locked**. Renaming either constant would
silently corrupt deployed contract state by leaving the old key orphaned and
reading a missing value as the default on the new key. They must never change
without a coordinated on-chain migration.

## Invariant Enforcement

The key name invariant is enforced at two levels:

1. **Unit test** — `test_pausable_storage_keys_are_stable` in
   `apexchainx_calculator/src/tests.rs` asserts at compile-and-test time that:
   ```rust
   assert_eq!(crate::PAUSED_KEY,      soroban_sdk::symbol_short!("PAUSED"));
   assert_eq!(crate::PAUSE_INFO_KEY,  soroban_sdk::symbol_short!("PAUSEINF"));
   ```
   Any rename of the constant's symbol literal will fail this test before it
   reaches review.

2. **Namespace collision test** — `test_storage_key_namespace_symbols_are_distinct`
   in `tests.rs` includes both `PAUSED_KEY` and `PAUSE_INFO_KEY` in its
   exhaustive deduplicated list, ensuring no future key can reuse `"PAUSED"` or
   `"PAUSEINF"` as its symbol string.

3. **Schema version pinning** — `storage_key_invariant_tests.rs` pins the full
   set of instance-storage key strings to `STORAGE_VERSION`. Any change to the
   set (add, remove, or rename) must be accompanied by a `STORAGE_VERSION` bump
   and a snapshot update.

## What Would Be Needed for a True Upgrade

If the Pausable logic ever needed to be restructured (e.g., splitting
`PAUSE_INFO_KEY` into two separate keys), the correct procedure is:

1. Bump `STORAGE_VERSION` in `lib.rs`.
2. Add migration logic in `storage_version.rs` to write the new keys from the
   old ones.
3. Update `KEYS_AT_V<N>` snapshot and advance `SNAPSHOT_STORAGE_VERSION` in
   `storage_key_invariant_tests.rs`.
4. Add the new constants to `test_storage_key_namespace_symbols_are_distinct`.
5. Follow the full checklist in `docs/STORAGE_KEY_MIGRATION_CHECKLIST.md`.

## See Also

- [docs/STORAGE_GAP_RATIONALE.md](STORAGE_GAP_RATIONALE.md) — project-wide rationale
- [docs/RESERVED_KEYS_POLICY.md](RESERVED_KEYS_POLICY.md) — key prefix conventions
- [docs/STORAGE_KEY_MIGRATION_CHECKLIST.md](STORAGE_KEY_MIGRATION_CHECKLIST.md) — step-by-step process for adding/changing keys
- `apexchainx_calculator/src/lib.rs` — canonical `*_KEY` constant definitions
- `apexchainx_calculator/src/storage_key_invariant_tests.rs` — schema version pinning
