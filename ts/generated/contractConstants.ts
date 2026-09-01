// GENERATED FILE - do not edit by hand.
//
// Every value below was read out of the running contract by
// `apexchainx_calculator/src/ts_parity_fixtures.rs`. Regenerate with
// `just ts-fixtures`. Editing this file by hand will be silently
// reverted by the next `cargo test`, and CI fails on the diff.
//
// See docs/TS_PARITY_CONTRACT.md for the surface this covers.

/** Upper bound the contract clamps a page `limit` to (`history::MAX_PAGE_SIZE`). */
export const MAX_PAGE_SIZE = 200;

/** Hard cap on retained history entries. */
export const MAX_HISTORY_SIZE = 1000;

/** Recalculations allowed per outage id under one config version. */
export const MAX_RECALCS_PER_OUTAGE = 16;

/** Retention limit applied when an admin has not set one. */
export const DEFAULT_RETENTION_LIMIT = 1000;

/** Numeric `SLAResult` schema version. */
export const RESULT_SCHEMA_VERSION = 1;

/** Number of named fields in `SLAResult` at this schema version. */
export const RESULT_FIELD_COUNT = 9;

/** Status, payment-type, rating and severity symbols the contract emits. */
export const SYMBOLS = {
  statusMet: "met",
  statusViolated: "viol",
  paymentReward: "rew",
  paymentPenalty: "pen",
  ratingTop: "top",
  ratingExcellent: "excel",
  ratingGood: "good",
  ratingPoor: "poor",
  severityCritical: "critical",
  severityHigh: "high",
  severityMedium: "medium",
  severityLow: "low",
} as const;

/** Canonical severities, in the order the contract snapshots them. */
export const CANONICAL_SEVERITIES = [
  "critical",
  "high",
  "medium",
  "low",
] as const;

/** Event topic symbols, keyed by role. */
export const EVENT_TOPICS = {
  slaCalculated: "sla_calc",
  settlementIntent: "set_int",
  duplicateInput: "dup_input",
  configUpdated: "cfg_upd",
  configRemoved: "cfg_rem",
  severityAdded: "sev_add",
  severityUpdated: "sev_upd",
  pruned: "pruned",
  prunedByAge: "pruned_a",
  retentionLimitSet: "ret_lim",
} as const;

/** Schema version carried in topic position 2 of every event. */
export const EVENT_VERSION = "v1";
