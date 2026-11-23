#![no_main]

//! Fuzz target: governance state machine (issue #498).
//!
//! The two-step admin/operator transfer handoff (`propose → accept`,
//! `propose → cancel`, `renounce`) is one of the contract's most stateful
//! paths and previously had no fuzz coverage. This target drives random
//! governance operation sequences through the real contract Env and asserts
//! the pending-slot invariants after every successful mutation:
//!
//! * `propose_admin(X)` ok ⇒ `get_pending_admin()` is `Some(X)`.
//! * `cancel_admin_proposal` / `accept_admin` ok ⇒ the pending admin slot is
//!   cleared.
//! * the stored admin matches the last accepted candidate until renunciation.
//! * Only the *named proposed address* can accept a pending proposal; accepting
//!   with any other address must fail (`Unauthorized`).
//! * `renounce_admin` ok ⇒ every pending slot (admin *and* operator) is
//!   cleared, and the contract stays role-consistent from then on.
//! * Operator handoff mirrors admin: `propose_operator(X)` ⇒ pending operator
//!   is `Some(X)`, `accept_operator`/`cancel_operator_proposal` clear it, and
//!   a successful `accept_operator` installs the accepted address.
//! * the stored operator matches the last accepted or directly assigned address.
//!
//! Everything runs through the `try_*` client variants so a *rejected* op
//! (no pending transfer, wrong proposer, admin renounced, expired proposal)
//! becomes an `Err` rather than a panic — the fuzzer is free to explore the
//! rejection paths without crashing, and only the *state transitions* that
//! succeeded are asserted against.

use apexchainx_calculator::SLACalculatorContract;
use apexchainx_calculator::SLACalculatorContractClient;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::EnvTestConfig;
use soroban_sdk::testutils::Ledger as _;
use soroban_sdk::{Address, Env};

/// Maximum number of operations decoded from a single fuzz input.
const MAX_OPS: usize = 32;
const PROPOSAL_EXPIRY_WINDOW: u64 = 90 * 24 * 60 * 60;

const OP_PROPOSE_ADMIN: u8 = 0;
const OP_ACCEPT_ADMIN: u8 = 1;
const OP_CANCEL_ADMIN: u8 = 2;
const OP_RENOUNCE_ADMIN: u8 = 3;
const OP_PROPOSE_OP: u8 = 4;
const OP_ACCEPT_OP: u8 = 5;
const OP_CANCEL_OP: u8 = 6;
const OP_SET_OPERATOR: u8 = 7;
const OP_ADVANCE_TIME: u8 = 8;
const OP_COUNT: u8 = 9;

fn read_u32(data: &[u8], pos: &mut usize) -> Option<u32> {
    let end = pos.checked_add(4)?;
    if end > data.len() {
        return None;
    }
    let value = u32::from_le_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos = end;
    Some(value)
}

