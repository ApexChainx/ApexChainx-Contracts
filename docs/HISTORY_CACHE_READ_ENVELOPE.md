# History Cache Read Envelope

> **Issue:** [#664](https://github.com/ApexChainx/ApexChainx-Contracts/issues/664)
> **Applies to:** `HISTORY_LEN_KEY`, `HISTORY_KEY`, `get_full_audit_state`, `get_history_page`

## Overview

The contract maintains a cached history length (`HISTORY_LEN_KEY`) alongside
the full history vector (`HISTORY_KEY`). This document defines the **canonical
read cost envelope** — which reads use the cache and which require full vector
deserialization — and tracks the intentional gaps.

---

## Cache definition

| Key | Type | Written by | Semantics |
|---|---|---|---|
| `HISTORY_LEN_KEY` | `u32` | `calculate_sla`, `prune_history`, `prune_history_by_age` | Cached count of retained entries. Always equal to `HISTORY_KEY.len()` after any write. |
| `HISTORY_KEY` | `Vec<SLAResult>` | Same as above | Full ordered history vector. |

The invariant `HISTORY_LEN_KEY == HISTORY_KEY.len()` is asserted by the tests
in `history_invariant_tests.rs` (#669).

---

## Defined readers and their cost class

| Accessor | Uses cache | Cost class | Notes |
|---|---|---|---|
| `get_full_audit_state` | ✅ Yes | **Cheap** — reads `HISTORY_LEN_KEY` directly; does not deserialize the vector | Primary motivation for the cache (#463). |
| `get_history_page` | ❌ No | **Full deserialization** | Reads the entire `HISTORY_KEY` vector then slices. See known gap below. |
| `get_history_page_with_meta` | ❌ No | **Full deserialization** | Same as above; `total` field mirrors the length from the deserialized vector. |
| `get_history_by_outage` | ❌ No | **Full deserialization** | Must scan every entry for matching `outage_id`. |
| `get_latest_by_outage` | ❌ No | **Full deserialization** | Same scan. |
| `get_history` (legacy) | ❌ No | **Full deserialization** (bounded by `MAX_PAGE_SIZE`) | Legacy endpoint; bounded but still full-vector. |

---

## Known gap: pagination reads (#663)

`get_history_page(offset, limit)` materializes up to `MAX_HISTORY_SIZE` (1000)
entries to return a `MAX_PAGE_SIZE` (200) slice. The cached length is not used
to bound the read because the Soroban instance storage API does not support
sparse/partial vector reads — the full `Vec<SLAResult>` must be deserialized
before indexing.

**Accepted cost:** every `get_history_page` call pays O(N) deserialization
regardless of `offset` or `limit`. This is documented here as a known cost
rather than a hidden surprise. A future storage layout redesign (e.g. per-entry
storage keys or a linked-list layout) could make page reads O(page_size).

**Mitigation:** backends should cache pages locally and poll infrequently,
using `HISTORY_LEN_KEY` (via `get_full_audit_state`) as a cheap change
indicator before fetching new pages.

---

## Retiring the cache

If a storage layout redesign makes `HISTORY_LEN_KEY` redundant, retire it by:

1. Removing the write in `calculate_sla`, `prune_history`, and `prune_history_by_age`.
2. Updating `get_full_audit_state` to read the length from the vector or a new accessor.
3. Incrementing `STORAGE_VERSION` and adding a `migrate()` step to remove the key.
4. Updating this document and the invariant tests in `history_invariant_tests.rs`.