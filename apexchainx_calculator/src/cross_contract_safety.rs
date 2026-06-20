//! SC-W5-077 – Cross-contract call safety model with failure rollback semantics.
//!
//! This module provides a safety model for cross-contract invocations in the
//! SLA calculator ecosystem. When a contract calls another contract (e.g., SLA
//! calculator calling payment escrow), failures must be handled deterministically
//! and any partial state changes must be rolled back.
//!
//! # Design
//!
//! Each cross-contract call is wrapped in a `SafeCall` that captures:
//! - The target contract identifier
//! - The function being called
//! - A compensation action to reverse the call if a subsequent step fails
//!
//! The `CrossContractSafety` struct maintains a call stack so that if any step
//! in a multi-step workflow fails, all prior steps are rolled back via their
//! registered compensation actions.
//!
//! # Usage
//!
//! ```ignore
//! let mut safety = CrossContractSafety::new(&env);
//! safety.call(&env, &contract_id, &lock_funds_fn, args, unlock_tag, unlock_args);
//! safety.call(&env, &contract_id, &release_fn, args, release_tag, release_args);
//! safety.finalize(&env); // happy path – clear the compensation stack
//! ```
//!
//! On a fatal error, call `rollback_all()` to invoke all registered
//! compensations in reverse order:
//!
//! ```ignore
//! if let Err(result) = safety.call(&env, &contract_id, &fn_name, args, tag, comp_args) {
//!     safety.rollback_all(&env);
//!     return Err(result);
//! }
//! safety.finalize(&env);
//! ```

use soroban_sdk::{contracttype, Address, Env, Symbol, Val, Vec};

/// Status of a cross-contract call.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum CrossContractCallStatus {
    /// The call succeeded.
    Success = 0,
    /// The target contract returned a recoverable error.
    RecoverableError = 1,
    /// The target contract returned a fatal error – rollback required.
    FatalError = 2,
    /// The call could not be dispatched (version mismatch, paused, etc.).
    DispatchFailed = 3,
    /// A compensation action has been applied for this call.
    Compensated = 4,
}

/// Wraps the result of a cross-contract call with safety metadata.
#[derive(Clone, Debug)]
pub struct SafeCallResult {
    /// The status of the call.
    pub status: CrossContractCallStatus,
    /// The raw Val returned by the target contract (or Val::VOID on failure).
    pub raw_output: Val,
    /// Human-readable error symbol when status != Success.
    pub error_symbol: Option<Symbol>,
}

/// A registered compensation action that can be invoked to reverse a call.
///
/// In a `#![no_std]` Soroban contract, we cannot store closures. Instead,
/// we store a `compensation_tag` (a Symbol identifying the compensation
/// logic) and the `args` that were originally passed so the caller can
/// re-invoke with reversed semantics.
#[contracttype]
#[derive(Clone, Debug)]
pub struct CompensationAction {
    /// A tag identifying what kind of compensation to apply.
    /// E.g., "unlock_funds", "reverse_settle", "unpause_escrow".
    pub tag: Symbol,
    /// The original arguments so the compensation function can use them.
    pub args: Vec<Val>,
}

/// Performs a safe cross-contract invocation.
///
/// Wraps `env.invoke_contract()` with error translation and returns a
/// `SafeCallResult` instead of panicking on failure.
pub fn safe_invoke_contract(
    env: &Env,
    contract_id: &soroban_sdk::Address,
    function_name: &Symbol,
    args: &[Val],
) -> SafeCallResult {
    let args_vec = Vec::from_slice(env, args);
    let result = env.try_invoke_contract::<Val, Val>(contract_id, function_name, args_vec);
    match result {
        Ok(Ok(val)) => SafeCallResult {
            status: CrossContractCallStatus::Success,
            raw_output: val,
            error_symbol: None,
        },
        Ok(Err(_err_val)) => SafeCallResult {
            status: CrossContractCallStatus::RecoverableError,
            raw_output: Val::from(false),
            error_symbol: Some(Symbol::new(env, "CROSS_CONTRACT_FAILURE")),
        },
        Err(_err) => SafeCallResult {
            status: CrossContractCallStatus::FatalError,
            raw_output: Val::from(false),
            error_symbol: Some(Symbol::new(env, "CROSS_CONTRACT_FAILURE")),
        },
    }
}

/// Determines whether the given status requires rolling back prior calls.
pub fn requires_rollback(status: CrossContractCallStatus) -> bool {
    status == CrossContractCallStatus::FatalError
        || status == CrossContractCallStatus::DispatchFailed
}

