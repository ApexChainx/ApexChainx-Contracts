# Access control audit, errors and gas

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `AccessControl` or `ProxyUpgrade` contract. The equivalents are:

- **AccessControl** - the role checks `require_admin` and `require_operator` in `apexchainx_calculator/src/lib.rs`, documented in `docs/AUTH_MODEL.md` and pinned by `auth_matrix_tests.rs`.
- **ProxyUpgrade** - `migrate()` and the `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3), documented in `docs/MIGRATION_STATE_CONSUMPTION.md`.

## Audit of access control modifiers (#724)

Every state-changing method calls `require_admin`, `require_operator` or `require_auth()` on the relevant address (see the tier table in `docs/AUTH_MODEL.md`). Read-only methods are public by design. `auth_matrix_tests.rs` pins this.

## Custom errors (#727)

Failures use the typed `SLAError` enum rather than string messages, so clients match on stable numeric codes (`Unauthorized = 3`, `AdminRenounced`, ...).

## Gas / resource use (#734)

Role checks do one instance-storage read and one address comparison. The admin renounce flag is checked with `has()` before the admin read, so a renounced contract fails fast.

## Fuzz tests (#722)

`fuzz_tests.rs` contains `access_control_fuzz_tests`: many random unauthorised callers can never pause, configure or migrate the contract, while the admin keeps working.

