# API Manifest

> **Issue:** [#633](https://github.com/ApexChainx/ApexChainx-Contracts/issues/633)
> **Status:** Active
> **Applies to:** `apexchainx_calculator` public surface, `get_public_api` descriptor

This document is the source-of-truth contract describing how the public API manifest
is produced, verified, and kept aligned with the compiled `#[contractimpl]` surface.

---

## 1. Where the manifest lives

The canonical manifest is the **`CANONICAL_PUBLIC_METHODS`** table in
`apexchainx_calculator/src/tests.rs` (`test_492` section). It is an impl-truth,
hand-maintained list that must match the `#[contractimpl]` block in `lib.rs`
**in both directions**:

- **Every declared public method** must appear in the manifest.
- **Every manifest entry** must exist on the impl.
- **Metadata must match byte-for-byte**: `name`, `mutates`, `auth`, `event`.

`get_public_api` returns this same surface as a versioned descriptor built
directly in `lib.rs` (the `methods.push_back(method(...))` block), alongside the
impl partial-eq chain naturally enforcing the RHS. The descriptor and the
manifest are mechanically cross-checked by two regression tests:

| Test | Direction enforced |
| --- | --- |
| `test_492_descriptor_covers_every_public_method` | impl-truth manifest → descriptor (no missing method, no metadata drift) |
| `test_492_descriptor_lists_only_declared_methods` | descriptor → impl-truth manifest (no phantom methods) |

Keeping both lists in sync is enforced by those tests; this file exists so the
invariant has a discoverable home and a change procedure.

## 2. Metadata columns

| Column | Meaning |
| --- | --- |
| `mutates` | Whether the method can mutate ledger state |
| `auth` | Auth tier required: `admin`, `operator`, `addr` (custom address), or `none` (public) |
| `event` | Event symbol the method emits, or `""` if read-only |

## 3. Change procedure for the public surface

1. Add/remove the method in the `#[contractimpl]` block in `src/lib.rs`.
2. Update the `methods.push_back(method(...))` descriptor entries in `src/lib.rs`.
3. Update `CANONICAL_PUBLIC_METHODS` in `src/tests.rs` to match exactly.
4. Regenerate `test_snapshots/tests/test_get_public_api_*.json` (the
   `test-archive` script) and `ts/fixtures` via the parity harness.
5. Run the `test_492_*` tests; both directions must pass.

## 4. Concrete drift fixed in #633

`replay_calculate_sla` is a public, read-only, event-less replay of a prior
decision. The manifest previously advertised it as `(mutates=true, auth=operator,
event=sla_calc)`, which contradicted the compiled surface (no `require_auth`, no
state writes, no `sla_calc` emission). It is now recorded as
`(mutates=false, auth=none, event="")`, matching the client-visible behavior.