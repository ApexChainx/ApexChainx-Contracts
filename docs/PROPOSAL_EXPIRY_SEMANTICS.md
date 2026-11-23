# Proposal Expiry Semantics

> **Issue:** [#661](https://github.com/ApexChainx/ApexChainx-Contracts/issues/661)
> **Applies to:** `governance.rs` — `PROPOSAL_EXPIRY_WINDOW`, `require_proposal_valid`

## The choice: calendar seconds vs. ledger sequence numbers

The governance proposal expiry window is implemented using **ledger timestamps
(calendar seconds)** rather than ledger sequence numbers. This document records
the rationale, the tradeoffs, and the operator guidance.

---

## Current implementation

```rust
// governance.rs
const PROPOSAL_EXPIRY_WINDOW: u64 = 90 * 24 * 60 * 60; // 90 days in seconds

fn require_proposal_valid(env: &Env, ts_key: Symbol) -> Result<(), SLAError> {
    let proposed: u64 = env.storage().instance().get(&ts_key)...;
    let now = env.ledger().timestamp();
    if now.saturating_sub(proposed) > PROPOSAL_EXPIRY_WINDOW {
        return Err(SLAError::ProposalExpired);
    }
    Ok(())
}
```

Both the proposal timestamp and the expiry check use `env.ledger().timestamp()`,
which is a Unix-epoch second value set by validators when the ledger closes.

---

## Tradeoff table

| Dimension | Calendar seconds (current) | Ledger sequence numbers (alternative) |
|---|---|---|
| **Portability** | Differs across networks with different ledger cadences: a 90-day window on mainnet is the same wall-clock time on a fast testnet that closes ledgers every 1s instead of every 5s. | A fixed number of ledgers represents different wall-clock durations across networks. |
| **Operator predictability** | An operator knows the proposal expires in 90 real-world days regardless of network speed. | An operator must convert ledger count × ledger close time to estimate wall-clock duration — network-specific. |
| **Clock skew risk** | Ledger timestamps can drift if validators disagree on clock; `env.ledger().timestamp()` is the consensus value, so skew is bounded by the network protocol. | Ledger sequence numbers are monotonically increasing with no skew. |
| **Test compatibility** | Existing tests use `env.ledger().with_mut(|li| li.timestamp += N)` to simulate time passage — directly compatible. | Tests would use sequence number advancement instead. |

---

## Decision: retain calendar-seconds semantics

The current implementation is **intentionally using calendar seconds** because:

1. **Operator intent is wall-clock**: "this proposal expires in 90 days" is
   unambiguous to a compliance operator regardless of the deployment network.
2. **Cross-network consistency**: the same governance policy applies whether
   deployed to mainnet, testnet, or a custom network with different ledger rates.
3. **Test infrastructure alignment**: all existing governance tests mock
   `timestamp` directly; switching to sequence numbers would require rebuilding
   the test harness.

---

## Operator guidance

- A pending admin or operator proposal created at ledger timestamp `T` will
  expire at `T + 7_776_000` seconds (90 days).
- The expiry is **lazy and observable**: it is only detected on the first
  `accept_*` call after the window lapses, at which point the expiry event
  (`adm_xp` / `op_xp`) is emitted and the pending keys are cleared.
- On networks with non-standard ledger cadences, the **wall-clock duration**
  of the window is unchanged; only the number of ledgers elapsed differs.
- If a shorter or longer window is required for a specific deployment, update
  `PROPOSAL_EXPIRY_WINDOW` and document the change in CHANGELOG.md and this
  file.

---

## Changing the semantics (future)

If the project decides to switch to ledger-sequence expiry:

1. Replace `PENDING_ADMIN_TS_KEY` / `PENDING_OP_TS_KEY` storage values with
   ledger sequence numbers (migration required).
2. Update `require_proposal_valid` to compare `env.ledger().sequence()`.
3. Update `PROPOSAL_EXPIRY_WINDOW` to a ledger count rather than seconds.
4. Update governance tests to advance `env.ledger().sequence()` instead of
   `env.ledger().timestamp()`.
5. Add a storage-version bump and migration entry.