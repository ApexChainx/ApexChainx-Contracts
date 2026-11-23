# Errors, validation and security CI

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `AccessControl` or `ProxyUpgrade` contract. The equivalents are:

- **AccessControl** - the role checks `require_admin` and `require_operator` in `apexchainx_calculator/src/lib.rs`, documented in `docs/AUTH_MODEL.md` and pinned by `auth_matrix_tests.rs`.
- **ProxyUpgrade** - `migrate()` and the `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3), documented in `docs/MIGRATION_STATE_CONSUMPTION.md`.

## Custom errors (#743)

The contract already returns typed `SLAError` values (`Unauthorized`, `NotInitialized`, `VersionMismatch`, `AdminRenounced`, ...) instead of string messages. `migrate()` uses these exclusively.

## Input validation (#740)

`migrate(caller)` takes only the caller, which is authenticated with `require_auth()` and compared with the stored admin. State is checked before any write (`NotInitialized`, `Unauthorized`, `VersionMismatch`).

## Security CI (#737)

Slither targets Solidity. For this Rust crate the equivalent static checks are `cargo clippy` (denying warnings), `cargo audit` and `cargo deny`. `.github/workflows/security.yml` now includes a clippy job next to the audit and deny jobs.

## Tests (#736)

`auth_matrix_tests.rs` contains `access_control_unit_tests` covering the negative role paths.