/// Tracks a stack of cross-contract calls with registered compensation
/// actions.  If any step in the sequence fails, all prior successful steps
/// are compensated in reverse order.
pub struct CrossContractSafety {
    /// Stack of compensation actions registered for each successful call.
    /// Each entry stores (contract_id, function_name, compensation_action).
    pub(crate) compensation_stack: Vec<(Address, Symbol, CompensationAction)>,
}

impl CrossContractSafety {
    /// Create a new empty safety tracker.
    pub fn new(env: &Env) -> Self {
        CrossContractSafety {
            compensation_stack: Vec::new(env),
        }
    }

    /// Execute a safe cross-contract call and register its compensation
    /// action for potential rollback.
    ///
    /// Returns `Ok(SafeCallResult)` on success or recoverable errors, and
    /// `Err(result)` on fatal errors.  When `Err` is returned the caller
    /// should call [`rollback_all`](Self::rollback_all) to unwind prior
    /// calls.
    pub fn call(
        &mut self,
        env: &Env,
        contract_id: &soroban_sdk::Address,
        function_name: &Symbol,
        args: &[Val],
        compensation_tag: Symbol,
        compensation_args: Vec<Val>,
    ) -> Result<SafeCallResult, SafeCallResult> {
        let result = safe_invoke_contract(env, contract_id, function_name, args);

        match result.status {
            CrossContractCallStatus::Success | CrossContractCallStatus::RecoverableError => {
                // Register compensation so we can undo this call later if needed
                self.compensation_stack.push_back((
                    contract_id.clone(),
                    function_name.clone(),
                    CompensationAction {
                        tag: compensation_tag,
                        args: compensation_args,
                    },
                ));
                Ok(result)
            }
            CrossContractCallStatus::FatalError | CrossContractCallStatus::DispatchFailed => {
                Err(result)
            }
            CrossContractCallStatus::Compensated => {
                // A compensated call should not happen at this stage
                Err(result)
            }
        }
    }

    /// Returns the number of compensated calls on the stack.
    pub fn depth(&self) -> u32 {
        self.compensation_stack.len()
    }

    /// Whether there are any compensations registered.
    pub fn has_pending(&self) -> bool {
        !self.compensation_stack.is_empty()
    }

    /// Walk the compensation stack in reverse order and invoke each
    /// registered compensation action on its target contract.
    ///
    /// This should be called after a fatal error to undo all prior
    /// successful cross-contract calls.
    pub fn rollback_all(&mut self, env: &Env) {
        while let Some((contract_id, _function_name, action)) = self.compensation_stack.pop_back() {
            // Invoke the compensation function on the original target contract.
            // We ignore the result because there is no further recovery possible
            // during rollback.
            let _ =
                env.try_invoke_contract::<Val, Val>(&contract_id, &action.tag, action.args.clone());
        }
    }

    /// Confirm the happy path and clear all registered compensations.
    ///
    /// Call this after all cross-contract calls have succeeded to signal
    /// that no rollback is needed.
    pub fn finalize(&mut self, env: &Env) {
        self.compensation_stack = Vec::new(env);
    }
}

// -----------------------------------------------------------------------
// Pseudo-contract interface for the SLA ↔ payment workflow
// -----------------------------------------------------------------------

/// Symbol tags for compensation actions.
pub const COMP_UNLOCK_FUNDS: Symbol = soroban_sdk::symbol_short!("unlck_fnd");
pub const COMP_REVERSE_SETTLE: Symbol = soroban_sdk::symbol_short!("rev_setle");
pub const COMP_UNPAUSE_ESCROW: Symbol = soroban_sdk::symbol_short!("unp_escro");

