/**
 * SC-017 / SC-W5-029: Event-size regression tests for lifecycle, SLA calculation,
 * and version negotiation events.
 * Catches payload bloat before deployment by asserting max byte sizes per event type.
 *
 * Event names and payload shapes match the on-chain ABI defined in
 * apexchainx_calculator/src/event_schema.rs. Size limits are derived from
 * Soroban SCVal encoding (not JSON serialisation), because on-chain event
 * payloads are SCVal-encoded byte vectors.
 */

// ─── Soroban SCVal encoding widths ─────────────────────────────────────────
// These mirror the byte-width rules the Soroban host uses when encoding
// SCVal, not JSON serialisation:
//   - Vec header:  4 bytes (length prefix)
//   - Symbol:      4 bytes overhead + length, capped at the 9-byte short-Symbol limit
//   - u32 / bool:  4 bytes (bool is encoded as SCVal::Bool -> u32 0/1)
//   - i128 / u64:  16 bytes (u64 is promoted to i128 width on the wire)
//   - Address:     32 bytes (ed25519 public key), independent of its string length
const VEC_HEADER_BYTES = 4;
const SYMBOL_OVERHEAD_BYTES = 4;
const SYMBOL_MAX_LEN = 9;
const SCALAR_BYTES = {
  u32: 4,
  bool: 4,
  i128: 16,
  u64: 16,
  address: 32,
} as const;

// Soroban/Stellar strkey addresses are always exactly 56 characters.
// This only guards against malformed *test fixtures* -- it has no effect
// on the size estimate itself, since Address always costs a fixed 32 bytes
// on the wire regardless of its string representation.
const STRKEY_ADDRESS_RE = /^[A-Z0-9]{56}$/;

/**
 * Estimated SCVal-encoded byte sizes for each on-chain event payload.
 * Recompute the "≈ Nb" comment by hand whenever event_schema.rs changes
 * so the constant and the derivation stay in sync.
 */
const EVENT_SIZE_LIMITS = {
  // sla_calc: (outage_id: Symbol, status: Symbol, payment_type: Symbol,
  //            rating: Symbol, mttr_minutes: u32, threshold_minutes: u32, amount: i128)
  // ≈ 4 + (4+7) + (4+3) + (4+3) + (4+4) + 4 + 4 + 16 = 61 bytes
  sla_calc: 72,
  // paused: (true,) — single bool
  paused: 12,
  // unpause: (false,) — single bool
  unpause: 12,
  // adm_prop: (new_admin: Address,)
  adm_prop: 40,
  // op_prop: (new_operator: Address,)
  op_prop: 40,
  // cfg_upd: (threshold_minutes: u32, penalty_per_minute: i128, reward_base: i128)
  cfg_upd: 40,
  // set_int: (outage_id: Symbol, status: Symbol, payment_type: Symbol,
  //           amount: i128, config_version_hash: u64, recorded_at: u64)
  set_int: 56,
} as const;

/** Event types are derived from the limits table, so a typo in a fixture's
 *  `type` field is a compile error instead of a silent runtime miss. */
type EventType = keyof typeof EVENT_SIZE_LIMITS;

type ScValKind = "symbol" | "u32" | "i128" | "u64" | "bool" | "address";

/**
 * A single SCVal tuple field. Fields are explicitly tagged with their
 * on-chain `kind` rather than inferred from the JS `typeof` of the value.
 *
 * This matters: inferring from `typeof` alone can't tell a Symbol string
 * from an Address string, and can't tell a small i128 amount (e.g. `750`)
 * from a u32 — both are plain JS numbers, but they cost 4 bytes vs 16 bytes
 * on the wire. Getting this wrong makes the regression check *under*-report
 * size, which defeats its entire purpose.
 */
interface ScValField {
  kind: ScValKind;
  value: string | number | boolean | bigint;
}

interface ContractEvent {
  type: EventType;
  /** Payload fields, in on-chain tuple order, matching event_schema.rs. */
  payload: ScValField[];
}

