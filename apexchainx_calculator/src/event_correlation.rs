//! Correlation IDs identify an (outage, ledger) pair, not a submission.
//!
//! Big-endian layout: ledger sequence in bytes 0..4, FNV-1a-32 of the raw
//! outage symbol in bytes 4..8. Different ledgers ALWAYS yield different IDs.
//! Repeating a pair yields the same ID. Within one ledger fingerprints CAN
//! collide: this is a tracing hint, never an authorization or deduplication key.
//! Scope joins by network, contract, ledger and full outage ID.
//! This v2 encoding replaces FNV-1a-64; decode historical IDs by event version.
//! The SLA contract carries the ID in the trailing `set_int` payload field,
//! retaining its three topics (name, per-name version, context).

use soroban_sdk::{Env, Symbol, SymbolStr, TryFromVal};

/// A ledger-bound tracing hint, with possible within-ledger hash collisions.
pub type CorrelationId = u64;
/// Encode the supplied ledger, independently of the environment's current ledger.
/// Fingerprint: FNV-1a-32, offset 2166136261, prime 16777619, raw symbol bytes.
pub fn generate_correlation_id(env: &Env, outage_id: &Symbol, ledger_sequence: u32) -> CorrelationId {
    let outage: SymbolStr = SymbolStr::try_from_val(env, &outage_id.to_symbol_val())
        .expect("outage id symbol must be well-formed");
    let bytes: &[u8] = outage.as_ref();
    let mut fingerprint = 0x811c9dc5u32;
    for byte in bytes {
        fingerprint ^= u32::from(*byte);
        fingerprint = fingerprint.wrapping_mul(0x01000193);
    }
    (u64::from(ledger_sequence) << 32) | u64::from(fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use soroban_sdk::symbol_short;

    #[test]
    fn ledger_is_structural_and_repeated_submissions_are_identical() {
        let env = Env::default();
        for name in ["", "OUT001", "INC_2024_03_15_ABCDEF123456"] {
            let outage = Symbol::new(&env, name);
            for ledger in [0, 1, 42, u32::MAX] {
                let id = generate_correlation_id(&env, &outage, ledger);
                assert_eq!((id >> 32) as u32, ledger);
                assert_eq!(id, generate_correlation_id(&env, &outage, ledger));
                assert_eq!(id as u32, generate_correlation_id(&env, &outage, 0) as u32);
            }
        }
        assert_eq!(
            generate_correlation_id(&env, &Symbol::new(&env, "hello"), 42),
            0x0000002a4f9f2cab
        );
        assert_ne!(
            generate_correlation_id(&env, &Symbol::new(&env, "OUT_A"), 42),
            generate_correlation_id(&env, &Symbol::new(&env, "OUT_B"), 42)
        );
    }

    proptest! {
        #[test]
        fn distinct_ledgers_never_collide(a in any::<u32>(), b in any::<u32>()) {
            prop_assume!(a != b);
            let env = Env::default();
            let first = generate_correlation_id(&env, &symbol_short!("OUT_A"), a);
            let second = generate_correlation_id(&env, &symbol_short!("OUT_B"), b);
            prop_assert_ne!(first, second);
            prop_assert_eq!((first >> 32) as u32, a);
            prop_assert_eq!((second >> 32) as u32, b);
        }
    }
}
