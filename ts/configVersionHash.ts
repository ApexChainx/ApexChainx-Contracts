/**
 * SC-009 — Config version hash (#129)
 *
 * Off-chain mirror of the contract's `get_config_version_hash`
 * (`SLACalculatorContract::compute_config_version_hash`).
 *
 * # What changed and why
 *
 * This file used to compute a djb2 hash over a canonical JSON serialisation of
 * a snapshot whose fields (`penaltyBps`, `rewardBps`) did not even exist on the
 * contract. It was never the contract's hash — it was a second, unrelated
 * fingerprint that happened to share a name. A backend comparing its value
 * against an on-chain `get_config_version_hash` would have seen a mismatch on
 * every single call and concluded the config had drifted when nothing had.
 *
 * [`configVersionHash`] now reproduces the contract's algorithm exactly: a
 * polynomial rolling hash over the four canonical severity configs, in
 * canonical order (critical → high → medium → low), reducing modulo a prime
 * near 2^63 with `u64` wrapping arithmetic throughout.
 *
 * # Consumer guidance
 *
 * Compare hashes for equality only. The value is a change detector, not an
 * ordering or a checksum, and its digits carry no meaning. Two snapshots with
 * the same hash are the same config; a changed hash means at least one of the
 * twelve config fields moved.
 *
 * # Arithmetic
 *
 * The contract works in `u64`. JavaScript numbers lose precision above 2^53, so
 * every intermediate here is a `BigInt` masked back to 64 bits with
 * `BigInt.asUintN(64, …)` — the direct equivalent of Rust's `wrapping_mul` /
 * `wrapping_add`. The result is returned as a `bigint`; stringify it before
 * putting it in JSON.
 *
 * Verified against contract-recorded output by
 * `ts/parity/readSemanticsParity.test.ts`.
 */

import { CANONICAL_SEVERITIES } from "./contractSemantics";

/** One severity's stored configuration, named as the contract names them. */
export interface SlaConfig {
  severity: string;
  thresholdMinutes: number;
  penaltyPerMinute: bigint;
  rewardBase: bigint;
}

/**
 * The four canonical configs. Order in the array is irrelevant — the hash
 * always walks them in canonical severity order, exactly as the contract does.
 */
export interface ConfigSnapshot {
  configs: SlaConfig[];
}

/** Polynomial base, matching `BASE` in `compute_config_version_hash`. */
const BASE = 91_138_233n;
/** Modulus `2^63 - 25`, matching `MODULUS`. */
const MODULUS = (1n << 63n) - 25n;
/** Final avalanche constant applied after the last severity. */
const FINAL_MIX = 0x9e3779b97f4a7c15n;

/** Rust's `u64` wrapping semantics: keep the low 64 bits. */
const u64 = (value: bigint): bigint => BigInt.asUintN(64, value);

/**
 * Computes the contract's config version hash for a snapshot.
 *
 * @throws if the snapshot does not contain exactly one config per canonical
 * severity. The contract loads all four unconditionally and fails the whole
 * call if one is missing, so a partial snapshot has no defined hash and
 * silently hashing three configs would produce a plausible-looking wrong
 * answer.
 */
export function configVersionHash(snapshot: ConfigSnapshot): bigint {
  const bySeverity = new Map(snapshot.configs.map((c) => [c.severity, c]));

  let hash = 1n;
  let power = 1n;

  for (const severity of CANONICAL_SEVERITIES) {
    const config = bySeverity.get(severity);
    if (config === undefined) {
      throw new Error(
        `config snapshot is missing canonical severity "${severity}"; ` +
          `the contract hashes all four and cannot hash a partial snapshot`,
      );
    }

    // Each field: hash = ((hash * BASE) + field) * power, mod MODULUS, with
    // every intermediate wrapped to u64 first — the multiplication order and
    // the interleaved `power` advance both matter.
    for (const field of [
      BigInt(config.thresholdMinutes),
      config.penaltyPerMinute,
      config.rewardBase,
    ]) {
      // `as u64` in Rust truncates an `i128` to its low 64 bits.
      hash = u64(u64(u64(hash * BASE) + u64(field)) * power) % MODULUS;
      power = u64(power * BASE) % MODULUS;
    }
  }

  return u64(u64(hash * BASE) + FINAL_MIX) % MODULUS;
}
