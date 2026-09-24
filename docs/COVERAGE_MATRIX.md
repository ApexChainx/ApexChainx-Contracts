# Cross-Language Coverage Matrix (#632)

The repository's behavioural contract is enforced by two tool chains that used
to pass or fail independently:

| Target | Runner | Surface |
| :--- | :--- | :--- |
| `rust` | `cargo test` (`just test`) | unit/integration tests in `apexchainx_calculator/src` |
| `ts-property` | `npm run test:property` | property/regression matrix in `tests/*.test.ts` |
| `ts-parity` | `npm run test:parity` (`just ts-parity`) | `ts/parity/` parity suite plus the `ts/` mirrors it replays |
| `offchain` | `npm run test:offchain` | off-chain integration probes in `offchain/*.ts` |

Each side had its own expectations, so a method covered by Rust tests but never
touched by the TypeScript mirrors was invisible — both tool chains could be
green while their models disagreed. The coverage matrix is the orchestration
layer that reports coverage per method across every target and flags the
mismatches.

## Script

`scripts/coverage-matrix.ts` drives the matrix:

    tsx scripts/coverage-matrix.ts              # print the matrix + gap summary
    tsx scripts/coverage-matrix.ts --strict     # exit 1 on any gap (CI mode)
    tsx scripts/coverage-matrix.ts --out FILE   # also write a markdown report

It is dependency-free (Node core modules only). The method registry is derived
from the contract's own `get_public_api()` descriptor (`method("…")` records in
`lib.rs`), so the matrix can never silently rename a method; methods missing
from the feature registry are reported as **MATRIX DRIFT** gaps.

## Verdicts

Each row is per public method, per target:

- `OK` — covered in Rust tests **and** in at least one TypeScript target.
- `RUST-ONLY GAP` — covered in Rust tests but no TypeScript target touches it.
- `TS-ONLY GAP` — covered only on the TypeScript side; Rust tests never do.
- `UNCOVERED` — exercised nowhere.
- `MATRIX DRIFT` — present in the contract ABI but not in the feature registry.

In `--strict` mode (used by the CI job) any non-`OK` row fails the run.

## CI

`.github/workflows/coverage-matrix.yml` runs the script with `--strict` on the
same events as `ci.yml` (currently temporarily disabled, matching the rest of
the pipeline). The generated `coverage-matrix-report.md` is uploaded as an
artifact so reviewers can inspect the matrix without running anything.

## Keeping the matrix honest

- A new public method must be added to the feature registry in
  `scripts/coverage-matrix.ts` (the drift check tells you if you forgot) and to
  the corresponding test sides.
- Adding Rust-only coverage for a method without TS coverage is a **gap** by
  design — either mirror it in `ts/`/the parity suite or document why the
  surface is Rust-only in the matrix row.