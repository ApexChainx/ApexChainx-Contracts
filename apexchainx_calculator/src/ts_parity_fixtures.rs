//! Generates the contract-derived fixtures that the TypeScript helpers in
//! `ts/` are parity-checked against.
//!
//! # The problem this closes
//!
//! `ts/` contains hand-written mirrors of contract read semantics — pagination,
//! the config version hash, per-outage lookup, age-based pruning. Nothing
//! generated them and nothing compared them to the contract, so a policy change
//! in Rust could land while the TypeScript kept answering with the old rules.
//! The mirrors had in fact already drifted: `ts/historyPagination.ts` capped
//! pages at 50 where the contract caps at
//! [`crate::history::MAX_PAGE_SIZE`] (200), and coerced `limit == 0` up to 1
//! where the contract returns an empty page.
//!
//! # Where the source of truth lives
//!
//! **The running contract.** This module executes the real
//! `SLACalculatorContract` inside a Soroban `Env` — the same code path an
//! on-chain call takes — and records what it actually returned into
//! `ts/fixtures/contract-read-semantics.json`. The TypeScript parity suite
//! (`ts/parity/readSemanticsParity.test.ts`) replays those recorded inputs
//! through the TypeScript helpers and asserts identical outputs.
//!
//! Nothing here is hand-transcribed from a doc: every number in the fixture is
//! a value the contract produced. The one exception is the symbol *table*,
//! where a literal (`"viol"`) is paired with the contract constant it must
//! equal and the pairing is asserted before it is written — so renaming a
//! status symbol or an event topic fails this test rather than silently
//! shipping a stale string to the backend.
//!
//! # How drift is caught
//!
//! The fixture is committed. `cargo test` rewrites it from live contract
//! behaviour, and CI then runs `git diff --exit-code -- ts/fixtures`:
//!
//! * Change a read semantic in Rust and the regenerated fixture differs from
//!   the committed one → CI fails until the fixture is committed.
//! * Commit the new fixture without updating the TypeScript → the parity suite
//!   fails on the changed values.
//!
//! A contract change that misses the TypeScript mirrors therefore cannot go
//! green in either order. See `docs/TS_PARITY_CONTRACT.md` for the surface this
//! covers and the helpers that are explicitly out of contract.

#![cfg(test)]

extern crate std;

use std::format;
use std::fs;
use std::path::PathBuf;
use std::string::{String, ToString};
use std::vec::Vec as StdVec;

use soroban_sdk::testutils::{Address as _, EnvTestConfig, Ledger as _};
use soroban_sdk::{symbol_short, Address, Env, Symbol};

use crate::{SLACalculatorContract, SLACalculatorContractClient};

/// Number of history entries recorded before the pagination cases are
/// captured. Deliberately above [`crate::history::MAX_PAGE_SIZE`] so the
/// clamp is exercised by real data rather than asserted in the abstract.
const HISTORY_ENTRIES: u32 = 250;

/// First ledger timestamp used when recording history; each subsequent entry
/// advances by [`TIMESTAMP_STEP`] so age-based pruning has a real spread.
const FIRST_TIMESTAMP: u64 = 1_700_000_000;
/// Seconds between consecutive recorded history entries.
const TIMESTAMP_STEP: u64 = 60;

/// How many history entries are recorded field-by-field in the fixture.
///
/// The pagination cases run against all [`HISTORY_ENTRIES`], but spelling out
/// every entry would make the committed file unreviewable. The severity cycle
/// has length 4 and the MTTR cycle length 7, which are coprime, so the first
/// 28 entries contain every (severity, MTTR) pair exactly once — full coverage
/// of the outcome mapping in a diff a human can read.
const DETAILED_HISTORY_ENTRIES: u32 = 28;

/// Every symbol the contract can place in a read response, paired with the
/// literal the TypeScript side uses for it.
///
/// The pairing is asserted, not assumed: see [`assert_symbol_table`].
const SYMBOL_TABLE: &[(&str, &str)] = &[
    ("status.met", "met"),
    ("status.violated", "viol"),
    ("payment.reward", "rew"),
    ("payment.penalty", "pen"),
    ("rating.top", "top"),
    ("rating.excellent", "excel"),
    ("rating.good", "good"),
    ("rating.poor", "poor"),
    ("severity.critical", "critical"),
    ("severity.high", "high"),
    ("severity.medium", "medium"),
    ("severity.low", "low"),
];

