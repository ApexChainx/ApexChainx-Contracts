/**
 * SC-010 — Paginated history read helpers (#130)
 *
 * Off-chain mirror of the contract's `get_history_page` /
 * `get_history_page_with_meta` accessors
 * (`apexchainx_calculator/src/history.rs`).
 *
 * # Contract semantics mirrored here
 *
 * History is append-only and stored oldest-first, so pagination is stable:
 * identical `(offset, limit)` inputs against the same history always select the
 * same slice.
 *
 *   - `offset` is the 0-based index of the first entry to return. An `offset`
 *     at or past the end returns an **empty page** — the canonical
 *     end-of-history signal, not an error.
 *   - `limit` is clamped to `MAX_PAGE_SIZE`, so no single call can read the
 *     whole retained history. The effective page is
 *     `min(min(limit, MAX_PAGE_SIZE), total - offset)`.
 *   - `limit === 0` returns an empty page. It is **not** coerced upward.
 *   - `end = min(offset + limit, total)` is computed with saturating
 *     arithmetic on the contract side, so extreme `u32` inputs clamp rather
 *     than wrap. The JavaScript mirror gets this for free from `Math.min`.
 *   - `hasMore` is `end < total` for a non-zero limit — true exactly when the
 *     requested range stops before the end of history. A zero limit is a
 *     degenerate request: it returns an empty page and reports `hasMore:
 *     false`. Iterating with `offset += entries.length` therefore only
 *     advances for a non-zero limit.
 *
 * `MAX_PAGE_SIZE` is imported, never re-declared — see `contractSemantics.ts`.
 * This file previously hard-coded 50, which silently truncated every backend
 * page that asked for more; `ts/parity/readSemanticsParity.test.ts` now replays
 * contract-recorded `(offset, limit)` probes through `getHistoryPage` so that
 * class of drift cannot come back.
 *
 * Full policy: `docs/HISTORY_PAGINATION_POLICY.md`.
 */

import { MAX_PAGE_SIZE } from "./contractSemantics";

export interface HistoryEntry {
  id: string;
  outageId: string;
  severity: string;
  mttr: number;
  slaMetPct: number;
  recordedAt: number;
}

export interface HistoryPage {
  entries: HistoryEntry[];
  offset: number;
  total: number;
  hasMore: boolean;
}

/**
 * Returns the page of history entries the contract would return for
 * `(offset, limit)`.
 *
 * @param history - full append-only history array (oldest first)
 * @param offset  - 0-based start index; at or past the end yields an empty page
 * @param limit   - requested page size; clamped to `MAX_PAGE_SIZE`, `0` yields
 *                  an empty page that reports no more
 */
export function getHistoryPage(
  history: HistoryEntry[],
  offset: number,
  limit: number,
): HistoryPage {
  const total = history.length;

  // The contract's parameters are `u32`; negative or fractional inputs cannot
  // reach it, so they are normalised here rather than given their own
  // behaviour.
  const safeOffset = Math.max(0, Math.floor(offset));
  const safeLimit = Math.min(MAX_PAGE_SIZE, Math.max(0, Math.floor(limit)));

  // Mirrors `end = min(saturating_add(offset, limit), len)`.
  const end = Math.min(safeOffset + safeLimit, total);
  const entries = safeOffset < end ? history.slice(safeOffset, end) : [];

  return {
    entries,
    offset: safeOffset,
    total,
    // `limit > 0 && end < total`, not `end < total || offset + entries.length
    // < total`: the first differs exactly when `limit === 0`, where the
    // contract reports `false`.
    hasMore: limit > 0 && end < total,
  };
}
