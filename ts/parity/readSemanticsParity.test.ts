/**
 * Cross-language parity: the `ts/` read helpers vs. the contract itself.
 *
 * # What makes this a parity test rather than a unit test
 *
 * Its expectations are not written here. Every one is read from
 * `ts/fixtures/contract-read-semantics.json`, which
 * `apexchainx_calculator/src/ts_parity_fixtures.rs` produces by **executing the
 * real contract in a Soroban `Env`** — the same code path an on-chain call
 * takes. This file replays the contract's recorded inputs through the
 * TypeScript helpers and asserts identical outputs. Nobody hand-maintains a
 * number in here, so nobody can hand-maintain it wrong.
 *
 * # How a drifting contract is caught
 *
 * The fixture is committed. `cargo test` rewrites it from live behaviour and CI
 * then runs `git diff --exit-code -- ts/fixtures ts/generated`:
 *
 *   - Change a read semantic in Rust and the regenerated fixture differs from
 *     the committed one → CI fails until it is committed.
 *   - Commit the new fixture without updating the TypeScript → this suite fails
 *     on the changed values.
 *
 * Neither order goes green, which is the point.
 *
 * # Scope
 *
 * Only the surface `docs/TS_PARITY_CONTRACT.md` declares in contract:
 * pagination, per-outage lookup, age-based pruning, the config version hash,
 * the result symbol vocabulary and event topic names. `governanceEvents.ts`,
 * `configUpdateMeta.ts` and `aggregateReadHelper.ts` are off-chain conveniences
 * with no contract counterpart and are deliberately not asserted here.
 *
 * Run with `just ts-parity` (or `npm run test:parity`).
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { test } from "node:test";

import {
  CANONICAL_SEVERITIES,
  EVENT_TOPICS,
  EVENT_VERSION,
  MAX_HISTORY_SIZE,
  MAX_PAGE_SIZE,
  RESULT_FIELD_COUNT,
  RESULT_SCHEMA_VERSION,
  SYMBOLS,
} from "../contractSemantics";
import { configVersionHash, type SlaConfig } from "../configVersionHash";
import { getHistoryByOutage, getLatestByOutage } from "../historyByOutage";
import { getHistoryPage, type HistoryEntry } from "../historyPagination";
import { pruneByMinAge } from "../historyPruneByAge";

interface Fixture {
  constants: Record<string, number>;
  symbols: Record<string, string>;
  eventTopics: Record<string, string>;
  eventVersion: string;
  configSnapshot: {
    versionHash: string;
    entries: {
      severity: string;
      thresholdMinutes: number;
      penaltyPerMinute: string;
      rewardBase: string;
    }[];
  };
  history: {
    outageId: string;
    mttrMinutes: number;
    thresholdMinutes: number;
    status: string;
    paymentType: string;
    rating: string;
    amount: string;
    recordedAt: string;
  }[];
  paginationCases: {
    offset: number;
    limit: number;
    pageLength: number;
    total: number;
    hasMore: boolean;
    firstMttr: number | null;
    lastMttr: number | null;
  }[];
  byOutageCases: {
    outageId: string;
    matchCount: number;
    latestMttr: number | null;
  }[];
  pruneByAgeCases: { now: string; minAgeSeconds: string; keptCount: number }[];
}

const FIXTURE: Fixture = JSON.parse(
  readFileSync(
    resolve(__dirname, "..", "fixtures", "contract-read-semantics.json"),
    "utf8",
  ),
) as Fixture;

/**
 * The contract's recorded history, projected onto the shape the `ts/` helpers
 * accept.
 *
 * The helpers were written against a looser `HistoryEntry` than the contract's
 * `SLAResult` (an `id`, an `slaMetPct` the contract has no concept of). Rather
 * than reshape a published interface, the fields the helpers actually route on
 * — `outageId` and `recordedAt` — are filled from the contract's values and the
 * rest carry the recorded result so a mis-selected entry is visible in the
 * assertion output.
 */
const HISTORY: HistoryEntry[] = FIXTURE.history.map((entry, index) => ({
  id: `e${index}`,
  outageId: entry.outageId,
  severity: entry.status,
  mttr: entry.mttrMinutes,
  slaMetPct: entry.thresholdMinutes,
  recordedAt: Number(entry.recordedAt),
}));

