# Public API Manifest vs Compiled Client Surface

> **Issue:** [#682](https://github.com/ApexChainx/ApexChainx-Contracts/issues/682)
> **Status:** Findings recorded; enforcement not yet implemented
> **Applies to:** `apexchainx_calculator` `get_public_api` descriptor, `#[contractimpl]` block

This document records the verification method used to compare the hand-maintained
public API manifest against the compiled client surface, and the drift that
comparison found. It is the reference for the enforcement work described in
[#682](https://github.com/ApexChainx/ApexChainx-Contracts/issues/682).

**Related documents:**
- [`API_MANIFEST.md`](API_MANIFEST.md) - the existing change procedure and manifest ownership rules

---

## 1. The three lists that must agree

There are currently **three** independently hand-maintained representations of the
public surface. Only one of them is authoritative.

| List | Location | Nature |
| --- | --- | --- |
| Compiled client | `#[contractimpl]` block, `apexchainx_calculator/src/lib.rs` | **Authoritative.** The `#[contractimpl]` macro generates `SLACalculatorContractClient` from this block. |
| Runtime descriptor | `get_public_api`, `lib.rs:2420`-`2534` | Hand-maintained. Built from a literal `methods.push_back(method(...))` chain. |
| Test manifest | `CANONICAL_PUBLIC_METHODS`, `apexchainx_calculator/src/tests.rs` (the `#492` section) | Hand-maintained. A `&[(&str, bool, &str, &str)]` literal. |

The two hand-maintained lists are the problem. Nothing derives either from the
compiled surface, so both can be wrong together and no check will notice.

## 2. Why the existing guardrail does not catch drift

`docs/API_MANIFEST.md` states the invariant is enforced in both directions by two
regression tests:

| Test | Direction enforced |
| --- | --- |
| `test_492_descriptor_covers_every_public_method` | manifest -> descriptor |
| `test_492_descriptor_lists_only_declared_methods` | descriptor -> manifest |

Both tests compare the **runtime descriptor** against the **test manifest**. The
`client` handle in each test is used only to transport the runtime
`get_public_api()` return value; neither test ever enumerates the methods the
compiler actually generated.

So the pair closes the loop between two hand-written literals and leaves the
`#[contractimpl]` block unverified. A method can be added to or removed from the
impl block and both tests still pass, provided the two literals are kept
consistent with each other.

The in-file comment above `CANONICAL_PUBLIC_METHODS` is explicit that the auth,
mutates, and event columns "cannot be derived mechanically from the Rust
signature" and are therefore pinned manually. That is a correct constraint for
those three metadata columns. It is **not** a correct reason to leave the method
**name set** unverified, because the name set is exactly what the compiler knows.

## 3. Drift found by this comparison

Comparing the 63 `pub fn` items in the `#[contractimpl]` block against the 63
descriptor entries produces one divergence in each direction:

| Method | In compiled client | In runtime descriptor | In test manifest |
| --- | --- | --- | --- |
| `get_retention_metrics` | **No** | Yes | Yes |
| `get_config_snapshot_by_version` | Yes | **No** | **No** |

**`get_retention_metrics` is a phantom.** It is advertised by `get_public_api` and
pinned by `CANONICAL_PUBLIC_METHODS`, but no such function exists in the
`#[contractimpl]` block. A consumer that trusts the documented discovery
mechanism and calls it receives a missing-function contract error.

**`get_config_snapshot_by_version` is unlisted.** It is a real, public, read-only
method added for
[#408](https://github.com/ApexChainx/ApexChainx-Contracts/issues/408) and callable
today, but it does not appear in the descriptor, so it is undiscoverable through
`get_public_api`.

Two consequences worth recording:

- The two hand lists are **byte-identical in membership**, which is precisely why
  both `test_492_*` tests pass while the surface is wrong. Passing tests here are
  not evidence of correctness.
- The descriptor is documented as "All public methods in alphabetical order"
  (`lib.rs:1041`) but is not strictly sorted: `get_last_config_update` is listed
  before `get_latest_by_outage`. `test_get_public_api_is_deterministic` only
  checks repeat-call stability, not sortedness.

`scripts/coverage-matrix.ts` also lists `get_retention_metrics` as a live method,
which propagates the phantom into the coverage matrix.

## 4. Verification method

The comparison is a source scan of `lib.rs`, following the pattern already used
elsewhere in this crate:

- `apexchainx_calculator/src/orphan_lint_tests.rs` reads `src/lib.rs` from
  `env!("CARGO_MANIFEST_DIR")` at test time and diffs declared modules against
  real files.
- `apexchainx_calculator/src/event_schema.rs` walks `src/*.rs` and asserts every
  declared event has a real emit site, excluding only the declaration files.

The events subsystem already closes this loop; the methods subsystem does not.
The method-name set should be held to the same standard.

The check needs no new dependencies. `serde` and `serde_json` are only present in
`Cargo.lock` transitively through `soroban-sdk` and are not declared in
`apexchainx_calculator/Cargo.toml`, so they are not importable from a test as the
crate stands. A `std::fs` source scan needs nothing new, and adding an unused
dev-dependency would trip the `cargo machete` and `cargo udeps` gates in
`.github/workflows/ci.yml`.

## 5. Enforcement required to close #682

Acceptance criteria from the issue, and what each still needs:

| Criterion | State |
| --- | --- |
| Manifest matches the client's method set | **Not met.** One phantom and one omission, above. |
| New/renamed methods auto-flagged in CI | **Not met.** No job compares the three lists. |
| Docs render the manifest as the API reference source | **Partially met** by this document and `API_MANIFEST.md`. |

To meet the first two, the check must be anchored on the compiled surface rather
than on a second literal. Concretely, a test that extracts `pub fn` names from the
`#[contractimpl]` block and compares that set against the descriptor would fail on
both the phantom and the omission, and would fail again automatically the next
time a method is added to the impl without a descriptor entry.

Until that check lands, the descriptor and the compiled client are known to
disagree, and the phantom in section 3 should be treated as a live defect for
consumers that use `get_public_api` for discovery.
