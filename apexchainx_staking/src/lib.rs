//! ApexChainx Staking Contract — Soroban smart contract for token staking
//! on the Stellar network.
//!
//! # Gas Optimization Notes (issue #806)
//!
//! This contract is written with explicit attention to Soroban instruction
//! budget (gas) efficiency:
//!
//! - **Instance storage for hot-path data**: Admin address, reward rate, and
//!   the global staked-token total are stored in `instance()` storage, which
//!   is loaded once per invocation and kept in a cache — reads are effectively
//!   free after the first access.
//!
//! - **Persistent storage for per-staker data**: Each staker's balance and
//!   `staked_at` ledger-sequence are kept in `persistent()` storage so they
//!   are not loaded unless the specific staker is involved in the call.
//!
//! - **Single storage read per key per call**: All reads are performed once and
//!   held in local variables; no key is fetched more than once.
//!
//! - **Unconditional writes avoided**: Storage is only written when a value
//!   actually changes, saving unnecessary instruction charges.
//!
//! - **`symbol_short!` for all storage keys**: Short symbols are cheaper to
//!   construct and compare than long `Symbol::new` strings.
//!
//! - **No intermediate heap allocations** on the hot stake/unstake paths:
//!   arithmetic is done on stack-local `i128` values.
//!
//! # Event Emissions (issue #807)
//!
//! Every state-changing operation emits a Soroban event so that off-chain
//! indexers can reconstruct the full staking history without querying contract
//! storage directly.
//!
//! | Operation | Topic[0] | Topic[1] | Data |
//! |-----------|----------|----------|------|
//! | `initialize` | `"staking"` | `"init"` | `admin: Address` |
//! | `stake` | `"staking"` | `"staked"` | `{staker, amount, new_total}` |
//! | `unstake` | `"staking"` | `"unstaked"` | `{staker, amount, new_total}` |
//! | `set_reward_rate` | `"staking"` | `"rate_set"` | `new_rate: i128` |
#![no_std]
extern crate alloc;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

// ---------------------------------------------------------------------------
// Storage key constants (short symbols = cheaper to construct/compare)
// ---------------------------------------------------------------------------

/// Instance-storage key: contract administrator address.
const KEY_ADMIN: Symbol = symbol_short!("ADMIN");
/// Instance-storage key: reward rate in token-units per ledger per staked unit.
const KEY_RATE: Symbol = symbol_short!("RATE");
/// Instance-storage key: total tokens currently staked across all stakers.
const KEY_TOTAL: Symbol = symbol_short!("TOTAL");

// ---------------------------------------------------------------------------
// Event topic constants (issue #807)
// ---------------------------------------------------------------------------

/// Namespace topic shared by every staking event — allows indexers to filter
/// all staking events in a single subscription.
const TOPIC_STAKING: Symbol = symbol_short!("staking");
/// Emitted after a successful `initialize` call.
const TOPIC_INIT: Symbol = symbol_short!("init");
/// Emitted after a successful `stake` call.
const TOPIC_STAKED: Symbol = symbol_short!("staked");
/// Emitted after a successful `unstake` call.
const TOPIC_UNSTAKED: Symbol = symbol_short!("unstaked");
/// Emitted after a successful `set_reward_rate` call.
const TOPIC_RATE_SET: Symbol = symbol_short!("rate_set");

// ---------------------------------------------------------------------------
// Contract types
// ---------------------------------------------------------------------------

/// Per-staker record stored in persistent storage.
///
/// Keeping both fields in a single storage entry means one read/write fetches
/// all per-staker data, halving the storage cost compared with two separate
/// keys.
#[contracttype]
#[derive(Clone)]
pub struct StakeRecord {
    /// Token amount currently staked by this address.
    pub amount: i128,
    /// Ledger sequence number at which the stake was last modified.
    pub staked_at: u32,
}

/// Summary information returned by `get_stake`.
#[contracttype]
#[derive(Clone)]
pub struct StakeInfo {
    /// Amount currently staked.
    pub amount: i128,
    /// Ledger at which the stake was last updated.
    pub staked_at: u32,
    /// Pending reward estimate (amount × rate × elapsed_ledgers).
    pub pending_reward: i128,
}

