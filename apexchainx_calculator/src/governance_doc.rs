//! Authoritative governance module documentation (Issue #652).
//!
//! The `governance.rs` file previously opened with two nearly identical module
//! doc comment blocks describing the same two-step governance design, doubling
//! maintenance and confusing doc-tooling.
//!
//! This module contains the single merged reference. The content of `governance.rs`
//! should carry only one module-level doc block; this file records the canonical
//! merged text for review and future reference.
//!
//! # Governance Overview
//!
//! The contract implements a two-step handoff pattern for admin and operator
//! role transfers:
//!
//! 1. The current role-holder **proposes** a new candidate.
//! 2. The candidate **accepts** to complete the transfer.
//!
//! Proposals expire after `PROPOSAL_EXPIRY_WINDOW` seconds if not accepted.
//! The expiry is lazy and observable: the first accept attempt after expiry
//! emits a typed expiry event and clears the pending keys.
//!
//! ## Role transitions
//!
//! | Action | Guard | Event |
//! |---|---|---|
//! | `propose_admin` | Admin, not paused, not frozen | `adm_prop` |
//! | `accept_admin` | Proposed admin | `adm_acc` |
//! | `cancel_admin_proposal` | Admin | `adm_can` |
//! | `renounce_admin` | Admin, not frozen | `adm_ren` |
//! | `propose_operator` | Admin, not paused, not frozen | `op_prop` |
//! | `accept_operator` | Proposed operator | `op_acc` |
//! | `set_operator` | Admin (direct, non-consensual) | `op_set` |
//!
//! See `docs/GOVERNANCE_REFERENCE.md` for the full state-machine diagram
//! and `docs/OPERATOR_ROTATION_GUIDE.md` for rotation semantics.

// This file is documentation-only; no runtime code.