// ─── Constants ──────────────────────────────────────────────────────────────

test("mirrored constants match the values the contract reported", () => {
  assert.equal(MAX_PAGE_SIZE, FIXTURE.constants.maxPageSize);
  assert.equal(MAX_HISTORY_SIZE, FIXTURE.constants.maxHistorySize);
  assert.equal(RESULT_SCHEMA_VERSION, FIXTURE.constants.resultSchemaVersion);
  assert.equal(RESULT_FIELD_COUNT, FIXTURE.constants.resultFieldCount);
});

test("the result symbol vocabulary matches the contract's schema", () => {
  assert.equal(SYMBOLS.statusMet, FIXTURE.symbols["status.met"]);
  assert.equal(SYMBOLS.statusViolated, FIXTURE.symbols["status.violated"]);
  assert.equal(SYMBOLS.paymentReward, FIXTURE.symbols["payment.reward"]);
  assert.equal(SYMBOLS.paymentPenalty, FIXTURE.symbols["payment.penalty"]);
  assert.equal(SYMBOLS.ratingTop, FIXTURE.symbols["rating.top"]);
  assert.equal(SYMBOLS.ratingExcellent, FIXTURE.symbols["rating.excellent"]);
  assert.equal(SYMBOLS.ratingGood, FIXTURE.symbols["rating.good"]);
  assert.equal(SYMBOLS.ratingPoor, FIXTURE.symbols["rating.poor"]);

  assert.deepEqual(
    [...CANONICAL_SEVERITIES],
    [
      FIXTURE.symbols["severity.critical"],
      FIXTURE.symbols["severity.high"],
      FIXTURE.symbols["severity.medium"],
      FIXTURE.symbols["severity.low"],
    ],
    "canonical severity order is load-bearing: it fixes telemetry lane indices " +
      "and the order the config version hash walks",
  );
});

test("event topic names and the schema version match the contract", () => {
  assert.deepEqual({ ...EVENT_TOPICS }, FIXTURE.eventTopics);
  assert.equal(EVENT_VERSION, FIXTURE.eventVersion);
});

test("every recorded status, payment type and rating is a known symbol", () => {
  // Typed as `Set<string>`: the generated constants are `as const`, so an
  // inferred set would be typed to those literals and reject the fixture's
  // `string` values at compile time — which would be checking the generated
  // file against itself rather than against what the contract recorded.
  const statuses = new Set<string>([SYMBOLS.statusMet, SYMBOLS.statusViolated]);
  const payments = new Set<string>([SYMBOLS.paymentReward, SYMBOLS.paymentPenalty]);
  const ratings = new Set<string>([
    SYMBOLS.ratingTop,
    SYMBOLS.ratingExcellent,
    SYMBOLS.ratingGood,
    SYMBOLS.ratingPoor,
  ]);

  for (const entry of FIXTURE.history) {
    assert.ok(statuses.has(entry.status), `unknown status ${entry.status}`);
    assert.ok(payments.has(entry.paymentType), `unknown payment ${entry.paymentType}`);
    assert.ok(ratings.has(entry.rating), `unknown rating ${entry.rating}`);

    // Sign discipline, as the contract guarantees it: a met result settles a
    // strictly positive reward, a violation a strictly negative penalty.
    const amount = BigInt(entry.amount);
    if (entry.status === SYMBOLS.statusMet) {
      assert.equal(entry.paymentType, SYMBOLS.paymentReward);
      assert.ok(amount > 0n, `met entry ${entry.outageId} settled ${amount}`);
    } else {
      assert.equal(entry.paymentType, SYMBOLS.paymentPenalty);
      assert.equal(entry.rating, SYMBOLS.ratingPoor);
      assert.ok(amount < 0n, `violated entry ${entry.outageId} settled ${amount}`);
    }
  }
});

// ─── Pagination ─────────────────────────────────────────────────────────────