fuzz_target!(|data: &[u8]| {
    let mut env = Env::default();
    env.set_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    env.mock_all_auths();
    let cid = env.register_contract(None, SLACalculatorContract);
    let client = SLACalculatorContractClient::new(&env, &cid);
    let mut admin = Address::generate(&env);
    let mut operator = Address::generate(&env);
    client.initialize(&admin, &operator);

    // A small address pool so sequences can propose/accept distinct parties.
    let mut candidates: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
    for _ in 0..4 {
        candidates.push_back(Address::generate(&env));
    }
    fn candidate(cands: &soroban_sdk::Vec<Address>, i: u32) -> Address {
        cands.get(i % cands.len()).unwrap()
    }

    let mut renounced = false;
    let mut pos = 0usize;
    let mut ops = 0usize;
    while pos < data.len() && ops < MAX_OPS {
        let opcode = data[pos] % OP_COUNT;
        pos += 1;
        match opcode {
            OP_PROPOSE_ADMIN => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                if renounced {
                    continue;
                }
                let new_admin = candidate(&candidates, raw);
                if client.try_propose_admin(&admin, &new_admin).is_ok() {
                    // Pending-slot invariant: the proposal is immediately visible.
                    assert_eq!(
                        client.get_pending_admin(),
                        Some(new_admin.clone()),
                        "propose_admin ok must set the pending admin slot"
                    );
                    // Only the named proposed address can accept.
                    let wrong = candidate(&candidates, raw.wrapping_add(1));
                    if wrong != new_admin {
                        assert!(
                            client.try_accept_admin(&wrong).is_err(),
                            "a stranger must not be able to accept an admin proposal"
                        );
                    }
                }
            }
            OP_ACCEPT_ADMIN => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                let would_be = candidate(&candidates, raw);
                if client.try_accept_admin(&would_be).is_ok() {
                    assert!(
                        client.get_pending_admin().is_none(),
                        "accept_admin ok must clear the pending admin slot"
                    );
                    if !renounced {
                        assert_eq!(
                            client.get_admin(),
                            would_be,
                            "accept_admin must promote the caller"
                        );
                        admin = would_be;
                    }
                }
            }
            OP_CANCEL_ADMIN => {
                if renounced {
                    continue;
                }
                if let Err(_e) = client.try_cancel_admin_proposal(&admin) {
                    // NoPendingTransfer when nothing is pending — nothing asserted.
                } else {
                    assert!(
                        client.get_pending_admin().is_none(),
                        "cancel_admin_proposal ok must clear the pending admin slot"
                    );
                }
            }
            OP_RENOUNCE_ADMIN => {
                if client.try_renounce_admin(&admin).is_ok() {
                    renounced = true;
                    assert!(
                        client.get_pending_admin().is_none() && client.get_pending_operator().is_none(),
                        "renounce_admin ok must clear every pending governance slot"
                    );
                }
            }
            OP_PROPOSE_OP => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                if renounced {
                    continue;
                }
                let new_op = candidate(&candidates, raw);
                if client.try_propose_operator(&admin, &new_op).is_ok() {
                    assert_eq!(
                        client.get_pending_operator(),
                        Some(new_op.clone()),
                        "propose_operator ok must set the pending operator slot"
                    );
                    let wrong = candidate(&candidates, raw.wrapping_add(1));
                    if wrong != new_op {
                        assert!(
                            client.try_accept_operator(&wrong).is_err(),
                            "a stranger must not be able to accept an operator proposal"
                        );
                    }
                }
            }
            OP_ACCEPT_OP => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                let would_be = candidate(&candidates, raw);
                if client.try_accept_operator(&would_be).is_ok() {
                    assert!(
                        client.get_pending_operator().is_none(),
                        "accept_operator ok must clear the pending operator slot"
                    );
                    assert_eq!(
                        client.get_operator(),
                        would_be,
                        "accept_operator must install the caller"
                    );
                    operator = would_be;
                }
            }
            OP_CANCEL_OP => {
                if renounced {
                    continue;
                }
                if client.try_cancel_operator_proposal(&admin).is_ok() {
                    assert!(
                        client.get_pending_operator().is_none(),
                        "cancel_operator_proposal ok must clear the pending operator slot"
                    );
                }
            }
            OP_SET_OPERATOR => {
                let Some(raw) = read_u32(data, &mut pos) else {
                    break;
                };
                if renounced {
                    continue;
                }
                let new_op = candidate(&candidates, raw);
                if client.try_set_operator(&admin, &new_op).is_ok() {
                    assert_eq!(
                        client.get_operator(),
                        new_op,
                        "set_operator must install the operator"
                    );
                    operator = new_op;
                }
            }
            OP_ADVANCE_TIME => {
                let current_timestamp = env.ledger().timestamp();
                env.ledger()
                    .set_timestamp(current_timestamp + PROPOSAL_EXPIRY_WINDOW + 1);

                if let Some(pending_admin) = client.get_pending_admin() {
                    let current_admin = client.get_admin();
                    assert!(
                        client.try_accept_admin(&pending_admin).is_err(),
                        "an expired admin proposal must not be accepted"
                    );
                    assert_eq!(client.get_admin(), current_admin);
                    assert_eq!(client.get_pending_admin(), Some(pending_admin));
                }

                if let Some(pending_operator) = client.get_pending_operator() {
                    let current_operator = client.get_operator();
                    assert!(
                        client.try_accept_operator(&pending_operator).is_err(),
                        "an expired operator proposal must not be accepted"
                    );
                    assert_eq!(client.get_operator(), current_operator);
                    assert_eq!(client.get_pending_operator(), Some(pending_operator));
                }
            }
            _ => unreachable!(),
        }

        assert_eq!(
            client.get_operator(),
            operator,
            "the active operator must match the last successful handoff"
        );
        if !renounced {
            assert_eq!(
                client.get_admin(),
                admin,
                "the active admin must match the last successful handoff"
            );
        }
        ops += 1;
    }

    // Terminal invariant: after any sequence, a renounced contract has no
    // pending role entries and active roles match the last successful changes.
    if renounced {
        assert!(client.get_pending_admin().is_none());
        assert!(client.get_pending_operator().is_none());
    }
    assert_eq!(client.get_operator(), operator);
    if !renounced {
        assert_eq!(client.get_admin(), admin);
    }
});
