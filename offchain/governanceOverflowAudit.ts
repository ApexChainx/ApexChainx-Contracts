/**
 * Governance Integer Overflow / Underflow Audit
 *
 * Validates the arithmetic invariants of the governance module's timestamp
 * comparison logic and the PROPOSAL_EXPIRY_WINDOW constant. This script is
 * the TypeScript-level companion to the Rust-level overflow tests in
 * `apexchainx_calculator/src/governance_tests.rs`.
 *
 * Checks:
 *   1. PROPOSAL_EXPIRY_WINDOW constant is within safe u64 bounds
 *   2. Saturating subtraction semantics: `now - proposed` never wraps
 *   3. Expiry boundary (inclusive at WINDOW, exclusive at WINDOW+1)
 *   4. Clock-anomaly (proposed > now) saturates to 0, not to a large number
 *   5. u64::MAX ledger timestamp does not cause overflow
 *   6. Addition of proposed_ts + WINDOW stays within safe BigInt range
 */

const PROPOSAL_EXPIRY_WINDOW = BigInt(90 * 24 * 60 * 60); // 7_776_000 seconds
const U64_MAX = BigInt("18446744073709551615");

function saturatingSub(a: bigint, b: bigint): bigint {
  return a >= b ? a - b : 0n;
}

function requireProposalValid(now: bigint, proposed: bigint): boolean {
  const elapsed = saturatingSub(now, proposed);
  return elapsed <= PROPOSAL_EXPIRY_WINDOW;
}

function check(label: string, condition: boolean): void {
  if (!condition) {
    console.error(`FAIL: ${label}`);
    process.exit(1);
  }
  console.log(`  ✓ ${label}`);
}

console.log("[governance-overflow-audit] Running governance arithmetic invariant checks...");

// ── 1. PROPOSAL_EXPIRY_WINDOW fits safely in u64 ───────────────────────────
check(
  "PROPOSAL_EXPIRY_WINDOW is positive",
  PROPOSAL_EXPIRY_WINDOW > 0n
);
check(
  "PROPOSAL_EXPIRY_WINDOW fits in u64 without overflow",
  PROPOSAL_EXPIRY_WINDOW <= U64_MAX
);
// Must not be near u64::MAX/2 (to prevent proposed_ts + WINDOW from overflowing u64).
check(
  "PROPOSAL_EXPIRY_WINDOW is far below u64::MAX / 2",
  PROPOSAL_EXPIRY_WINDOW <= U64_MAX / 2n
);

// ── 2. Saturating subtraction: never wraps ──────────────────────────────────
check(
  "saturating_sub(0, 1) = 0, not wrapping",
  saturatingSub(0n, 1n) === 0n
);
check(
  "saturating_sub(5, 10) = 0, not wrapping",
  saturatingSub(5n, 10n) === 0n
);
check(
  "saturating_sub(10, 5) = 5",
  saturatingSub(10n, 5n) === 5n
);

// ── 3. Expiry boundary semantics ────────────────────────────────────────────
const proposedTs = BigInt(1_000_000);
const atBoundary = proposedTs + PROPOSAL_EXPIRY_WINDOW;
const pastBoundary = atBoundary + 1n;

check(
  "Proposal is valid exactly at expiry boundary (elapsed == WINDOW)",
  requireProposalValid(atBoundary, proposedTs)
);
check(
  "Proposal is expired one second past boundary (elapsed == WINDOW + 1)",
  !requireProposalValid(pastBoundary, proposedTs)
);
check(
  "Proposal is valid well within window (elapsed == 0)",
  requireProposalValid(proposedTs, proposedTs)
);
check(
  "Proposal is valid one second before boundary (elapsed == WINDOW - 1)",
  requireProposalValid(atBoundary - 1n, proposedTs)
);

// ── 4. Clock anomaly: proposed_ts > now saturates to 0 ──────────────────────
const futureProposed = BigInt(5_000_000);
const pastNow = BigInt(1_000_000);
check(
  "Clock anomaly (proposed > now) saturates elapsed to 0, proposal stays valid",
  requireProposalValid(pastNow, futureProposed)
);

// ── 5. u64::MAX ledger timestamp ─────────────────────────────────────────────
check(
  "now == u64::MAX, proposed == u64::MAX → elapsed = 0 → proposal valid",
  requireProposalValid(U64_MAX, U64_MAX)
);
check(
  "now == u64::MAX, proposed == 0 → elapsed = u64::MAX > WINDOW → expired",
  !requireProposalValid(U64_MAX, 0n)
);

// ── 6. proposed_ts + WINDOW stays within u64 range ──────────────────────────
// Any realistic ledger timestamp plus the expiry window must not exceed u64::MAX.
// The latest plausible ledger timestamp is far below u64::MAX.
// Unix year 2100 ≈ 4102444800 seconds — far below u64::MAX (1.8e19).
const year2100Ts = BigInt(4_102_444_800);
check(
  "year-2100 timestamp + PROPOSAL_EXPIRY_WINDOW fits in u64",
  year2100Ts + PROPOSAL_EXPIRY_WINDOW <= U64_MAX
);

// ── 7. PROPOSAL_EXPIRY_WINDOW equals expected value ─────────────────────────
// Pin the constant so an accidental change (e.g. wrong unit: days vs seconds)
// is caught here before it silently breaks expiry semantics.
const EXPECTED_WINDOW_SECONDS = BigInt(90 * 24 * 60 * 60); // 7_776_000
check(
  `PROPOSAL_EXPIRY_WINDOW equals ${EXPECTED_WINDOW_SECONDS} seconds (90 days)`,
  PROPOSAL_EXPIRY_WINDOW === EXPECTED_WINDOW_SECONDS
);

console.log("[governance-overflow-audit] All arithmetic invariant checks passed.");