test("getHistoryPage reproduces every contract-recorded page", () => {
  // The fixture pages over the contract's full history; the TypeScript mirror
  // is fed an array of the same length so offsets line up. Only the entries the
  // fixture spelled out carry real values.
  const total = FIXTURE.constants.historyEntries;
  const padded: HistoryEntry[] = Array.from({ length: total }, (_, index) =>
    index < HISTORY.length
      ? HISTORY[index]
      : { id: `e${index}`, outageId: `o${String(index).padStart(3, "0")}`,
          severity: "?", mttr: -1, slaMetPct: -1, recordedAt: -1 },
  );

  for (const expected of FIXTURE.paginationCases) {
    const page = getHistoryPage(padded, expected.offset, expected.limit);
    const label = `offset=${expected.offset} limit=${expected.limit}`;

    assert.equal(page.total, expected.total, `total for ${label}`);
    assert.equal(page.entries.length, expected.pageLength, `page length for ${label}`);
    assert.equal(page.hasMore, expected.hasMore, `hasMore for ${label}`);

    // Where the fixture recorded real MTTRs, check the page selected the same
    // slice — a page of the right length starting in the wrong place would
    // otherwise pass.
    if (expected.firstMttr !== null && expected.offset < HISTORY.length) {
      assert.equal(page.entries[0].mttr, expected.firstMttr, `first entry for ${label}`);
    }
    if (
      expected.lastMttr !== null &&
      expected.offset + expected.pageLength <= HISTORY.length
    ) {
      assert.equal(
        page.entries[page.entries.length - 1].mttr,
        expected.lastMttr,
        `last entry for ${label}`,
      );
    }
  }
});

test("a limit above MAX_PAGE_SIZE is clamped, not honoured", () => {
  const clamped = FIXTURE.paginationCases.filter(
    (c) => c.limit > MAX_PAGE_SIZE && c.offset === 0,
  );
  assert.ok(clamped.length > 0, "fixture must probe an over-large limit");
  for (const expected of clamped) {
    assert.equal(
      expected.pageLength,
      MAX_PAGE_SIZE,
      "the contract clamps to MAX_PAGE_SIZE; if this fails the fixture is stale",
    );
  }
});

test("limit 0 yields an empty page and reports no more", () => {
  const zeroLimit = FIXTURE.paginationCases.filter(
    (c) => c.limit === 0 && c.offset < c.total,
  );
  assert.ok(zeroLimit.length > 0, "fixture must probe limit === 0");
  for (const expected of zeroLimit) {
    // The contract treats limit == 0 as a degenerate request: an empty page
    // that never reports more (has_more is false). A `offset + entries.length
    // < total` mirror and a `Math.max(1, limit)` mirror both get this wrong.
    assert.equal(expected.pageLength, 0);
    assert.equal(expected.hasMore, false);
  }
});

// ─── Per-outage lookup ──────────────────────────────────────────────────────

test("getHistoryByOutage and getLatestByOutage match contract lookups", () => {
  for (const expected of FIXTURE.byOutageCases) {
    // The fixture probes ids across the full history; only ids inside the
    // detailed slice can be reproduced from it.
    const known = HISTORY.some((e) => e.outageId === expected.outageId);
    const result = getHistoryByOutage(HISTORY, expected.outageId);
    const latest = getLatestByOutage(HISTORY, expected.outageId);

    if (!known) {
      assert.equal(result.count, 0, `${expected.outageId} should not match`);
      assert.equal(latest, null);
      continue;
    }

    assert.equal(result.count, expected.matchCount, `match count for ${expected.outageId}`);
    assert.notEqual(latest, null);
    assert.equal(latest!.mttr, expected.latestMttr, `latest for ${expected.outageId}`);
  }
});

// ─── Age-based pruning ──────────────────────────────────────────────────────