/// Event topics that carry read-relevant payloads to backend indexers, paired
/// with the contract constant each must equal.
fn event_topic_table() -> StdVec<(&'static str, &'static str, Symbol)> {
    std::vec![
        ("slaCalculated", "sla_calc", crate::EVENT_SLA_CALC),
        ("settlementIntent", "set_int", crate::EVENT_SETTLE_INTENT),
        ("duplicateInput", "dup_input", crate::EVENT_DUP_INPUT),
        ("configUpdated", "cfg_upd", crate::EVENT_CONFIG_UPD),
        ("configRemoved", "cfg_rem", crate::EVENT_CONFIG_REM),
        ("severityAdded", "sev_add", crate::EVENT_SEV_ADD),
        ("severityUpdated", "sev_upd", crate::EVENT_SEV_UPD),
        ("pruned", "pruned", crate::EVENT_PRUNED),
        ("prunedByAge", "pruned_a", crate::EVENT_PRUNED_AGE),
        ("retentionLimitSet", "ret_lim", crate::EVENT_RET_LIM),
    ]
}

/// Resolves a symbol returned by the contract back to its fixture literal.
///
/// Panics on an unrecognised symbol. That is deliberate: a new status, rating
/// or payment symbol must be added to [`SYMBOL_TABLE`] — and therefore to the
/// fixture and the TypeScript mirror — before it can reach a backend.
fn symbol_literal(env: &Env, value: &Symbol) -> String {
    for (_, literal) in SYMBOL_TABLE {
        if *value == Symbol::new(env, literal) {
            return (*literal).to_string();
        }
    }
    panic!(
        "contract returned a symbol that is not in SYMBOL_TABLE; add it here, to \
         ts/contractSemantics.ts, and to the parity suite before shipping it"
    )
}

/// Asserts every literal in [`SYMBOL_TABLE`] still equals the symbol the
/// contract reports for that role, reading the roles from the contract's own
/// `get_result_schema` rather than from this file's expectations.
fn assert_symbol_table(env: &Env, client: &SLACalculatorContractClient<'_>) {
    let schema = client.get_result_schema();
    assert_eq!(schema.status_met, Symbol::new(env, "met"));
    assert_eq!(schema.status_violated, Symbol::new(env, "viol"));
    assert_eq!(schema.payment_reward, Symbol::new(env, "rew"));
    assert_eq!(schema.payment_penalty, Symbol::new(env, "pen"));
    assert_eq!(schema.rating_exceptional, Symbol::new(env, "top"));
    assert_eq!(schema.rating_excellent, Symbol::new(env, "excel"));
    assert_eq!(schema.rating_good, Symbol::new(env, "good"));
    assert_eq!(schema.rating_poor, Symbol::new(env, "poor"));

    let severities = client.get_config_snapshot();
    let expected = ["critical", "high", "medium", "low"];
    assert_eq!(severities.entries.len(), expected.len() as u32);
    for (index, name) in expected.iter().enumerate() {
        let entry = severities.entries.get(index as u32).expect("canonical entry");
        assert_eq!(
            entry.severity,
            Symbol::new(env, name),
            "canonical severity order changed at index {}",
            index
        );
    }

    for (role, literal, constant) in event_topic_table() {
        assert_eq!(
            Symbol::new(env, literal),
            constant,
            "event topic literal for {} no longer matches the contract constant",
            role
        );
    }
}

// ─── Minimal JSON writer ────────────────────────────────────────────────────
//
// The contract crate is `no_std` and deliberately carries no serialisation
// dependency, so the fixture is assembled by hand. Values are numbers, bools
// and short identifiers; `json_string` still escapes properly so a future
// field with punctuation cannot corrupt the file.

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Renders a `u64`/`i128` as a JSON **string**.
///
/// Config version hashes are `u64` and settlement amounts are `i128`; both
/// exceed the range JavaScript numbers represent exactly. Emitting them as
/// strings forces the TypeScript side to parse them as `BigInt`, so a parity
/// failure can never be a float-rounding artefact.
fn json_bignum(value: &str) -> String {
    json_string(value)
}