/** Estimates the Soroban SCVal-encoded byte size of an event payload. */
function estimateScValSize(fields: ScValField[]): number {
  let size = VEC_HEADER_BYTES;

  for (const field of fields) {
    switch (field.kind) {
      case "symbol": {
        const len = String(field.value).length;
        size += SYMBOL_OVERHEAD_BYTES + Math.min(len, SYMBOL_MAX_LEN);
        break;
      }
      case "address": {
        if (typeof field.value === "string" && !STRKEY_ADDRESS_RE.test(field.value)) {
          throw new Error(
            `Invalid address fixture "${field.value}": expected a 56-character strkey address`,
          );
        }
        size += SCALAR_BYTES.address;
        break;
      }
      case "bool":
        size += SCALAR_BYTES.bool;
        break;
      case "u32":
        size += SCALAR_BYTES.u32;
        break;
      case "i128":
      case "u64":
        size += SCALAR_BYTES.i128;
        break;
      default: {
        // Exhaustiveness check: fails to compile if a ScValKind is added
        // above without being handled here.
        const _exhaustive: never = field.kind;
        throw new Error(`Unhandled SCVal kind: ${_exhaustive}`);
      }
    }
  }

  return size;
}

interface CheckResult {
  type: EventType;
  size: number;
  limit: number;
  passed: boolean;
}

function checkEventSize(event: ContractEvent): CheckResult {
  const limit = EVENT_SIZE_LIMITS[event.type];
  const size = estimateScValSize(event.payload);
  return { type: event.type, size, limit, passed: size <= limit };
}

/** Runs every check and returns the full result set, rather than throwing
 *  on the first failure -- so a single CI run surfaces every regression
 *  at once instead of one-per-rerun. */
function runChecks(events: ContractEvent[]): CheckResult[] {
  return events.map(checkEventSize);
}

function report(results: CheckResult[]): void {
  console.log("[SC-017] Event-size regression checks (Soroban SCVal estimation):");
  for (const r of results) {
    const mark = r.passed ? "✓" : "✗";
    console.log(`  ${mark} ${r.type}: ${r.size}B / ${r.limit}B`);
  }

  const failures = results.filter((r) => !r.passed);
  if (failures.length > 0) {
    console.error(`\n${failures.length} event(s) exceeded their size budget:`);
    for (const f of failures) {
      console.error(`  - ${f.type}: ${f.size}B exceeds the ${f.limit}B limit by ${f.size - f.limit}B`);
    }
  } else {
    console.log("\nAll event-size checks passed.");
  }
}

// ─── Test fixtures matching the on-chain ABI ───────────────────────────────
// Payload shapes and field order match the schemas in event_schema.rs.
// Address fixtures are padded to a realistic 56-character strkey length.
const events: ContractEvent[] = [
  {
    type: "sla_calc",
    payload: [
      { kind: "symbol", value: "out_001" },
      { kind: "symbol", value: "met" },
      { kind: "symbol", value: "rew" },
      { kind: "symbol", value: "good" },
      { kind: "u32", value: 45 },
      { kind: "u32", value: 60 },
      { kind: "i128", value: 750 },
    ],
  },
  {
    type: "paused",
    payload: [{ kind: "bool", value: true }],
  },
  {
    type: "unpause",
    payload: [{ kind: "bool", value: false }],
  },
  {
    type: "adm_prop",
    payload: [
      { kind: "address", value: "GABC123DEF456GHI789JKL012MNO345PQR678STU901VWX234YZA567X" },
    ],
  },
  {
    type: "op_prop",
    payload: [
      { kind: "address", value: "GDEF789GHI012JKL345MNO678PQR901STU234VWX567YZA890BCD123X" },
    ],
  },
  {
    type: "cfg_upd",
    payload: [
      { kind: "u32", value: 60 },
      { kind: "i128", value: 100 },
      { kind: "i128", value: 750 },
    ],
  },
  {
    type: "set_int",
    payload: [
      { kind: "symbol", value: "out_001" },
      { kind: "symbol", value: "met" },
      { kind: "symbol", value: "rew" },
      { kind: "i128", value: 750 },
      { kind: "u64", value: 0 },
      { kind: "u64", value: 0 },
    ],
  },
];

const results = runChecks(events);
report(results);

// Fail the process (and therefore CI) without throwing away the full
// report above -- `assertEventSize` used to throw on the first offender,
// which meant every other event's status was left unknown.
if (results.some((r) => !r.passed)) {
  process.exitCode = 1;
}