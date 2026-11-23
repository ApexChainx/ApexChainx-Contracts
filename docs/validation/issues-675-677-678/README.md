# Verification: issues #675, #677 and #678

Validated on 2026-09-27 against base `e0a513369a940f0e9ae1a3236f4cda7ff16b3424`,
on branch `fix/verified-event-correlation-parity` with Rust 1.94.1 and locked dependencies.

## Acceptance evidence

| Issue | Implemented acceptance criteria | Evidence |
|---|---|---|
| #675 | Per-name topic versions; global ABI/result/storage co-bump policy retained; name/version consumer dispatch | Rust event schema, topic/payload, contract-info and version-negotiation tests; TS dispatcher tests |
| #677 | Explicit uniqueness limits; structural ledger bits; determinism, ledger separation and Rust/TS golden vector | Rust correlation property tests and TS correlation tests |
| #678 | Manifest reconciled with real entrypoints and fixture inventory; new methods/fields fail coverage; explicit counted skips | Runtime manifest test and TS mutation/discovery tests |

- **83 Rust acceptance/regression tests passed**, 0 failures, 0 ignored:
  [full output](rust-acceptance.txt).
- **47 supporting Rust tests passed**, 0 failures, 0 ignored:
  [full output](supporting-regressions.txt). Combined: 130 distinct tests.
- **21 TypeScript parity tests passed**, 0 failures, 0 skipped:
  [full output](typescript-parity.txt).
- Typecheck (`tsc --noEmit`) and `cargo fmt --all --check` passed.
- Clippy with warnings denied passed: [output](clippy.txt).
- Production WASM release build passed: [output](wasm-build.txt).
- Coverage accounts for **63 methods: 8 fixture-covered, 55 explicitly skipped**;
  **37 contracttype declarations** are pinned. Skips are not semantic parity coverage.
- Runtime-generated fixtures/constants regenerated without changing the staged
  files or creating untracked artifacts: [output](fixture-regeneration.txt). Their
  reviewed content hashes are in [fixture-sha256.json](fixture-sha256.json).

## Reproduction

`.github/workflows/event-parity-validation.yml` runs the exact Rust groups,
TypeScript parity, coverage, typecheck, format, lint, production WASM build and
fixture-freshness checks, and uploads logs even when a check fails.

## Scope and limits

This is issue-specific verification, not a claim that the complete repository test
suite or release pipeline is green. The full-suite Windows run was interrupted
before completion; its pagination setup-budget failure has a passing targeted
regression here. Existing unrelated orphan-module and release-tooling work is not
included. Generated Soroban environment dump files are not used as the acceptance
evidence; the tracked parity fixtures and logs above are the evidence for this PR.

Issue #674 remains open: no release-preflight implementation, nightly fuzz proof,
waiver registry, release workflow changes or offchain budget repairs are included.
This PR does not approve a release.

## Prerequisite repairs

The base checkout had interleaved legacy/sharded history implementations and a
malformed config-bundle symbol literal that prevented compilation. This PR restores
sharded accessors and contract delegation, repairs those compilation errors and
stale test inputs, and keeps result-schema descriptors consistent across audit and
standalone endpoints. These are necessary to execute the issue-specific tests.

## Compatibility

Event ABI generation 2 requires storage v4 and result schema v2. Only `set_int`
changes its per-name topic version to v2; unchanged events remain v1. Storage
migration from v3 to v4 acknowledges the ABI change without rewriting history.
Correlation IDs are unique across different ledger sequences, but 32-bit outage
fingerprints can collide within one ledger and are not deduplication/authentication
keys. See [migration guidance](../../EVENT_VERSION_MIGRATION.md).