/// One `key: value` pair joined into an object body.
fn json_object(fields: &[(&str, String)]) -> String {
    let mut out = String::from("{");
    for (index, (key, value)) in fields.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&json_string(key));
        out.push(':');
        out.push_str(value);
    }
    out.push('}');
    out
}

fn json_array(items: &[String]) -> String {
    let mut out = String::from("[");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(item);
    }
    out.push(']');
    out
}

/// Re-indents a compact JSON document so the committed fixture reviews as a
/// readable diff rather than one enormous line.
fn pretty(compact: &str) -> String {
    let mut out = String::with_capacity(compact.len() * 2);
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for ch in compact.chars() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '{' | '[' => {
                depth += 1;
                out.push(ch);
                out.push('\n');
                out.push_str(&"  ".repeat(depth));
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                out.push('\n');
                out.push_str(&"  ".repeat(depth));
                out.push(ch);
            }
            ',' => {
                out.push(ch);
                out.push('\n');
                out.push_str(&"  ".repeat(depth));
            }
            ':' => out.push_str(": "),
            c => out.push(c),
        }
    }
    out.push('\n');
    out
}

/// Absolute path of a committed artefact under the repository's `ts/`.
fn ts_path(segments: &[&str]) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("ts");
    for segment in segments {
        path.push(segment);
    }
    path
}

/// Writes `contents` to `path`, creating parent directories, but only when the
/// bytes actually differ — so a clean `cargo test` leaves file mtimes (and
/// incremental TypeScript builds) untouched.
fn write_if_changed(path: &PathBuf, contents: &str) {
    let parent = path.parent().expect("artefact directory");
    fs::create_dir_all(parent).expect("create ts artefact directory");
    let unchanged = fs::read_to_string(path)
        .map(|existing| existing == contents)
        .unwrap_or(false);
    if !unchanged {
        fs::write(path, contents).expect("write ts artefact");
    }
}

/// Renders the generated TypeScript constants module.
///
/// The behavioural fixture is JSON because the parity suite iterates over it,
/// but the *constants* are emitted as a plain `.ts` module so the read helpers
/// can import them with no filesystem access and no bundler configuration —
/// they stay as portable as the hand-written copies they replace, while no
/// longer being hand-written.
fn render_constants_module(
    default_retention_limit: u32,
    symbols: &[(&str, &str)],
    topics: &[(&str, &str)],
) -> String {
    let mut out = String::new();
    out.push_str("// GENERATED FILE - do not edit by hand.\n");
    out.push_str("//\n");
    out.push_str("// Every value below was read out of the running contract by\n");
    out.push_str("// `apexchainx_calculator/src/ts_parity_fixtures.rs`. Regenerate with\n");
    out.push_str("// `just ts-fixtures`. Editing this file by hand will be silently\n");
    out.push_str("// reverted by the next `cargo test`, and CI fails on the diff.\n");
    out.push_str("//\n");
    out.push_str("// See docs/TS_PARITY_CONTRACT.md for the surface this covers.\n\n");

    fn constant(out: &mut String, doc: &str, name: &str, value: &str) {
        out.push_str(&format!("/** {} */\nexport const {} = {};\n\n", doc, name, value));
    }
    constant(
        &mut out,
        "Upper bound the contract clamps a page `limit` to (`history::MAX_PAGE_SIZE`).",
        "MAX_PAGE_SIZE",
        &crate::history::MAX_PAGE_SIZE.to_string(),
    );
    constant(
        &mut out,
        "Hard cap on retained history entries.",
        "MAX_HISTORY_SIZE",
        &crate::MAX_HISTORY_SIZE.to_string(),
    );
    constant(
        &mut out,
        "Recalculations allowed per outage id under one config version.",
        "MAX_RECALCS_PER_OUTAGE",
        &crate::MAX_RECALCS_PER_OUTAGE.to_string(),
    );
    constant(
        &mut out,
        "Retention limit applied when an admin has not set one.",
        "DEFAULT_RETENTION_LIMIT",
        &default_retention_limit.to_string(),
    );
    constant(
        &mut out,
        "Numeric `SLAResult` schema version.",
        "RESULT_SCHEMA_VERSION",
        &crate::RESULT_SCHEMA_VERSION.to_string(),
    );
    constant(
        &mut out,
        "Number of named fields in `SLAResult` at this schema version.",
        "RESULT_FIELD_COUNT",
        &crate::RESULT_SCHEMA_FIELD_COUNT.to_string(),
    );

    out.push_str("/** Status, payment-type, rating and severity symbols the contract emits. */\n");
    out.push_str("export const SYMBOLS = {\n");
    for (role, literal) in symbols {
        out.push_str(&format!("  {}: {},\n", ts_key(role), json_string(literal)));
    }
    out.push_str("} as const;\n\n");

    out.push_str("/** Canonical severities, in the order the contract snapshots them. */\n");
    out.push_str("export const CANONICAL_SEVERITIES = [\n");
    for name in ["critical", "high", "medium", "low"] {
        out.push_str(&format!("  {},\n", json_string(name)));
    }
    out.push_str("] as const;\n\n");

    out.push_str("/** Event topic symbols, keyed by role. */\n");
    out.push_str("export const EVENT_TOPICS = {\n");
    for (role, literal) in topics {
        out.push_str(&format!("  {}: {},\n", role, json_string(literal)));
    }
    out.push_str("} as const;\n\n");

    out.push_str("/** Schema version carried in topic position 2 of every event. */\n");
    out.push_str("export const EVENT_VERSION = \"v1\";\n");
    out
}

