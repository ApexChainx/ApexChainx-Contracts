# SLA Operations Workspace

This module namespace is the dedicated, project-specific contribution area for the ApexChainx Contracts repository (calculator crate).

## Purpose
`apexchainx_calculator/src/sla_operations/` is reserved for contributor-built SLA operations extensions that complement the Soroban SLA calculator (analysis helpers, policy/profile utilities, validation and reporting helpers, test fixtures/utilities, and other additive modules) without destabilizing the core contract logic maintained separately.

## Why this location fits the project
- `apexchainx_calculator` is the primary contract crate implementing deterministic SLA calculation, configuration, governance controls, history, and telemetry on Soroban/Stellar.
- The crate already uses modular internal namespaces under `src/` (e.g. `calculation`, `policy`, `metrics`, `history`, `oracle`).
- A dedicated `sla_operations` namespace provides an architecturally native location for additive contributor modules aligned with the contract's SLA operations domain.

## Boundaries
- **Do primary implementation work here:** `apexchainx_calculator/src/sla_operations/**`
- **Avoid unrelated edits** to stabilized core modules, public contract entrypoints, storage layout, event schemas, and release-critical logic unless an assigned issue explicitly requires and documents a minimal integration change.
- Any required module registration or wiring outside this directory (for example exporting from `lib.rs` or crate-level configuration) must be narrowly scoped, explicitly justified in the issue/PR, and coordinated to avoid ABI/storage/event regressions.

## Suggested structure (as needed per issue)
- `mod.rs` / submodules – isolated Rust modules for contributed functionality
- `validators/` or `policies/` – additive validation/policy helpers
- `analytics/` – read-only or pure analysis helpers
- `fixtures/` – deterministic test data/helpers for contributed modules
- `tests.rs` or `*_tests.rs` – module-focused tests
- `docs/` – module-level notes and invariants

## Contributor workflow
1. Select an assigned issue scoped to `apexchainx_calculator/src/sla_operations/`.
2. Create a branch: `feat/issue-<number>-short-description` or `fix/issue-<number>-short-description`.
3. Implement only the requested work inside this workspace.
4. Add/update tests relevant to your changes.
5. Run checks before opening a PR (as applicable from crate/repo root):
   - `cargo fmt -- --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - `cargo check --target wasm32-unknown-unknown --lib`
6. Open a PR referencing the issue and clearly note any approved exception that touches files outside this workspace.

## Quality expectations
- Preserve `no_std` compatibility and Soroban constraints for contract code.
- Keep changes deterministic and compatible with existing storage/event/versioning guarantees.
- Do not introduce breaking API/event/storage behavior in core contract surfaces.
- Favor pure, well-tested helpers and explicit documentation of assumptions.
