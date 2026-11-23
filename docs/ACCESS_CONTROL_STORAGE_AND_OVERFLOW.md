# Storage layout, overflow and events

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `AccessControl` or `ProxyUpgrade` contract. The equivalents are:

- **AccessControl** - the role checks `require_admin` and `require_operator` in `apexchainx_calculator/src/lib.rs`, documented in `docs/AUTH_MODEL.md` and pinned by `auth_matrix_tests.rs`.
- **ProxyUpgrade** - `migrate()` and the `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3), documented in `docs/MIGRATION_STATE_CONSUMPTION.md`.

## Storage gap (#731)

Solidity storage gaps reserve slots in upgradeable base contracts. Soroban instance storage is keyed by named symbols, not positional slots, so there is nothing to reserve. Upgrade safety comes from `STORAGE_VERSION`, the `migrate()` arms, and the reserved-key snapshot in `storage_key_invariant_tests.rs`, which fails CI if a key is added without a version bump.

## Integer overflow/underflow (#730)

Calculation and governance code uses `checked_*` / `saturating_*` arithmetic (see `calculation.rs` and `lib.rs`) and validates inputs through `validate_config` (`InvalidThreshold`, `InvalidPenalty`, `InvalidReward`, `InvalidSeverity`) before storing them. Role checks do no arithmetic.

## Events (#745)

`migrate()` publishes `migrate_done` with topics `(name, version, caller)` and data `(stored_version, current_version)`. Admin-role changes publish their own versioned events through `event_schema`.

## Tests (#746)

`schema_migration_tests.rs` contains `proxy_upgrade_unit_tests` covering stranger rejection and idempotent migration.