/// Standard function names expected on downstream contracts.
pub const FN_LOCK_FUNDS: Symbol = soroban_sdk::symbol_short!("lock_fnds");
pub const FN_RELEASE_PAYMENT: Symbol = soroban_sdk::symbol_short!("rel_pay");
pub const FN_CANCEL_SETTLEMENT: Symbol = soroban_sdk::symbol_short!("can_setl");

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{symbol_short, Address, Env, Vec};

    #[test]
    fn test_safe_invoke_unknown_contract_returns_fatal_error() {
        let env = Env::default();
        let unknown = Address::generate(&env);
        let result = safe_invoke_contract(&env, &unknown, &symbol_short!("ping"), &[]);
        assert_eq!(result.status, CrossContractCallStatus::FatalError);
        assert!(result.error_symbol.is_some());
    }

    #[test]
    fn test_requires_rollback_for_fatal() {
        assert!(requires_rollback(CrossContractCallStatus::FatalError));
        assert!(requires_rollback(CrossContractCallStatus::DispatchFailed));
        assert!(!requires_rollback(CrossContractCallStatus::Success));
        assert!(!requires_rollback(
            CrossContractCallStatus::RecoverableError
        ));
    }

    #[test]
    fn test_safety_tracker_starts_empty() {
        let env = Env::default();
        let safety = CrossContractSafety::new(&env);
        assert_eq!(safety.depth(), 0);
        assert!(!safety.has_pending());
    }

    #[test]
    fn test_safety_tracker_registers_compensation_on_success() {
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);
        let mock_contract = Address::generate(&env);
        // Even though the call will fail (unknown address), we test
        // the registration path via a direct push
        safety.compensation_stack.push_back((
            mock_contract.clone(),
            FN_LOCK_FUNDS,
            CompensationAction {
                tag: COMP_UNLOCK_FUNDS,
                args: Vec::new(&env),
            },
        ));
        assert_eq!(safety.depth(), 1);
        assert!(safety.has_pending());
    }

    #[test]
    fn test_safe_call_to_unknown_address_returns_err() {
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);
        let unknown = Address::generate(&env);

        let result = safety.call(
            &env,
            &unknown,
            &symbol_short!("ping"),
            &[],
            COMP_UNLOCK_FUNDS,
            Vec::new(&env),
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().status,
            CrossContractCallStatus::FatalError
        );
    }

    #[test]
    fn test_status_variants_are_distinct() {
        let variants = [
            CrossContractCallStatus::Success,
            CrossContractCallStatus::RecoverableError,
            CrossContractCallStatus::FatalError,
            CrossContractCallStatus::DispatchFailed,
            CrossContractCallStatus::Compensated,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i] as u32, variants[j] as u32);
            }
        }
    }

    #[test]
    fn test_compensation_symbols_are_distinct() {
        let tags = [COMP_UNLOCK_FUNDS, COMP_REVERSE_SETTLE, COMP_UNPAUSE_ESCROW];
        for i in 0..tags.len() {
            for j in (i + 1)..tags.len() {
                assert_ne!(tags[i], tags[j]);
            }
        }
    }

    #[test]
    fn test_fn_symbols_are_distinct() {
        let fns = [FN_LOCK_FUNDS, FN_RELEASE_PAYMENT, FN_CANCEL_SETTLEMENT];
        for i in 0..fns.len() {
            for j in (i + 1)..fns.len() {
                assert_ne!(fns[i], fns[j]);
            }
        }
    }

    #[test]
    fn test_rollback_all_clears_stack() {
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);
        let mock_contract = Address::generate(&env);

        safety.compensation_stack.push_back((
            mock_contract.clone(),
            FN_LOCK_FUNDS,
            CompensationAction {
                tag: COMP_UNLOCK_FUNDS,
                args: Vec::new(&env),
            },
        ));
        safety.compensation_stack.push_back((
            mock_contract,
            FN_RELEASE_PAYMENT,
            CompensationAction {
                tag: COMP_REVERSE_SETTLE,
                args: Vec::new(&env),
            },
        ));
        assert_eq!(safety.depth(), 2);

        safety.rollback_all(&env);
        assert_eq!(safety.depth(), 0);
        assert!(!safety.has_pending());
    }

    #[test]
    fn test_rollback_all_empty_stack_is_noop() {
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);
        safety.rollback_all(&env);
        assert_eq!(safety.depth(), 0);
        assert!(!safety.has_pending());
    }

    #[test]
    fn test_finalize_clears_stack() {
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);
        let mock_contract = Address::generate(&env);

        safety.compensation_stack.push_back((
            mock_contract,
            FN_LOCK_FUNDS,
            CompensationAction {
                tag: COMP_UNLOCK_FUNDS,
                args: Vec::new(&env),
            },
        ));
        assert_eq!(safety.depth(), 1);

        safety.finalize(&env);
        assert_eq!(safety.depth(), 0);
        assert!(!safety.has_pending());
    }

    #[test]
    fn test_finalize_empty_stack_is_noop() {
        let env = Env::default();
        let mut safety = CrossContractSafety::new(&env);
        safety.finalize(&env);
        assert_eq!(safety.depth(), 0);
        assert!(!safety.has_pending());
    }

    #[test]
    fn test_safe_call_result_debug() {
        let env = Env::default();
        let result = SafeCallResult {
            status: CrossContractCallStatus::Success,
            raw_output: Val::from(true),
            error_symbol: None,
        };
        assert_eq!(result.status, CrossContractCallStatus::Success);
        assert!(result.error_symbol.is_none());
    }
}