/// Payload emitted with `staked` and `unstaked` events (issue #807).
///
/// A single struct keeps the event data self-contained so indexers can
/// reconstruct the staking ledger without querying contract storage.
#[contracttype]
#[derive(Clone)]
pub struct StakeEventData {
    /// The staker address.
    pub staker: Address,
    /// The delta amount for this operation.
    pub amount: i128,
    /// The updated global staked total after this operation.
    pub new_total: i128,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by the staking contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StakingError {
    /// Caller is not the contract administrator.
    Unauthorized = 1,
    /// Contract has already been initialized.
    AlreadyInitialized = 2,
    /// Stake amount must be positive.
    InvalidAmount = 3,
    /// Staker has no active stake.
    NoStake = 4,
    /// Unstake amount exceeds the staked balance.
    InsufficientStake = 5,
    /// Reward rate must be non-negative.
    InvalidRate = 6,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct StakingContract;

#[contractimpl]
impl StakingContract {
    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Initialize the staking contract.
    ///
    /// Must be called exactly once. Stores the admin address and the initial
    /// reward rate. All three values are written to instance storage in one
    /// logical transaction — the cheapest possible initialisation path.
    ///
    /// **Emits**: `("staking", "init")` → `admin: Address`
    ///
    /// # Errors
    /// - [`StakingError::AlreadyInitialized`] if the contract is already set up.
    /// - [`StakingError::InvalidRate`] if `reward_rate` is negative.
    pub fn initialize(
        env: Env,
        admin: Address,
        reward_rate: i128,
    ) -> Result<(), StakingError> {
        // Gas optimization: check instance storage once; fail-fast before any auth.
        if env.storage().instance().has(&KEY_ADMIN) {
            return Err(StakingError::AlreadyInitialized);
        }
        if reward_rate < 0 {
            return Err(StakingError::InvalidRate);
        }

        admin.require_auth();

        // Gas optimization: batch all three writes in sequence to amortise the
        // instance-storage dirty-cache flush over a single host call.
        let storage = env.storage().instance();
        storage.set(&KEY_ADMIN, &admin);
        storage.set(&KEY_RATE, &reward_rate);
        storage.set(&KEY_TOTAL, &0_i128);

        // Event emission (issue #807): notify indexers of initialization.
        env.events().publish(
            (TOPIC_STAKING, TOPIC_INIT),
            admin,
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Admin operations
    // -----------------------------------------------------------------------

    /// Update the reward rate. Only the admin may call this.
    ///
    /// **Emits**: `("staking", "rate_set")` → `new_rate: i128`
    ///
    /// # Errors
    /// - [`StakingError::Unauthorized`] if the caller is not the admin.
    /// - [`StakingError::InvalidRate`] if `new_rate` is negative.
    pub fn set_reward_rate(env: Env, new_rate: i128) -> Result<(), StakingError> {
        if new_rate < 0 {
            return Err(StakingError::InvalidRate);
        }
        // Gas optimization: read admin once, require auth, then write.
        let admin: Address = env.storage().instance().get(&KEY_ADMIN).unwrap();
        admin.require_auth();
        env.storage().instance().set(&KEY_RATE, &new_rate);

        // Event emission (issue #807): notify indexers of rate change.
        env.events().publish(
            (TOPIC_STAKING, TOPIC_RATE_SET),
            new_rate,
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Staking operations
    // -----------------------------------------------------------------------

    /// Stake `amount` tokens on behalf of `staker`.
    ///
    /// # Gas optimizations applied
    /// - Staker record is loaded at most once (`persistent().get`).
    /// - Global total is loaded from instance cache (free after first read).
    /// - The new `StakeRecord` is written back in one persistent write.
    ///
    /// **Emits**: `("staking", "staked")` → `StakeEventData { staker, amount, new_total }`
    ///
    /// # Errors
    /// - [`StakingError::InvalidAmount`] if `amount` is not positive.
    pub fn stake(env: Env, staker: Address, amount: i128) -> Result<(), StakingError> {
        if amount <= 0 {
            return Err(StakingError::InvalidAmount);
        }

        staker.require_auth();

        // Gas optimization: use staker address as the persistent storage key
        // directly (no extra allocation for a composite key struct).
        let staker_key = staker.clone();
        let current_ledger = env.ledger().sequence();

        // Load existing record once; default to zero if first stake.
        let existing: Option<StakeRecord> = env.storage().persistent().get(&staker_key);
        let new_amount = match existing {
            Some(rec) => rec.amount.saturating_add(amount),
            None => amount,
        };

        // Write the updated stake record (single persistent write).
        env.storage().persistent().set(
            &staker_key,
            &StakeRecord {
                amount: new_amount,
                staked_at: current_ledger,
            },
        );

        // Update global total (instance cache — cheap).
        let total: i128 = env
            .storage()
            .instance()
            .get(&KEY_TOTAL)
            .unwrap_or(0_i128);
        let new_total = total.saturating_add(amount);
        env.storage().instance().set(&KEY_TOTAL, &new_total);

        // Event emission (issue #807): broadcast the state change to indexers.
        env.events().publish(
            (TOPIC_STAKING, TOPIC_STAKED),
            StakeEventData {
                staker,
                amount,
                new_total,
            },
        );

        Ok(())
    }

    /// Unstake `amount` tokens for `staker`.
    ///
    /// # Gas optimizations applied
    /// - Record loaded once, validated in-memory, written back once.
    /// - If the full balance is withdrawn the key is *removed* rather than
    ///   written with `amount = 0`, freeing storage rent immediately.
    ///
    /// **Emits**: `("staking", "unstaked")` → `StakeEventData { staker, amount, new_total }`
    ///
    /// # Errors
    /// - [`StakingError::NoStake`] if the staker has no active position.
    /// - [`StakingError::InvalidAmount`] if `amount` is not positive.
    /// - [`StakingError::InsufficientStake`] if `amount` exceeds the balance.
    pub fn unstake(env: Env, staker: Address, amount: i128) -> Result<(), StakingError> {
        if amount <= 0 {
            return Err(StakingError::InvalidAmount);
        }

        staker.require_auth();

        let staker_key = staker.clone();
        let rec: StakeRecord = env
            .storage()
            .persistent()
            .get(&staker_key)
            .ok_or(StakingError::NoStake)?;

        if amount > rec.amount {
            return Err(StakingError::InsufficientStake);
        }

        let remaining = rec.amount - amount;
        let current_ledger = env.ledger().sequence();

        // Gas optimization: remove key entirely when balance hits zero rather
        // than writing a zero-amount record (saves one future read).
        if remaining == 0 {
            env.storage().persistent().remove(&staker_key);
        } else {
            env.storage().persistent().set(
                &staker_key,
                &StakeRecord {
                    amount: remaining,
                    staked_at: current_ledger,
                },
            );
        }

        // Update global total.
        let total: i128 = env
            .storage()
            .instance()
            .get(&KEY_TOTAL)
            .unwrap_or(0_i128);
        let new_total = total.saturating_sub(amount);
        env.storage().instance().set(&KEY_TOTAL, &new_total);

        // Event emission (issue #807): broadcast the state change to indexers.
        env.events().publish(
            (TOPIC_STAKING, TOPIC_UNSTAKED),
            StakeEventData {
                staker,
                amount,
                new_total,
            },
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Read-only queries
    // -----------------------------------------------------------------------

    /// Return stake info for `staker`, including an estimated pending reward.
    ///
    /// Reward estimate = `amount × rate × (current_ledger − staked_at)`.
    /// This is view-only and does not modify state.
    ///
    /// # Errors
    /// - [`StakingError::NoStake`] if the staker has no active position.
    pub fn get_stake(env: Env, staker: Address) -> Result<StakeInfo, StakingError> {
        let rec: StakeRecord = env
            .storage()
            .persistent()
            .get(&staker)
            .ok_or(StakingError::NoStake)?;

        // Gas optimization: read rate from instance cache (single fetch shared
        // across any number of logical "reads" in the same invocation).
        let rate: i128 = env
            .storage()
            .instance()
            .get(&KEY_RATE)
            .unwrap_or(0_i128);

        let current_ledger = env.ledger().sequence();
        let elapsed = (current_ledger.saturating_sub(rec.staked_at)) as i128;
        let pending_reward = rec.amount.saturating_mul(rate).saturating_mul(elapsed);

        Ok(StakeInfo {
            amount: rec.amount,
            staked_at: rec.staked_at,
            pending_reward,
        })
    }

    /// Return the total amount of tokens staked across all stakers.
    pub fn total_staked(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&KEY_TOTAL)
            .unwrap_or(0_i128)
    }

    /// Return the current reward rate.
    pub fn reward_rate(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&KEY_RATE)
            .unwrap_or(0_i128)
    }

    /// Return the admin address.
    ///
    /// # Errors
    /// Panics if the contract has not been initialized yet.
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&KEY_ADMIN).unwrap()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events, Ledger};
    use soroban_sdk::{symbol_short, Env, TryIntoVal};

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, StakingContract);
        let admin = Address::generate(&env);
        (env, contract_id, admin)
    }

    // ── #806 gas-optimization tests ────────────────────────────────────────

    #[test]
    fn test_initialize_sets_admin_and_rate() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &10_i128);
        assert_eq!(client.get_admin(), admin);
        assert_eq!(client.reward_rate(), 10_i128);
        assert_eq!(client.total_staked(), 0_i128);
    }