/// Converts a dotted fixture role (`status.met`) into a TypeScript identifier
/// (`statusMet`).
fn ts_key(role: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for ch in role.chars() {
        if ch == '.' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// A deterministic outage id for entry `index` — four characters, well inside
/// the nine-character `Symbol` limit.
fn outage_id(env: &Env, index: u32) -> Symbol {
    Symbol::new(env, &format!("o{:03}", index))
}

/// Severity assigned to entry `index`, cycling through the canonical four so
/// every threshold and both SLA outcomes appear in the recorded history.
fn severity_for(index: u32) -> Symbol {
    match index % 4 {
        0 => symbol_short!("critical"),
        1 => symbol_short!("high"),
        2 => symbol_short!("medium"),
        _ => symbol_short!("low"),
    }
}

/// MTTR assigned to entry `index`. The seven values straddle each severity's
/// threshold, so cycling them against the four severities yields met and
/// violated entries across every reward tier. Seven is coprime with four so
/// no (severity, MTTR) pair is skipped.
fn mttr_for(index: u32) -> u32 {
    match index % 7 {
        0 => 0,
        1 => 5,
        2 => 14,
        3 => 29,
        4 => 45,
        5 => 90,
        _ => 200,
    }
}

/// Builds an initialised contract with a recorded history and returns the
/// client plus the ids that were recorded, in insertion order.
fn seed_contract(env: &Env) -> (SLACalculatorContractClient<'_>, StdVec<String>) {
    env.mock_all_auths();
    // Recording 250 results rewrites the whole history vector each time, which
    // exhausts the default test budget long before the page-size clamp is
    // reachable. The budget is a cost model, not a semantic one, and nothing in
    // this fixture depends on it.
    env.budget().reset_unlimited();
    let contract_id = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let operator = Address::generate(env);
    client.initialize(&admin, &operator);

    let mut ids = StdVec::new();
    for index in 0..HISTORY_ENTRIES {
        env.ledger()
            .set_timestamp(FIRST_TIMESTAMP + u64::from(index) * TIMESTAMP_STEP);
        let id = outage_id(env, index);
        client.calculate_sla(&operator, &id, &severity_for(index), &mttr_for(index));
        ids.push(format!("o{:03}", index));
        env.budget().reset_unlimited();
    }

    (client, ids)
}

/// Regenerates `ts/fixtures/contract-read-semantics.json` from live contract
/// behaviour.
///
/// This test always rewrites the fixture. CI runs `git diff --exit-code --
/// ts/fixtures` afterwards, so an uncommitted change to contract read
/// semantics fails the build; see the module docs.
#[test]
fn generate_ts_parity_fixtures() {
    let mut env = Env::default();
    // Each `Env` in this test is dropped while the fixture is being written;
    // the default drop-time snapshot capture would write stray files into the
    // crate root and can itself panic during unwinding.
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    let (client, ids) = seed_contract(&env);

    assert_symbol_table(&env, &client);

    // ── Constants the TypeScript mirrors must not re-derive ────────────────
    let constants = json_object(&[
        ("maxPageSize", crate::history::MAX_PAGE_SIZE.to_string()),
        ("maxHistorySize", crate::MAX_HISTORY_SIZE.to_string()),
        ("maxRecalcsPerOutage", crate::MAX_RECALCS_PER_OUTAGE.to_string()),
        ("defaultRetentionLimit", client.get_retention_limit().to_string()),
        ("resultSchemaVersion", crate::RESULT_SCHEMA_VERSION.to_string()),
        ("resultFieldCount", crate::RESULT_SCHEMA_FIELD_COUNT.to_string()),
        ("historyEntries", HISTORY_ENTRIES.to_string()),
        ("detailedHistoryEntries", DETAILED_HISTORY_ENTRIES.to_string()),
    ]);

    // ── Symbol vocabulary, verified against the contract's own schema ──────
    let symbol_fields: StdVec<(&str, String)> = SYMBOL_TABLE
        .iter()
        .map(|(role, literal)| (*role, json_string(literal)))
        .collect();
    let symbols = json_object(&symbol_fields);

    let topic_fields: StdVec<(&str, String)> = event_topic_table()
        .into_iter()
        .map(|(role, literal, _)| (role, json_string(literal)))
        .collect();
    let event_topics = json_object(&topic_fields);

    // ── Config snapshot and its version hash ───────────────────────────────
    let snapshot = client.get_config_snapshot();
    let mut config_entries = StdVec::new();
    for index in 0..snapshot.entries.len() {
        let entry = snapshot.entries.get(index).expect("config entry");
        config_entries.push(json_object(&[
            ("severity", json_string(&symbol_literal(&env, &entry.severity))),
            ("thresholdMinutes", entry.config.threshold_minutes.to_string()),
            (
                "penaltyPerMinute",
                json_bignum(&entry.config.penalty_per_minute.to_string()),
            ),
            ("rewardBase", json_bignum(&entry.config.reward_base.to_string())),
        ]));
    }
    let config_version_hash = client.get_config_version_hash();
    let config_snapshot = json_object(&[
        ("versionHash", json_bignum(&config_version_hash.to_string())),
        ("entries", json_array(&config_entries)),
    ]);

    // ── Recorded history, exactly as the contract stored it ────────────────
    let full_history = client.get_history_page(&0, &DETAILED_HISTORY_ENTRIES);
    let total = client.get_history_page_with_meta(&0, &1).total;
    let mut history_entries = StdVec::new();
    for index in 0..full_history.len() {
        let entry = full_history.get(index).expect("history entry");
        history_entries.push(json_object(&[
            ("outageId", json_string(&ids[index as usize])),
            ("mttrMinutes", entry.mttr_minutes.to_string()),
            ("thresholdMinutes", entry.threshold_minutes.to_string()),
            ("status", json_string(&symbol_literal(&env, &entry.status))),
            (
                "paymentType",
                json_string(&symbol_literal(&env, &entry.payment_type)),
            ),
            ("rating", json_string(&symbol_literal(&env, &entry.rating))),
            ("amount", json_bignum(&entry.amount.to_string())),
            ("recordedAt", json_bignum(&entry.recorded_at.to_string())),
        ]));
    }

    // ── Pagination cases, recorded from real calls ─────────────────────────
    //
    // Covers the documented edge cases: the MAX_PAGE_SIZE clamp, a limit
    // beyond the end of history, `limit == 0`, an out-of-range offset, and
    // `u32::MAX` on both parameters (the saturating-arithmetic guarantee).
    let pagination_inputs: [(u32, u32); 14] = [
        (0, 1),
        (0, 10),
        (10, 10),
        (0, 200),
        (0, 201),
        (0, u32::MAX),
        (100, 200),
        (240, 10),
        (240, 50),
        (249, 5),
        (250, 5),
        (0, 0),
        (5, 0),
        (u32::MAX, u32::MAX),
    ];
    let mut pagination_cases = StdVec::new();
    for (offset, limit) in pagination_inputs {
        let page = client.get_history_page_with_meta(&offset, &limit);
        let plain = client.get_history_page(&offset, &limit);
        assert_eq!(
            page.items.len(),
            plain.len(),
            "get_history_page and get_history_page_with_meta disagree at offset={} limit={}",
            offset,
            limit
        );
        let first = page
            .items
            .get(0)
            .map(|entry| entry.mttr_minutes.to_string())
            .unwrap_or_else(|| "null".to_string());
        let last = page
            .items
            .get(page.items.len().saturating_sub(1))
            .map(|entry| entry.mttr_minutes.to_string())
            .unwrap_or_else(|| "null".to_string());
        pagination_cases.push(json_object(&[
            ("offset", offset.to_string()),
            ("limit", limit.to_string()),
            ("pageLength", page.items.len().to_string()),
            ("total", page.total.to_string()),
            ("hasMore", page.has_more.to_string()),
            ("firstMttr", first),
            ("lastMttr", last),
        ]));
    }
    assert_eq!(
        total, HISTORY_ENTRIES,
        "history length drifted from the fixture plan"
    );

    // ── Per-outage lookup ──────────────────────────────────────────────────
    let by_outage_inputs = ["o000", "o007", "o249", "o999"];
    let mut by_outage_cases = StdVec::new();
    for id in by_outage_inputs {
        let symbol = Symbol::new(&env, id);
        let matches = client.get_history_by_outage(&symbol);
        let latest = client.get_latest_by_outage(&symbol);
        by_outage_cases.push(json_object(&[
            ("outageId", json_string(id)),
            ("matchCount", matches.len().to_string()),
            (
                "latestMttr",
                latest
                    .map(|entry| entry.mttr_minutes.to_string())
                    .unwrap_or_else(|| "null".to_string()),
            ),
        ]));
    }

    // ── Age-based pruning, measured on the live contract ──────────────────
    //
    // `prune_history_by_age(min_age)` keeps every entry with
    // `recorded_at >= now - min_age`, so the retained set depends only on the
    // cutoff, and a smaller `min_age` always keeps a subset of what a larger
    // one keeps. Walking the cases in descending `min_age` order therefore
    // yields exactly what a freshly seeded contract would return for each,
    // without re-seeding 250 results per case.
    //
    // The contract rejects `min_age >= now` (InvalidInput), so the "retain
    // everything" case is expressed with a large-but-valid age instead of
    // `u64::MAX`.
    let newest = FIRST_TIMESTAMP + u64::from(HISTORY_ENTRIES - 1) * TIMESTAMP_STEP;
    let admin = client.get_admin();
    env.ledger().set_timestamp(newest);
    let mut prune_cases = StdVec::new();
    for min_age_seconds in [newest - 1, 15_000, 3_600, 600, 0] {
        env.budget().reset_unlimited();
        client.prune_history_by_age(&admin, &min_age_seconds);
        let kept = client.get_history_page_with_meta(&0, &1).total;
        prune_cases.push(json_object(&[
            ("now", json_bignum(&newest.to_string())),
            ("minAgeSeconds", json_bignum(&min_age_seconds.to_string())),
            ("keptCount", kept.to_string()),
        ]));
    }

    let document = json_object(&[
        (
            "$comment",
            json_string(
                "GENERATED FILE - do not edit by hand. Regenerate with \
                 `cargo test -p apexchainx_calculator ts_parity_fixtures` (or `just ts-fixtures`). \
                 Every value below was produced by executing the contract in a Soroban Env; \
                 see apexchainx_calculator/src/ts_parity_fixtures.rs and \
                 docs/TS_PARITY_CONTRACT.md.",
            ),
        ),
        (
            "generator",
            json_string("apexchainx_calculator/src/ts_parity_fixtures.rs"),
        ),
        ("constants", constants),
        ("symbols", symbols),
        ("eventTopics", event_topics),
        ("eventVersion", json_string("v1")),
        ("configSnapshot", config_snapshot),
        ("history", json_array(&history_entries)),
        ("paginationCases", json_array(&pagination_cases)),
        ("byOutageCases", json_array(&by_outage_cases)),
        ("pruneByAgeCases", json_array(&prune_cases)),
    ]);

    write_if_changed(
        &ts_path(&["fixtures", "contract-read-semantics.json"]),
        &pretty(&document),
    );

    let symbol_pairs: StdVec<(&str, &str)> = SYMBOL_TABLE.to_vec();
    let topic_pairs: StdVec<(&str, &str)> = event_topic_table()
        .into_iter()
        .map(|(role, literal, _)| (role, literal))
        .collect();
    write_if_changed(
        &ts_path(&["generated", "contractConstants.ts"]),
        &render_constants_module(client.get_retention_limit(), &symbol_pairs, &topic_pairs),
    );
}
