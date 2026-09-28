# Full-suite failure repairs

Historical evidence for commit `3555a2a`. Current main subsequently removed the thirteen orphan modules; the conflict resolution adopts those deletions instead of reintroducing the helpers. Current CI is authoritative for the integrated branch.

The first release-validation run completed with 790 passed, three failed and five ignored. The targeted issue checks had passed, so the release-preflight acceptance workflow now runs the entire Rust library suite as well as checking generated freshness.

- Declare all thirteen orphan modules; no lint exemptions or ignored tests were added. Repair their previously uncompiled SDK/module imports, use canonical sharded history for duplicate scans and invariants, and check feasibility examples against the real validator.
- Re-export the existing endpoint RentEstimate type instead of compiling a second, incompatible contracttype with the same name. The reviewed parity inventory change removes only that duplicate declaration (37 to 36); the endpoint type stays unchanged.
- Newly declared optional helpers do not reroute contract entrypoints or migrate storage. Their documentation now states this explicitly. The outage helper uses the actual Soroban symbol alphabet and raw symbol bytes.
- Pin numeric result schema version 2 alongside its v2 label and exported constant.
- Assert precise ProposalExpired errors on repeated operator accepts, preserved proposal/timestamp/role and no committed events, followed by explicit successful cancellation. This reflects Soroban rollback instead of requiring an impossible committed mutation from a rejected invocation.

Local validation: ten focused Rust tests, all 156 TypeScript tests, Clippy with warnings denied and release WASM build passed. Hosted full-suite evidence will be linked in PR #721. The nightly evidence gate remains strict; these repairs do not authorize a release waiver.