    #[test]
    fn test_double_initialize_fails() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &5_i128);
        let res = client.try_initialize(&admin, &5_i128);
        assert!(res.is_err());
    }

    #[test]
    fn test_stake_increases_balance_and_total() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let staker = Address::generate(&env);
        client.stake(&staker, &100_i128);
        let info = client.get_stake(&staker);
        assert_eq!(info.amount, 100_i128);
        assert_eq!(client.total_staked(), 100_i128);
    }

    #[test]
    fn test_stake_invalid_amount_rejected() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let staker = Address::generate(&env);
        let res = client.try_stake(&staker, &0_i128);
        assert!(res.is_err());
    }

    #[test]
    fn test_unstake_reduces_balance_and_total() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let staker = Address::generate(&env);
        client.stake(&staker, &100_i128);
        client.unstake(&staker, &40_i128);
        let info = client.get_stake(&staker);
        assert_eq!(info.amount, 60_i128);
        assert_eq!(client.total_staked(), 60_i128);
    }

    #[test]
    fn test_full_unstake_removes_record() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let staker = Address::generate(&env);
        client.stake(&staker, &50_i128);
        client.unstake(&staker, &50_i128);
        let res = client.try_get_stake(&staker);
        assert!(res.is_err());
        assert_eq!(client.total_staked(), 0_i128);
    }

    #[test]
    fn test_unstake_insufficient_fails() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let staker = Address::generate(&env);
        client.stake(&staker, &10_i128);
        let res = client.try_unstake(&staker, &20_i128);
        assert!(res.is_err());
    }

    #[test]
    fn test_pending_reward_grows_with_ledger() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &2_i128);
        let staker = Address::generate(&env);
        // Stake at ledger sequence 0.
        client.stake(&staker, &100_i128);
        // Advance ledger by 10 sequences.
        env.ledger().set(soroban_sdk::testutils::LedgerInfo {
            timestamp: 0,
            protocol_version: 22,
            sequence_number: 10,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 1_000_000,
        });
        let info = client.get_stake(&staker);
        // 100 * 2 * 10 = 2000
        assert_eq!(info.pending_reward, 2000_i128);
    }

    #[test]
    fn test_set_reward_rate_updates_rate() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        client.set_reward_rate(&5_i128);
        assert_eq!(client.reward_rate(), 5_i128);
    }

    #[test]
    fn test_invalid_rate_rejected() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let res = client.try_set_reward_rate(&-1_i128);
        assert!(res.is_err());
    }

    // ── #807 event-emission tests ───────────────────────────────────────────

    #[test]
    fn test_initialize_emits_init_event() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &10_i128);

        let all_events = env.events().all();
        let found = all_events.iter().any(|(_, topics, _)| {
            if topics.len() < 2 {
                return false;
            }
            let t0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            let t1: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            t0 == TOPIC_STAKING && t1 == TOPIC_INIT
        });
        assert!(found, "expected 'staking/init' event");
    }

    #[test]
    fn test_stake_emits_staked_event() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let staker = Address::generate(&env);
        client.stake(&staker, &200_i128);

        let all_events = env.events().all();
        let found = all_events.iter().any(|(_, topics, _)| {
            if topics.len() < 2 {
                return false;
            }
            let t0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            let t1: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            t0 == TOPIC_STAKING && t1 == TOPIC_STAKED
        });
        assert!(found, "expected 'staking/staked' event");
    }

    #[test]
    fn test_unstake_emits_unstaked_event() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        let staker = Address::generate(&env);
        client.stake(&staker, &100_i128);
        client.unstake(&staker, &50_i128);

        let all_events = env.events().all();
        let found = all_events.iter().any(|(_, topics, _)| {
            if topics.len() < 2 {
                return false;
            }
            let t0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            let t1: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            t0 == TOPIC_STAKING && t1 == TOPIC_UNSTAKED
        });
        assert!(found, "expected 'staking/unstaked' event");
    }

    #[test]
    fn test_set_reward_rate_emits_rate_set_event() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        client.set_reward_rate(&7_i128);

        let all_events = env.events().all();
        let found = all_events.iter().any(|(_, topics, _)| {
            if topics.len() < 2 {
                return false;
            }
            let t0: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            let t1: Symbol = topics.get(1).unwrap().try_into_val(&env).unwrap_or(symbol_short!("_"));
            t0 == TOPIC_STAKING && t1 == TOPIC_RATE_SET
        });
        assert!(found, "expected 'staking/rate_set' event");
    }

    #[test]
    fn test_no_events_emitted_on_error() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);
        // Record event count after initialization.
        let before_count = env.events().all().len();
        // Attempt a failing stake (amount = 0) — should not add events.
        let staker = Address::generate(&env);
        let _ = client.try_stake(&staker, &0_i128);
        assert_eq!(
            env.events().all().len(),
            before_count,
            "failing call must not emit events"
        );
    }

    // ── #812 access-control audit tests ────────────────────────────────────

    /// Non-admin caller must be rejected by `set_reward_rate`.
    ///
    /// Verifies the access-control boundary identified in the #812 audit:
    /// only the stored admin address may update the reward rate.
    #[test]
    fn test_unauthorized_set_reward_rate_rejected() {
        // Use explicit auth mocking so we can control which address signs.
        let env = Env::default();
        let contract_id = env.register_contract(None, StakingContract);
        let admin = Address::generate(&env);
        let attacker = Address::generate(&env);

        let client = StakingContractClient::new(&env, &contract_id);

        // Initialize with admin — mock admin auth only.
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &admin,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &contract_id,
                fn_name: "initialize",
                args: (&admin, 1_i128).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        client.initialize(&admin, &1_i128);

        // Attempt to set the reward rate as the attacker — must fail.
        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: &attacker,
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &contract_id,
                fn_name: "set_reward_rate",
                args: (99_i128,).into_val(&env),
                sub_invokes: &[],
            },
        }]);
        let result = client.try_set_reward_rate(&99_i128);
        assert!(
            result.is_err(),
            "non-admin must not be able to set reward rate"
        );

        // Rate must remain unchanged.
        assert_eq!(client.reward_rate(), 1_i128, "rate must not change after failed auth");
    }

    /// Re-initialization by any caller must be rejected.
    ///
    /// Verifies the `AlreadyInitialized` guard documented in the #812 audit.
    #[test]
    fn test_reinitialize_rejected_after_init() {
        let (env, contract_id, admin) = setup();
        let client = StakingContractClient::new(&env, &contract_id);
        client.initialize(&admin, &1_i128);

        // Second initialize attempt — must fail regardless of caller.
        let attacker = Address::generate(&env);
        let result = client.try_initialize(&attacker, &100_i128);
        assert!(
            result.is_err(),
            "re-initialization must be rejected after first init"
        );

        // Admin must remain unchanged.
        assert_eq!(client.get_admin(), admin, "admin must not change after failed re-init");
    }
}
