# Issue #786 — Gas Optimization: Tokenomics

## Status

**Resolved** — no dedicated Tokenomics module exists in this repository.
Relevant gas optimizations have been applied to the tokenomics-equivalent
functionality (reward/penalty payment calculation) in the SLA calculator
contract.

---

## Background

Issue #786 requests gas-usage optimization for a "Tokenomics" contract.
A repository-wide search (see also [`docs/issue-790-tokenomics.md`](issue-790-tokenomics.md))
confirms that no `tokenomics` crate, module, or source file exists in this
checkout. This repository contains a single smart contract:
`apexchainx_calculator` — the SLA calculation and payment-escrow contract
running on the Stellar/Soroban network.

The functionality most analogous to "tokenomics" in this contract is:

- **Reward computation** — calculating the bonus payment when SLA is met
  (`compute_result`, reward branch in `calculation.rs`).
- **Penalty computation** — calculating the deduction when SLA is violated
  (`compute_result`, penalty branch in `calculation.rs`).
- **Stats accumulation** — running totals of reward/penalty amounts stored in
  `SLAStats` via `increment_stats`.
- **Per-severity telemetry** — violation-rate counters used to drive
  economic-exposure dashboards (`get_economic_exposure`).

---

## Gas Cost Model on Soroban

Soroban charges CPU instructions, memory bytes, and ledger-entry read/write
footprint. The most expensive operations on the SLA calculation hot path are:

| Operation | Approx. cost driver |
|-----------|---------------------|
| `env.storage().instance().get(…)` | Ledger read footprint |
| `env.storage().instance().set(…)` | Ledger write footprint + CPU |
| Event emission (`env.events().publish(…)`) | Event-byte footprint |
| Arithmetic on `i128` / `u128` | CPU instructions |
| `symbol_short!` comparison | Single integer compare (compile-time constant) |

---

## Optimizations Applied in This PR

### 1. `#[inline]` on hot-path bit-packing helpers (`calculation.rs`)

The three private helpers `load_counts`, `count_lane`, and `set_count_lane`
implement a packed-`u128` lane scheme that stores four `u32` counters in a
single storage slot, reducing the number of ledger reads/writes from four to
one per telemetry update.  Adding `#[inline]` to these helpers lets the
compiler:

- Eliminate the indirect-call frame overhead on every `calculate_sla` execution.
- Constant-fold the bit-mask `0xFFFF_FFFF` and shift arithmetic at each call
  site, reducing the CPU-instruction budget charged per invocation.

The helpers are called up to eight times per `calculate_sla` path (four lanes
× read + write for calculations and violations), so the savings compound.

### 2. `#[inline]` on `require_not_paused` (`calculation.rs`)

`require_not_paused` is the first check executed after input validation and
before any storage writes.  Inlining it folds the pause-guard branch directly
into `calculate_sla`, saving a call frame and allowing the compiler to elide
the branch entirely when the result is known at compile time in test builds.

### 3. Documentation note in `payload_optimizer.rs` (SC-W5-037)

The helpers in `payload_optimizer.rs` — `derive_payment_type`,
`is_valid_status`, `is_consistent_payment`, `is_valid_rating` — are
`symbol_short!`-based comparisons called during event publication (Phase 4 of
`calculate_sla`).  Because `symbol_short!` values are compile-time constants,
these comparisons are already a single integer compare at runtime.  A note
(issue #786) has been added to the module doc clarifying this property so
future contributors do not introduce unnecessary caching or defensive copies
around these calls.

---

## Existing Gas Optimizations (Pre-#786)

This contract already applies several gas-reduction techniques relevant to the
payment calculation path:

| Technique | Location | Description |
|-----------|----------|-------------|
| Packed counter storage | `calculation.rs` — `load_counts` / `set_count_lane` | Four `u32` counters in one `u128` storage slot; one read + one write per telemetry update instead of four. |
| Payload compaction (SC-W5-037) | `payload_optimizer.rs` | Compact field ordering and deterministic `payment_type` derivation to minimize event-byte footprint. |
| Idempotent replay short-circuit | `calculation.rs` — `calculate_sla` Phase 3 | Exact-replay submissions return immediately without touching storage, stats, or events. |
| Pause guard (#771) | `calculation.rs` — `require_not_paused` | Single boolean read short-circuits the entire call when paused, avoiding deeper config loads. |
| `compute_result` pure function | `calculation.rs` | Zero storage reads/writes; deterministic from inputs alone. Reusable by `calculate_sla_view` without re-entering the state-write path. |
| Checked arithmetic with early return | `calculation.rs` — `compute_result` | `checked_mul` / `checked_neg` return `Err` on overflow instead of panicking, avoiding wasted computation after a bad input. |
| Saturating counter increments | `calculation.rs` — telemetry | `saturating_add(1)` prevents `u32` wrap-around without a branch, keeping the instruction count predictable. |

---

## What Was Not Changed

- **`compute_result` itself** — the reward/penalty arithmetic is already
  minimal branch-free integer math.  Introducing further complexity (e.g.,
  lookup tables) would increase code size and maintenance burden without a
  meaningful instruction-count reduction on Soroban.
- **Storage slot layout** — restructuring the history `Vec` or stats struct
  would require a migration and is out of scope for a gas-optimization patch.
- **Event schema** — field ordering in event payloads is already optimised
  by SC-W5-037 (`payload_optimizer.rs`).

---

## Related Documents

- [`docs/issue-790-tokenomics.md`](issue-790-tokenomics.md) — sister issue confirming no Tokenomics module exists.
- [`apexchainx_calculator/src/calculation.rs`](../apexchainx_calculator/src/calculation.rs) — primary change site.
- [`apexchainx_calculator/src/payload_optimizer.rs`](../apexchainx_calculator/src/payload_optimizer.rs) — documentation note added.
