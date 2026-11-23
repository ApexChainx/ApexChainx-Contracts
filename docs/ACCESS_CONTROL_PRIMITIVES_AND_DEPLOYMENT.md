# Access control primitives, validation and deployment

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `AccessControl` or `ProxyUpgrade` contract. The equivalents are:

- **AccessControl** - the role checks `require_admin` and `require_operator` in `apexchainx_calculator/src/lib.rs`, documented in `docs/AUTH_MODEL.md` and pinned by `auth_matrix_tests.rs`.
- **ProxyUpgrade** - `migrate()` and the `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3), documented in `docs/MIGRATION_STATE_CONSUMPTION.md`.

## Primitives (#733)

OpenZeppelin is a Solidity library. On Soroban the equivalent primitive is `Address::require_auth()`, which the contract already uses through `require_admin` / `require_operator`; no custom signature handling exists. Role state lives in instance storage (`ADMIN`, `OPERATOR`, `PADMIN`, `POP`).

## Input validation (#726)

Role-changing methods take `Address` values authenticated by `require_auth()`; proposals must be accepted by the proposed address, and a renounced admin is rejected with `AdminRenounced`.

## Deployment (#728)

`scripts/deploy-access-control.sh` builds the wasm, deploys it with the Stellar CLI and calls `initialize` with the admin and operator addresses.

## Invariant tests (#729)

`storage_key_invariant_tests.rs` contains `access_control_invariant_tests`: repeated pause/unpause cycles never change roles or stats.

