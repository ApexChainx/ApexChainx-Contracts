# Upgrade flow verification

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `AccessControl` or `ProxyUpgrade` contract. The equivalents are:

- **AccessControl** - the role checks `require_admin` and `require_operator` in `apexchainx_calculator/src/lib.rs`, documented in `docs/AUTH_MODEL.md` and pinned by `auth_matrix_tests.rs`.
- **ProxyUpgrade** - `migrate()` and the `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3), documented in `docs/MIGRATION_STATE_CONSUMPTION.md`.

## Documentation (#742)

`migrate(caller)` (admin only) brings instance storage up to `STORAGE_VERSION`:

| Step | Effect |
|------|--------|
| v0 -> v1 | Initialises missing storage defaults and stamps the version |
| v1 -> v2 | Backfills the cached history length from the stored vector |
| v2 -> v3 | Converts history to per-entry sub-keys with an outage index and backfills the config count |

It returns `Ok(())` when already current, `VersionMismatch` when the stored version is newer than the binary, and publishes `migrate_done` on success. Use `get_migration_state()` to see whether a migration is needed.

## Tests (#739, #747, #748)

`topic_stability_tests.rs` contains `proxy_upgrade_flow_tests`:

- integration: migrate, then normal operation continues;
- invariant: repeated migration does not change stats;
- fuzz-style: many unrelated callers can never migrate, the admin still can.

