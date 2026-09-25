compute_config_version_hash should include custom severity bounds
Repo Avatar
ApexChainx/ApexChainx-Contracts
Context
Custom severities influence recalculation policy through the version hash, but the hash computation excludes them entirely (lib.rs compute_config_version_hash covers CONFIG_KEY canonical four).

Problem
Beyond outright exclusion (see related issue), a subtler gap remains even after hashing is fixed: hashing the custom map at byte level will churn on any custom edit, and the ordering of the custom map must be canonicalized. Without a deterministic ordering, the same custom config serialized in two insertion orders hashes differently, breaking the replay contract. Hashing needs normalization rules for the custom tier, not just inclusion.

Proposed approach
Include an ordered (sorted-by-symbol) serialization of CUSTOM_CONFIG_KEY in the hash, mirror the canonical-four handling, and add tests proving insertion-order invariance.

Acceptance criteria
Custom config feeds the hash deterministically.
Insertion order does not change the hash.
Replay/duplicate policy stays stable.