# Storage Gap Rationale for ApexChainx Governance

## TL;DR

Storage gaps are not used in this contract. Soroban's key-value storage model
makes them unnecessary and counterproductive.

## What is a storage gap?

In Ethereum upgradeable contracts (using the proxy pattern with `delegatecall`),
storage variables are laid out sequentially in numbered slots. If an upgradeable
base contract does not reserve empty slots, a new version that adds state
variables will collide with derived contracts that start reading from the same
slot positions.

The conventional fix is to add a `uint256[50] __gap` array at the end of the
base contract, consuming 50 slots so derived contracts start at a predictable
offset.

## Why Soroban does not need storage gaps

Soroban uses an explicit **key-value** storage model, not sequential slot
numbering. Every storage entry is addressed by a named `Symbol` key rather
than an implicit slot offset. Examples from this contract:

```rust
pub(crate) const ADMIN_KEY: Symbol = symbol_short!("ADMIN");
pub(crate) const PENDING_ADMIN_KEY: Symbol = symbol_short!("PADMIN");
pub(crate) const PENDING_OP_KEY: Symbol = symbol_short!("POP");
```

Adding a new storage entry in a future upgrade simply introduces a new key. No
existing key shifts because no key occupies an "adjacent" position. There is no
concept of a slot offset to collide with.

## Governance-specific notes

The governance module (`governance.rs`) stores four keys:

- `ADMIN_KEY` — current admin address
- `OPERATOR_KEY` — current operator address
- `PENDING_ADMIN_KEY` + `PENDING_ADMIN_TS_KEY` — pending admin proposal
- `PENDING_OP_KEY` + `PENDING_OP_TS_KEY` — pending operator proposal
- `ADMIN_RENOUNCED_KEY` — set permanently on renounce

Any future addition (e.g. a `PENDING_ADMIN2_KEY` for a multi-sig path) adds a
new `symbol_short!` constant and a corresponding entry in the storage key
collision test (`test_storage_key_namespace_symbols_are_distinct` in
`tests.rs`). It cannot collide with existing keys.

## What does apply: key namespace uniqueness

The relevant invariant in this project is **symbol uniqueness**, not slot
spacing. Every storage key constant must have a distinct `Symbol` value. The
enforcement mechanism is:

```
apexchainx_calculator/src/tests.rs
→ test_storage_key_namespace_symbols_are_distinct
```

Any new governance key must be added to that test's exhaustive list.

## See also

- `docs/STORAGE_KEY_MIGRATION_CHECKLIST.md` — step-by-step process for adding
  a new storage key.
- `docs/RESERVED_KEYS_POLICY.md` — key prefix conventions.
- `src/lib.rs` — the authoritative list of all `*_KEY` constants.
- `src/storage_version.rs` — migration logic for evolving the schema.
