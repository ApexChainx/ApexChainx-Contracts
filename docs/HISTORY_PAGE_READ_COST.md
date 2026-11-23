# History Page Read Cost

> **Issue:** [#663](https://github.com/ApexChainx/ApexChainx-Contracts/issues/663)
> **Applies to:** `get_history_page`, `get_history_page_with_meta`, `HISTORY_LEN_KEY`

## Problem statement

`get_history_page(offset, limit)` and `get_history_page_with_meta` read the
**full** `HISTORY_KEY` vector and then slice it. Even a page request for 1
entry at offset 0 deserializes all retained history (up to `MAX_HISTORY_SIZE`
= 1000 `SLAResult` entries). The cached `HISTORY_LEN_KEY` exists (#463) but
is not used to reduce this cost.

---

## Why the full vector is read

The Soroban instance storage API stores `HISTORY_KEY` as a single serialized
`Vec<SLAResult>`. There is no sparse/partial read primitive — the entire blob
must be deserialized before any index operation. The cached length cannot
substitute for the data read; it can only substitute for the *count*.

This is a **storage layout constraint**, not an oversight in the pagination
implementation.

---

## Current cost envelope

| Call | Deserialization cost | Notes |
|---|---|---|
| `get_history_page(0, 1)` | O(N) — full vector | N = retained entries (up to 1000) |
| `get_history_page(900, 200)` | O(N) — full vector | Same cost regardless of offset |
| `get_history_page_with_meta(0, 200)` | O(N) — full vector | `total` field derived from deserialized length |
| `get_full_audit_state` (length only) | O(1) — reads `HISTORY_LEN_KEY` | No vector deserialization |

---

## Mitigation guidance for backends

Because full deserialization is unavoidable with the current layout, backends
should:

1. **Use `get_full_audit_state` as a cheap change indicator.** It reads only
   `HISTORY_LEN_KEY` (O(1)). Poll it frequently; fetch pages only when the
   length has changed.

2. **Cache pages locally.** Once a page is fetched, it is stable: entries
   are append-only (pruning is the only removal path, and pruning is an
   explicit admin action). A cached page is valid until either the length
   increases or a `pruned` / `pruned_a` event is observed.

3. **Batch page fetches.** If a full history sync is needed, fetch pages
   sequentially from `offset=0` to `has_more=false`. Each call pays O(N)
   deserialization, so fetching all P pages costs O(P × N) — prefer a single
   initial sync over repeated small polls.

---

## Future improvement path

A per-entry storage layout (e.g. one key per `SLAResult`, indexed by sequence
number) would allow O(page_size) reads. Implementing this requires:

1. A new storage key scheme (e.g. `HIST_{seq}`) and a sequence counter.
2. A migration that rewrites the existing `HISTORY_KEY` vector into per-entry
   keys and increments `STORAGE_VERSION`.
3. Updated pagination logic to read only the required key range.
4. Regression tests comparing per-entry and full-vector read costs.

Until that migration is implemented, the full-vector cost is **accepted and
documented** here rather than silently surprising consumers.

---

## Accepted cost regression baseline

| Scenario | Expected deserialization entries |
|---|---|
| Empty history | 0 |
| 100 entries, page size 10 | 100 |
| 1000 entries (max), page size 200 | 1000 |