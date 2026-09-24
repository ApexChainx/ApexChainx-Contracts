//! #602 – Storage-key addition co-bump invariant.
//!
//! The reserved instance-storage key set is part of the storage schema: a key
//! added to the on-chain layout (as `HISTORY_LEN_KEY` was at v2) changes the
//! layout that backends cache and migrate on, so it must never ship on an
//! unchanged `STORAGE_VERSION`. This module pins the key set observed at each
//! `STORAGE_VERSION` and fails CI when the current set diverges from the
//! snapshot of that version without also carrying a version bump.

use crate::STORAGE_VERSION;

/// The reserved instance-storage key set pinned to the storage schema as of
/// `SNAPSHOT_STORAGE_VERSION`. This is the historical record of what each
/// migration step added: `HISTORY_LEN_KEY` at v2, `CONFIG_COUNT_KEY` at v3.
// Retained as an audit record for the v2 schema; the live invariant compares
// against the v3 snapshot below.
#[allow(dead_code)]
const KEYS_AT_V2: [&str; 22] = [
    "ADMIN",
    "OPERATOR",
    "PADMIN",
    "POP",
    "PADMINTS",
    "POPTS",
    "CONFIG",
    "CUSTCFG",
    "PAUSED",
    "PAUSEINF",
    "STATS",
    "CALCCNT",
    "VIOLCNT",
    "CALCTS",
    "VIOLTS",
    "HIST",
    "HISTLEN",
    "VER",
    "RETLIM",
    "TPRUNED",
    "TTOTENT",
    "LCFGUPD",
];

/// The reserved instance-storage key set as of `STORAGE_VERSION` 3, which adds
/// `CONFIG_COUNT_KEY` (cached config count for issue #606).
const KEYS_AT_V3: [&str; 23] = [
    "ADMIN",
    "OPERATOR",
    "PADMIN",
    "POP",
    "PADMINTS",
    "POPTS",
    "CONFIG",
    "CUSTCFG",
    "PAUSED",
    "PAUSEINF",
    "STATS",
    "CALCCNT",
    "VIOLCNT",
    "CALCTS",
    "VIOLTS",
    "HIST",
    "HISTLEN",
    "CFGCNT",
    "VER",
    "RETLIM",
    "TPRUNED",
    "TTOTENT",
    "LCFGUPD",
];

/// The `STORAGE_VERSION` the snapshot above was observed at.
const SNAPSHOT_STORAGE_VERSION: u32 = 3;

#[test]
fn test_storage_key_set_pinned_to_version_snapshot_or_newer() {
    // The invariant: while STORAGE_VERSION is unchanged, the key set must equal
    // the snapshot for that version. Adding or removing a key is a schema
    // change and MUST ship with a STORAGE_VERSION bump — otherwise backends
    // migrating on the storage version miss (or expect) a key that is not
    // there. Removal is allowed only under the same version-step rule, so the
    // check is exact equality against the snapshot, not just containment.
    let current = crate::api_stability::storage_key_symbols();

    let matches_snapshot = current.len() == KEYS_AT_V3.len()
        && KEYS_AT_V3.iter().all(|k| current.contains(k))
        && current.iter().all(|k| KEYS_AT_V3.contains(k));

    assert!(
        matches_snapshot || STORAGE_VERSION > SNAPSHOT_STORAGE_VERSION,
        "storage-key set diverged from the STORAGE_VERSION {} snapshot without a matching \
         STORAGE_VERSION bump. Adding/removing a reserved key is a storage-schema change: \
         bump STORAGE_VERSION in lib.rs in the same commit (and document the new key in \
         docs/RESERVED_KEYS_POLICY.md), then advance the snapshot here (#602).",
        SNAPSHOT_STORAGE_VERSION
    );
}