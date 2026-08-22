# History Pagination Policy

> **Status:** Active
> **Reference:** [Issue #263](https://github.com/ApexChainx/ApexChainx-Contracts/issues/263)
> **Last updated:** 2026-08-20
> **Audience:** Backend consumers, operators, and contract contributors

## Table of Contents

- [Overview](#overview)
- [Contract implementation](#contract-implementation)
- [Policy: offset semantics](#policy-offset-semantics)
- [Policy: limit & page size](#policy-limit--page-size)
- [Policy: end-of-history signalling](#policy-end-of-history-signalling)
- [Policy: pagination metadata](#policy-pagination-metadata)
- [Policy: overflow safety](#policy-overflow-safety)
- [Policy: ordering & stability](#policy-ordering--stability)
- [Canonical source of truth](#canonical-source-of-truth)

## Overview

The contract exposes SLA calculation history through a set of read-only
accessors backed by the append-only `HISTORY_KEY` vector
(`apexchainx_calculator/src/history.rs`, storage key `HIST`). This policy
defines how paginated history reads behave so that backend consumers can
page through arbitrarily large histories deterministically, without relying
on undocumented behaviour.

The canonical behaviour is the contract's actual return behaviour; this
document exists to make that behaviour explicit and reviewable.

## Contract implementation

The paginated accessors are:

```rust
pub fn get_history_page(env: Env, offset: u32, limit: u32) -> Result<Vec<SLAResult>, SLAError>
pub fn get_history_page_with_meta(env: Env, offset: u32, limit: u32) -> Result<HistoryPage, SLAError>
```

Each is implemented in two places that must stay in lockstep:

- `apexchainx_calculator/src/lib.rs` — the `#[contractimpl]` entry point
  (the on-chain method consumers call).
- `apexchainx_calculator/src/history.rs` — the module-level helper.

Both read the full history vector, compute `end = min(saturating_add(offset, limit), len)`,
and return the slice `history[offset..end]`. History is capped at
`MAX_HISTORY_SIZE` (1000) entries and is append-only; entries are never
reordered, so pagination is stable across calls.

## Policy: offset semantics

- `offset` is the **0-based index** of the first entry to return.
- History is stored **oldest-first** (insertion order). `offset = 0` is the
  earliest recorded result; the largest valid offset is `len - 1`.
- An `offset >= len` (including any offset on an empty history) returns an
  **empty page** — it is **not** an error.
- Offsets are `u32`. Extreme offsets such as `u32::MAX` are therefore
  representable and simply produce an empty page.

## Policy: limit & page size

- `limit` is the **maximum number of entries** returned per page.
- `limit` is **not** clamped to an upper bound. The effective page size is
  `min(limit, len - offset)`: a `limit` larger than the remaining history
  returns everything that remains.
- A page **shorter than the requested `limit`** therefore signals that fewer
  entries remain than were asked for — the strongest end-of-history signal.
- `limit == 0` returns an empty page (zero items requested).
- `limit` is `u32`, so consumers may pass up to `u32::MAX` safely (see
  [overflow safety](#policy-overflow-safety)).

## Policy: end-of-history signalling

There are two equivalent end-of-history signals:

1. A returned page with **fewer than `limit` entries** (when `limit > 0`), or
2. An **empty page** (which is also the result of `offset >= len` or
   `limit == 0`).

Consumers are encouraged to iterate with a fixed page size and stop on the
first short page, which is exactly one extra call after the last full page
and needs no special-casing for empty histories.

## Policy: pagination metadata

`get_history_page_with_meta` returns the same page as `get_history_page`
wrapped in a `HistoryPage` struct:

```rust
pub struct HistoryPage {
    pub items: Vec<SLAResult>, // identical to get_history_page(offset, limit)
    pub total: u32,            // full history length at read time
    pub has_more: bool,        // end = min(saturating_add(offset, limit), total) < total
}
```

- `items` is **byte-for-byte identical** to `get_history_page(offset, limit)`
  for the same inputs; the legacy accessor remains unchanged for backward
  compatibility.
- `total` is the full history length, so consumers no longer need a separate
  `get_history` or `get_retention_limit` call to learn the total size.
- `has_more` is `true` exactly when the requested range ends before the end
  of history (`end < total`). A consumer can therefore iterate with
  `offset += items.len()` and stop when `has_more` is `false`.
- The same empty-page edge cases apply to `items`: `offset >= total` and
  `limit == 0` both produce an empty `items`. For `limit == 0` with
  `offset < total`, `has_more` is still `true` because the cursor has not
  advanced past `offset`.

`get_history_page_with_meta` is read-only, performs no storage writes, emits
no events, and never mutates history.

## Policy: overflow safety

`offset` and `limit` are `u32`, so the naive computation `offset + limit`
can overflow (e.g. `offset` near `u32::MAX`, or `limit = u32::MAX`). The
implementation therefore uses **saturating addition**:

```rust
let end = offset.saturating_add(limit).min(len);
```

Saturation guarantees that any page request is clamped to the real history
length and can never wrap into a wrong slice or panic. Consumers do not need
to pre-validate their offsets or limits for arithmetic safety.

## Policy: ordering & stability

- Entries are returned in **insertion order** (oldest first); the contract
  never reorders history.
- Pagination is **stable across calls**: identical `(offset, limit)` inputs
  against the same history produce identical slices.
- The accessor is **read-only**: it performs no storage writes, emits no
  events, and never mutates history. It is safe to call concurrently and
  repeatedly.

## Canonical source of truth

The contract implementation in `apexchainx_calculator/src/lib.rs` (and the
matching helper in `apexchainx_calculator/src/history.rs`) is the canonical
source of truth for this policy. If this document and the code ever disagree,
the code wins — update this document to match the code, and add a test in
`apexchainx_calculator/src/tests.rs` covering the divergent case.
