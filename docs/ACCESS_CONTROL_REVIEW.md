# Access control review

The contract is a Soroban crate (`apexchainx_calculator`); there is no Solidity `AccessControl` or `ProxyUpgrade` contract. The equivalents are:

- **AccessControl** - the role checks `require_admin` and `require_operator` in `apexchainx_calculator/src/lib.rs`, documented in `docs/AUTH_MODEL.md` and pinned by `auth_matrix_tests.rs`.
- **ProxyUpgrade** - `migrate()` and the `STORAGE_VERSION` steps (v0 -> v1 -> v2 -> v3), documented in `docs/MIGRATION_STATE_CONSUMPTION.md`.

## Documentation (#732)

- `require_admin(env, caller)`: `caller.require_auth()`, then `AdminRenounced` if the admin role was renounced, `NotInitialized` if no admin is stored, `Unauthorized` if the caller is not the stored admin.
- `require_operator(env, caller)`: `caller.require_auth()`, then `NotInitialized` / `Unauthorized` against the stored operator.
- Two-step handover: `propose_admin` / `accept_admin` and `propose_operator` / `accept_operator`; the pending address must sign.

## Reentrancy review (#725)

Role checks only read instance storage and make no outbound calls. The only cross-contract call helper is `safe_invoke_contract`, which uses `try_invoke_contract`. No reentrancy path exists through the access-control code.

## Events (#723)

Role changes publish versioned events through `event_schema` (see `docs/EVENT_COMPATIBILITY_POLICY.md`); consumers should key off the topic name and event version.

## Integration tests (#735)

`event_state_tests.rs` contains `access_control_integration_tests`: admin handover moves authority, and operator rotation keeps admin authority.