test("pruneByMinAge reproduces the contract's retained-set sizes", () => {
  // Rebuild the full recorded timeline: the fixture spells out the first N
  // entries, and the contract recorded one every `step` seconds from the first.
  const total = FIXTURE.constants.historyEntries;
  const first = BigInt(FIXTURE.history[0].recordedAt);
  const step = BigInt(FIXTURE.history[1].recordedAt) - first;
  const timeline: HistoryEntry[] = Array.from({ length: total }, (_, index) => ({
    id: `e${index}`,
    outageId: `o${String(index).padStart(3, "0")}`,
    severity: "?",
    mttr: 0,
    slaMetPct: 0,
    recordedAt: Number(first + BigInt(index) * step),
  }));

  for (const expected of FIXTURE.pruneByAgeCases) {
    const result = pruneByMinAge(
      timeline,
      BigInt(expected.now),
      BigInt(expected.minAgeSeconds),
    );
    assert.equal(
      result.kept.length,
      expected.keptCount,
      `kept count for minAgeSeconds=${expected.minAgeSeconds}`,
    );
    assert.equal(result.pruned, total - expected.keptCount);
  }
});

test("min age just below the ledger timestamp keeps the full history", () => {
  // The contract rejects min_age >= now as InvalidInput, so the largest probed
  // min-age (strictly below `now`) must still retain every entry.
  const cases = FIXTURE.pruneByAgeCases;
  assert.ok(cases.length > 0, "fixture must probe prune-by-age cases");
  const currentLedger = BigInt(cases[0].now);
  const largestValid = cases.reduce((a, b) =>
    BigInt(a.minAgeSeconds) > BigInt(b.minAgeSeconds) ? a : b,
  );
  assert.ok(
    BigInt(largestValid.minAgeSeconds) < currentLedger,
    "fixture must probe a min age strictly below the ledger timestamp",
  );
  assert.equal(largestValid.keptCount, FIXTURE.constants.historyEntries);
});

// ─── Config version hash ────────────────────────────────────────────────────

test("configVersionHash reproduces the contract's hash exactly", () => {
  const configs: SlaConfig[] = FIXTURE.configSnapshot.entries.map((entry) => ({
    severity: entry.severity,
    thresholdMinutes: entry.thresholdMinutes,
    penaltyPerMinute: BigInt(entry.penaltyPerMinute),
    rewardBase: BigInt(entry.rewardBase),
  }));

  assert.equal(
    configVersionHash({ configs }).toString(),
    FIXTURE.configSnapshot.versionHash,
    "the TypeScript hash must equal get_config_version_hash for the same snapshot",
  );
});

test("configVersionHash is order-independent but field-sensitive", () => {
  const configs: SlaConfig[] = FIXTURE.configSnapshot.entries.map((entry) => ({
    severity: entry.severity,
    thresholdMinutes: entry.thresholdMinutes,
    penaltyPerMinute: BigInt(entry.penaltyPerMinute),
    rewardBase: BigInt(entry.rewardBase),
  }));

  // The contract always walks canonical order, so the input array's order
  // must not matter.
  assert.equal(
    configVersionHash({ configs: [...configs].reverse() }),
    configVersionHash({ configs }),
    "hash must not depend on the order configs are supplied in",
  );

  // Every field must feed the hash — a change detector that misses a field is
  // worse than none, because consumers trust it.
  for (const index of configs.keys()) {
    for (const mutate of [
      (c: SlaConfig): SlaConfig => ({ ...c, thresholdMinutes: c.thresholdMinutes + 1 }),
      (c: SlaConfig): SlaConfig => ({ ...c, penaltyPerMinute: c.penaltyPerMinute + 1n }),
      (c: SlaConfig): SlaConfig => ({ ...c, rewardBase: c.rewardBase + 1n }),
    ]) {
      const mutated = configs.map((c, i) => (i === index ? mutate(c) : c));
      assert.notEqual(
        configVersionHash({ configs: mutated }),
        configVersionHash({ configs }),
        `changing a field of ${configs[index].severity} did not change the hash`,
      );
    }
  }
});

test("configVersionHash refuses a partial snapshot", () => {
  const configs: SlaConfig[] = FIXTURE.configSnapshot.entries
    .slice(1)
    .map((entry) => ({
      severity: entry.severity,
      thresholdMinutes: entry.thresholdMinutes,
      penaltyPerMinute: BigInt(entry.penaltyPerMinute),
      rewardBase: BigInt(entry.rewardBase),
    }));

  assert.throws(
    () => configVersionHash({ configs }),
    /missing canonical severity/,
    "hashing three of four configs would produce a plausible wrong answer",
  );
});
