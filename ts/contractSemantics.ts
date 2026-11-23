/**
 * The contract's read semantics, as the contract itself reports them.
 *
 * # Why this file exists
 *
 * Every module in `ts/` used to carry its own private copy of a contract rule —
 * a page-size cap here, a status string there — with nothing tying any of them
 * to the Rust. Some of those copies were wrong: `historyPagination.ts` capped
 * pages at 50 while `history.rs` capped them at 200 (issue #409), so a backend
 * paging with `limit = 200` believed history ended after 50 entries.
 *
 * This module is the single place those facts enter TypeScript, and it does not
 * restate them. It re-exports `generated/contractConstants.ts`, which is
 * **written by executing the contract** in a Soroban `Env` — see
 * `apexchainx_calculator/src/ts_parity_fixtures.rs`. `MAX_PAGE_SIZE` below is
 * not "200 because a doc said so"; it is the number `get_history_page`
 * actually clamped to when it was last run.
 *
 * The indirection through this file (rather than importing the generated
 * module directly) keeps one stable import path for consumers while leaving
 * the generator free to change what it emits.
 *
 * # What is in contract, and what is not
 *
 * Only the surface listed in `docs/TS_PARITY_CONTRACT.md` is mirrored and
 * parity-checked: pagination, per-outage lookup, age-based pruning, the config
 * version hash, the result symbol vocabulary, and event topic names. Helpers
 * such as `governanceEvents.ts`, `configUpdateMeta.ts` and
 * `aggregateReadHelper.ts` are off-chain conveniences with no contract
 * counterpart and are explicitly out of scope — do not add them here.
 *
 * # Changing any of this
 *
 * Change the Rust, run `just ts-fixtures`, and commit what it regenerates. The
 * parity suite (`just ts-parity`) then tells you which hand-written helper
 * still disagrees. Editing the generated file by hand is pointless: the next
 * `cargo test` reverts it and CI fails on the diff.
 */

export {
  MAX_PAGE_SIZE,
  MAX_HISTORY_SIZE,
  MAX_RECALCS_PER_OUTAGE,
  DEFAULT_RETENTION_LIMIT,
  RESULT_SCHEMA_VERSION,
  RESULT_FIELD_COUNT,
  SYMBOLS,
  CANONICAL_SEVERITIES,
  EVENT_TOPICS,
  EVENT_VERSION,
  EVENT_VERSIONS,
} from "./generated/contractConstants";
