#![no_main]

//! Fuzz target: history state machine (issue #498).
//!
//! History is the contract's highest-volume stateful surface: append
//! (`calculate_sla` with automatic retention trim), `prune_history` (keep-N),
//! `prune_history_by_age`, and offset-based pagination. Previously none of
//! these were fuzzed. This target drives random op sequences through a real
//! contract Env and asserts the retention and pagination invariants after
//! every mutation:
//!
//! * A successful `calculate_sla` never leaves history larger than the current
//!   retention limit (the automatic trim fires in the same call).
//! * A successful `prune_history(N)` bounds history to `N`.
//! * `get_history_page(offset, limit)` agrees with
//!   `get_history_page_with_meta(offset, limit).items`, never exceeds
//!   `min(limit, MAX_PAGE_SIZE)`, returns an empty page for `offset >= total`
//!   or `limit == 0`, and `has_more` is consistent with the covered range.
//! * Walking pages by `offset` reconstructs the stored history exactly.
//! * Every stored entry is well-formed (met/rew or viol/pen, correct sign).

use apexchainx_calculator::{SLACalculatorContract, SLACalculatorContractClient};
use libfuzzer_sys::fuzz_target;
use soroban_sdk::symbol_short;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::EnvTestConfig;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{Address, Env, Symbol};

/// Maximum number of operations decoded from a single fuzz input.
const MAX_OPS: usize = 32;
/// Must match the contract's pagination cap (`history::MAX_PAGE_SIZE`).
const MAX_PAGE_SIZE: u32 = 200;

const OP_CALCULATE: u8 = 0;
const OP_PRUNE_KEEP: u8 = 1;
const OP_PRUNE_AGE: u8 = 2;
const OP_SET_RETENTION: u8 = 3;
const OP_READ_PAGE: u8 = 4;
const OP_COUNT: u8 = 5;

fn read_u32(data: &[u8], pos: &mut usize) -> Option<u32> {
    let end = pos.checked_add(4)?;
    if end > data.len() {
        return None;
    }
    let value = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos = end;
    Some(value)
}

fn read_u64(data: &[u8], pos: &mut usize) -> Option<u64> {
    let end = pos.checked_add(8)?;
    if end > data.len() {
        return None;
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&data[*pos..end]);
    *pos = end;
    Some(u64::from_le_bytes(buf))
}

fn severity(i: u32) -> Symbol {
    match i % 4 {
        0 => symbol_short!("critical"),
        1 => symbol_short!("high"),
        2 => symbol_short!("medium"),
        _ => symbol_short!("low"),
    }
}

fn outage_id(i: u32) -> Symbol {
    match i % 8 {
        0 => symbol_short!("out0"),
        1 => symbol_short!("out1"),
        2 => symbol_short!("out2"),
        3 => symbol_short!("out3"),
        4 => symbol_short!("out4"),
        5 => symbol_short!("out5"),
        6 => symbol_short!("out6"),
        _ => symbol_short!("out7"),
    }
}

fn mttr(raw: u32) -> u32 {
    raw % 2880
}

fn retention(raw: u32) -> u32 {
    1 + (raw % 1000)
}

fn assert_entries_well_formed<'a>(client: &SLACalculatorContractClient<'a>) {
    let history = client.get_history();
    for entry in history.iter() {
        let met = entry.status == symbol_short!("met");
        let viol = entry.status == symbol_short!("viol");
        assert!(met || viol, "invalid status symbol in history");
        if met {
            assert_eq!(
                entry.payment_type,
                symbol_short!("rew"),
                "met entry must pay a reward"
            );
            assert!(entry.amount > 0, "met entry has non-positive amount");
            assert!(
                entry.rating == symbol_short!("top")
                    || entry.rating == symbol_short!("excel")
                    || entry.rating == symbol_short!("good"),
                "invalid met rating in history"
            );
        } else {
            assert_eq!(
                entry.payment_type,
                symbol_short!("pen"),
                "viol entry must be a penalty"
            );
            assert!(entry.amount < 0, "violation entry has non-negative amount");
            assert_eq!(entry.rating, symbol_short!("poor"));
        }
    }
}

