Problem
get_config_snapshot (src/lib.rs/src/config.rs) builds entries from canonical_severities only, and get_config_bundle composes it with get_result_schema:

pub fn get_config_snapshot(env: Env) -> Result<SLAConfigSnapshot, SLAError> {
    ...
    for severity in Self::canonical_severities(&env) { ... }
Custom severities live in a separate endpoint (get_custom_config_snapshot), which the bundle does not call. The bundle doc says it is "the recommended way for backend consumers to bootstrap their configuration cache in one RPC" and to "Use the snapshot for SLA evaluation parameters".

Consequences:

A backend bootstrapping from the bundle cannot evaluate custom-severity outages: calculate_sla accepts custom severities, but a consumer whose cache came from get_config_bundle has no parameters for them; first evaluation either fails with ConfigNotFound or, worse, uses a stale default.
The one-RPC bootstrap promise excludes a whole feature: #93 deployments must issue a second RPC (get_custom_config_snapshot) and stitch two snapshots with different schemas — the exact fragmentation the bundle exists to prevent.
The hash-based cache check compounds the gap: compute_config_version_hash ignores custom configs (companion issue), so a cache refreshed on hash change never updates for custom-severity edits.
Root cause
The bundle was built from the pre-#93 snapshot; the custom snapshot endpoint was added alongside instead of integrated.

Why this is architecturally hard
ConfigBundle is #[contracttype] and stability-guarded; adding a custom_snapshot field is additive (safe per the append-only rule) but requires updating the guardrail's canonical_field_counts (companion issue) and the bundle's consumers.
Alternatively, SLAConfigSnapshot itself could carry a custom_entries vector — a change to a widely-consumed type — so the design decision is where custom entries belong in the type hierarchy.
The cache-invalidation semantics (which hash changes) must be fixed in the same release for the bundle to be trustworthy (companion issue on the hash).
Acceptance criteria
 A consumer that bootstraps from the bundle can evaluate every severity calculate_sla accepts.
 The bundle's snapshot (or a documented companion) includes custom severities.
 Cache refresh via get_config_version_hash/get_last_config_update reflects custom-severity changes.
Out of scope
The hash exclusion itself (companion issue) and a merged snapshot endpoint.

Getting started
just test
Good first files to read: apexchainx_calculator/src/lib.rs (get_config_snapshot, get_config_bundle), apexchainx_calculator/src/config_bundle.rs.


