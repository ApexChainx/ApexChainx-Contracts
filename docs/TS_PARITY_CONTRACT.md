# TypeScript Parity Contract

> **Status:** Active
> **Audience:** Backend consumers and contract contributors
> **Last updated:** 2026-08-29

## Table of Contents

- [Overview](#overview)
- [The source of truth](#the-source-of-truth)
- [The in-contract surface](#the-in-contract-surface)
- [Explicitly out of contract](#explicitly-out-of-contract)
- [How drift is caught](#how-drift-is-caught)
- [Changing a read semantic](#changing-a-read-semantic)
- [Drift this replaced](#drift-this-replaced)
- [Running the checks](#running-the-checks)

## Overview

`ts/` contains off-chain helpers that reproduce parts of the contract's read
behaviour in TypeScript, so backend consumers can page, filter and fingerprint
without a round-trip for every decision.

Reimplementing contract behaviour in a second language is a copy of the truth,
and copies drift. This document defines which parts of `ts/` are held to the
contract's behaviour, which are not, and the mechanism that makes a silent
divergence impossible.

## The source of truth

**The running contract.** Not a doc, not a schema file, and not the TypeScript.

`apexchainx_calculator/src/ts_parity_fixtures.rs` is a test that executes the
real `SLACalculatorContract` inside a Soroban `Env` — the same code path an
on-chain call takes — and writes down what it actually returned:

| Artefact | Contents | Consumed by |
|---|---|---|
| `ts/generated/contractConstants.ts` | Constants, symbol vocabulary, event topic names | The `ts/` helpers, via `ts/contractSemantics.ts` |
| `ts/fixtures/contract-read-semantics.json` | Recorded inputs and outputs: 250 SLA results, 14 pagination probes, per-outage lookups, age-prune probes, a config version hash | `ts/parity/readSemanticsParity.test.ts` |

Both are **generated and committed**. Nothing in them is hand-transcribed, with
one deliberate exception: the symbol *table* pairs a literal (`"viol"`) with the
contract constant it must equal, and the pairing is asserted against
`get_result_schema` before it is written — so renaming a status symbol or an
event topic fails the generator rather than shipping a stale string.

The generated constants are emitted as a plain `.ts` module rather than read
from JSON at runtime, so the helpers need no filesystem access and stay as
portable as the hand-written copies they replace — while no longer being
hand-written.

## The in-contract surface

Cross-language parity has a real cost in fixtures and serialisation, so the
surface is bounded. These are held to the contract's behaviour and are covered
by the parity suite:

| TypeScript | Contract counterpart | What is asserted |
|---|---|---|
| `contractSemantics.ts` | `history::MAX_PAGE_SIZE`, `MAX_HISTORY_SIZE`, `get_result_schema`, `EVENT_*` | Constants, symbol vocabulary, canonical severity order, event topic names, event schema version |
| `historyPagination.ts` | `get_history_page`, `get_history_page_with_meta` | Page length, selected slice, `total`, `hasMore`, the `MAX_PAGE_SIZE` clamp, `limit == 0`, out-of-range offsets, saturating `offset + limit` |
| `historyByOutage.ts` | `get_history_by_outage`, `get_latest_by_outage` | Match count, insertion-order semantics, "latest" = last match (not largest timestamp), zero-match behaviour |
| `historyPruneByAge.ts` | `prune_history_by_age` | Retained-set size for a given `(now, min_age_seconds)`, and the saturating cutoff |
| `configVersionHash.ts` | `get_config_version_hash` | Exact hash equality against a contract-recorded value, order-independence, sensitivity to every config field |

Result-payload semantics — the `met`/`viol` split, `rew`/`pen`, the four
ratings, and the guarantee that a reward is strictly positive and a penalty
strictly negative — are asserted over the recorded history in the same suite.

## Explicitly out of contract

These files have **no contract counterpart**. They are off-chain conveniences
that model backend-side concepts, and the parity suite deliberately does not
assert them. Do not add them to `contractSemantics.ts` or the fixture.

| File | Why it is out of contract |
|---|---|
| `governanceEvents.ts` | An in-memory event log with its own kind vocabulary (`admin_proposed`, `governance_locked`, …). The contract emits `adm_prop` / `adm_acc` / `adm_ren` / `op_prop` / `op_acc` / `op_can` / `cfg_frz` / `cfg_unfrz` topics with different names and no `metadata` field. This models a backend's own aggregation, not the chain's events. If it is ever meant to mirror the contract, that is a separate change — and it joins the table above. |
| `configUpdateMeta.ts` | Tracks an actor and an update counter that the contract does not store. Since storage v4 (#671) the contract does persist an actor with the most recent config update (`get_last_config_update().actor`), but it records a *single* last-update metadata slot — not a per-update history with a monotonic counter — and it keys the actor off the authorized admin, not an arbitrary string. The in-memory mirror remains an off-chain convenience. |
| `aggregateReadHelper.ts` | Bundles several reads into one backend-side snapshot. Its `SeverityConfig` uses `threshold_seconds` / `reward_bps` / `penalty_bps`; the contract's `SLAConfig` uses `threshold_minutes` / `penalty_per_minute` / `reward_base`. Different units, different fields, by design. |
| `upgradeGuardTests.ts` | A standalone scratch harness, not a mirror. |

If one of these grows a real contract counterpart, move its row into the
in-contract table, add fixtures for it, and assert it. The point of listing
them here is that "not checked" is a recorded decision rather than an oversight.

## How drift is caught

The fixture and the generated constants are committed, and `cargo test`
regenerates them from live contract behaviour. CI then runs:

```sh
git diff --exit-code -- ts/fixtures ts/generated
```

That produces a two-sided trap:

- **Change a read semantic in Rust and forget the TypeScript.** The regenerated
  artefacts differ from the committed ones and CI fails on the diff.
- **Commit the regenerated artefacts but leave the TypeScript alone.** The
  parity suite fails on the changed values.

Neither order goes green. A contract change that misses the mirrors cannot land.

## Changing a read semantic

1. Change the Rust.
2. Run `just ts-fixtures` (or `cargo test -p apexchainx_calculator ts_parity_fixtures`).
3. Review the regenerated diff — it is the behavioural change, stated in
   contract-produced values. This is the review artefact; read it.
4. Run `just ts-parity`. It tells you which hand-written helper still disagrees.
5. Update the helper, re-run, commit the Rust, the artefacts and the helper
   together.
6. Update the prose policy doc if one covers the behaviour (for pagination,
   `docs/HISTORY_PAGINATION_POLICY.md`).

Editing `ts/generated/contractConstants.ts` or the fixture by hand is pointless:
the next `cargo test` reverts it and CI fails on the diff.

## Drift this replaced

The mechanism above was not hypothetical. When it was first run against the
existing helpers it found:

- **`historyPagination.ts` capped pages at 50** while `history.rs` capped them
  at 200 (issue #409). A backend paging with `limit = 200` received 50 entries
  and, because the mirror also computed `hasMore` from the returned length,
  could conclude history had ended. The doc was wrong in the other direction —
  it stated the limit was *not* clamped at all, describing behaviour from before
  #409 — so all three descriptions of one policy disagreed.
- **`historyPagination.ts` coerced `limit = 0` up to 1**, returning an entry
  where the contract returns an empty page, and reported `hasMore: false` where
  the contract reports `true`.
- **`configVersionHash.ts` computed an entirely different hash** — djb2 over a
  canonical JSON serialisation of a snapshot whose fields (`penaltyBps`,
  `rewardBps`) do not exist on the contract. A backend comparing it against
  `get_config_version_hash` would have seen a mismatch on every call and
  concluded the config had drifted when nothing had.

The per-file `require.main === module` self-tests that used to sit at the bottom
of these helpers have been removed. They asserted the helper against itself,
which is what let all three defects above survive; the parity suite asserts it
against the contract instead.

## Running the checks

```sh
just ts-fixtures   # regenerate the contract-derived artefacts
just ts-parity     # run the parity suite against them
just ts-check      # both, plus the generated-artefact freshness check CI runs
```

## Manifest and type coverage gate (#678)

`npm run check:parity-coverage` reconciles the public method manifest with
contract entrypoints and `PARITY_METHOD_COVERAGE` in `ts_parity_fixtures.rs`.
The Rust fixture test also compares that inventory with the runtime descriptor.
Every method needs exactly one fixture target or an explicit skip explanation.
The checker verifies fixture targets exist, are nonempty, and invoke the method.
It reports covered/skipped counts; a skipped method is not parity-tested.

The parity suite recursively scans Rust modules and compares every discovered `#[contracttype]` declaration
against `ts/parity/contract-types.json`. Adding or changing fields/types/variants
requires fixture review and an explicit inventory update:
`npx tsx scripts/check-parity-coverage.ts --write-types`. Review the resulting
diff alongside the relevant fixture or skip. This inventory is a review gate,
not automatic generation of semantic tests for arbitrary new methods.

`npm run test:parity`, `just ts-check`, and CI parity include
these gates. No automatically generated default skip is permitted.
Commented calls and calls in unrelated tests do not satisfy fixture coverage:
the invocation must appear in `generate_ts_parity_fixtures` itself.