/// Asserts the offset-based pagination contract for a single (offset, limit).
fn assert_pagination_consistent<'a>(client: &SLACalculatorContractClient<'a>, offset: u32, limit: u32) {
    let history = client.get_history();
    let total = history.len() as u32;

    let page = client.get_history_page(&offset, &limit);
    let meta = client.get_history_page_with_meta(&offset, &limit); // The two page accessors agree on items and total.
    for (i, item) in page.iter().enumerate() {
        assert_eq!(
            item,
            meta.items.get(i as u32).unwrap(),
            "page vs meta.items diverged"
        );
    }
    assert_eq!(page.len(), meta.items.len(), "page vs meta length diverged");
    assert_eq!(
        meta.total, total,
        "get_history_page_with_meta must report the true total"
    );

    let effective_limit = limit.min(MAX_PAGE_SIZE);
    let expected_len = if offset >= total || effective_limit == 0 {
        0
    } else {
        (total.saturating_sub(offset)).min(effective_limit)
    };
    assert_eq!(
        page.len() as u32,
        expected_len,
        "page length mismatch for (offset={}, limit={})",
        offset,
        limit
    );

    // has_more: entries remain when the requested range stops short of the end
    // and limit > 0.
    let end = offset.saturating_add(effective_limit).min(total);
    let expect_more = effective_limit != 0 && end < total;
    assert_eq!(
        meta.has_more, expect_more,
        "has_more mismatch for (offset={}, limit={})",
        offset, limit
    );

    // Empty-page signalling per the policy.
    if limit == 0 || offset >= total {
        assert_eq!(
            page.len(),
            0,
            "empty page expected for (offset={}, limit={})",
            offset,
            limit
        );
    }
}

fuzz_target!(|data: &[u8]| {
    let mut env = Env::default();
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env.mock_all_auths();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let admin = Address::generate(&env);
    let operator = Address::generate(&env);
    client.initialize(&admin, &operator);

    // Move the ledger forward across ops so recorded_at differs and age-based
    // pruning is exercised with varied inputs.
    env.ledger().set_timestamp(1_000_000);
    let mut now: u64 = 1_000_000;

    let mut pos = 0usize;
    let mut ops = 0usize;
    while pos < data.len() && ops < MAX_OPS {
        let opcode = data[pos] % OP_COUNT;
        pos += 1;

        // Advance the clock a little for every op.
        now = now.saturating_add(1 + (ops as u64) % 120);
        env.ledger().set_timestamp(now);

        match opcode {
            OP_CALCULATE => {
                let (Some(sev), Some(out), Some(m)) = (
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                    read_u32(data, &mut pos),
                ) else {
                    break;
                };
                if client
                    .try_calculate_sla(&operator, &outage_id(out), &severity(sev), &mttr(m))
                    .is_ok()
                {
                    // A successful calculation must respect the retention limit
                    // immediately (the automatic trim runs in the same call).
                    let limit = client.get_retention_limit();
                    let len = client.get_history().len();
                    assert!(
                        len as u32 <= limit,
                        "history {} exceeds retention limit {} after calculate_sla",
                        len,
                        limit
                    );
                }
            }
            OP_PRUNE_KEEP => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                if client.try_prune_history(&admin, &(raw % 300)).is_ok() {
                    let len = client.get_history().len();
                    assert!(
                        len as u32 <= raw % 300,
                        "history {} exceeds prune keep target",
                        len
                    );
                }
            }
            OP_PRUNE_AGE => {
                let Some(raw) = read_u64(data, &mut pos) else {
                    break;
                };
                let _ = client.try_prune_history_by_age(&admin, &raw);
                assert_entries_well_formed(&client);
            }
            OP_SET_RETENTION => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                let limit = retention(raw);
                let _ = client.try_set_retention_limit(&admin, &limit);
                let got = client.get_retention_limit();
                assert!(
                    got == limit || got == 1000,
                    "retention limit clamped unexpectedly"
                );
            }
            OP_READ_PAGE => {
                let (Some(off), Some(lim)) = (read_u32(data, &mut pos), read_u32(data, &mut pos)) else {
                    break;
                };
                assert_pagination_consistent(&client, off, lim);
            }
            _ => unreachable!(),
        }
        ops += 1;
    }

    // Terminal checks.
    assert_entries_well_formed(&client);

    // A full page walk reconstructs the stored history exactly.
    let history = client.get_history();
    let total = history.len() as u32;
    let chunk = 64u32;
    let mut offset = 0u32;
    let mut idx = 0usize;
    loop {
        let page = client.get_history_page(&offset, &chunk);
        for item in page.iter() {
            let stored = history.get(idx as u32).unwrap();
            assert_eq!(
                item, stored,
                "page walk diverged from stored history at index {}",
                idx
            );
            idx += 1;
        }
        if offset.saturating_add(chunk) >= total {
            break;
        }
        offset = offset.saturating_add(chunk);
    }
    assert_eq!(
        idx as u32, total,
        "page walk did not cover the full retained history"
    );

    // Pagination edge cases against the current history.
    assert_pagination_consistent(&client, 0, 0);
    assert_pagination_consistent(&client, 0, u32::MAX);
    assert_pagination_consistent(&client, total, 1);
    assert_pagination_consistent(&client, total.saturating_add(1), 200);
    assert_pagination_consistent(&client, u32::MAX, u32::MAX);
});
