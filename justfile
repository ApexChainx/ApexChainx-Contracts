# =============================================================================
# ApexChainx Contracts — common developer commands (issue #113)
# =============================================================================
#
# One-liners for the sequences that CI runs, so a contributor can reproduce a
# CI failure locally without reading the workflow YAML. Every cargo recipe
# mirrors the matching step in .github/workflows/ci.yml (noted per recipe) and
# runs in the contract crate, which is CI's `working-directory`.
#
# Install just:  brew install just  |  cargo install just  |  https://just.systems
# Usage:         just <recipe>      |  `just` on its own lists every recipe.
# =============================================================================

# Contract crate — matches `working-directory: apexchainx_calculator` in CI.
crate := "apexchainx_calculator"
wasm_target := "wasm32-unknown-unknown"

# Pinned toolchain channel sourced from rust-toolchain.toml.
# Kept in sync with the `channel` field so bootstrap installs the right version.
toolchain_channel := "1.94.1"

# List available recipes.
default:
    @just --list

# ----------------------------------------------------------- bootstrap ------

# Bootstrap the dev environment — install pinned toolchain, WASM target, and verify setup.
#
# This recipe is session-safe: it is idempotent and can be re-run at any time
# without side effects. Each step is guarded so it only performs work when
# something is actually missing or outdated.
#
# What it does (in order):
#   1. Verify that rustup is installed and reachable.
#   2. Install / update the pinned Rust toolchain (1.94.1) if not already present.
#   3. Add the wasm32-unknown-unknown cross-compilation target if missing.
#   4. Confirm cargo is available on PATH.
#   5. Print a summary and prompt the contributor to run `just ci`.
#
# Prerequisites (not installed by this script — see CONTRIBUTING.md):
#   • rustup   https://rustup.rs
#   • just     https://just.systems  (brew install just / cargo install just)
bootstrap:
    #!/usr/bin/env bash
    set -euo pipefail

    echo ""
    echo "╔══════════════════════════════════════════════════════╗"
    echo "║  ApexChainx — Developer Environment Bootstrap        ║"
    echo "╚══════════════════════════════════════════════════════╝"
    echo ""

    # ── Step 1: verify rustup ──────────────────────────────────────────────
    echo "▶ [1/4] Checking for rustup..."
    if ! command -v rustup >/dev/null 2>&1; then
        echo ""
        echo "  ✗ rustup not found."
        echo "  Install it from https://rustup.rs then re-run: just bootstrap"
        echo ""
        exit 1
    fi
    RUSTUP_VERSION=$(rustup --version 2>&1 | head -1)
    echo "  ✓ rustup found: ${RUSTUP_VERSION}"

    # ── Step 2: install / confirm pinned toolchain ────────────────────────
    echo ""
    echo "▶ [2/4] Ensuring pinned Rust toolchain ({{toolchain_channel}}) is installed..."
    # `rustup toolchain install` is idempotent: it is a no-op when the toolchain
    # is already current and only downloads when an update is needed.
    rustup toolchain install "{{toolchain_channel}}" --component rustfmt clippy
    echo "  ✓ Toolchain {{toolchain_channel}} ready."

    # ── Step 3: add WASM target ───────────────────────────────────────────
    echo ""
    echo "▶ [3/4] Adding wasm32-unknown-unknown target..."
    rustup target add {{wasm_target}} --toolchain "{{toolchain_channel}}"
    echo "  ✓ Target {{wasm_target}} installed."

    # ── Step 4: verify cargo ──────────────────────────────────────────────
    echo ""
    echo "▶ [4/4] Verifying cargo is available..."
    if ! command -v cargo >/dev/null 2>&1; then
        echo ""
        echo "  ✗ cargo not found on PATH."
        echo "  Ensure ~/.cargo/bin is on your PATH, then re-run: just bootstrap"
        echo ""
        exit 1
    fi
    CARGO_VERSION=$(cargo --version 2>&1)
    echo "  ✓ cargo found: ${CARGO_VERSION}"

    # ── Done ──────────────────────────────────────────────────────────────
    echo ""
    echo "╔══════════════════════════════════════════════════════╗"
    echo "║  ✅  Bootstrap complete!                              ║"
    echo "║                                                      ║"
    echo "║  Next step: run  just ci  to verify your build.     ║"
    echo "╚══════════════════════════════════════════════════════╝"
    echo ""

# ---------------------------------------------------------------- test ------

# Run the library test suite.            [CI: E2E Tests]
test:
    cd {{crate}} && cargo test --lib

# Run the property-based fuzz tests.     [CI: Fuzz Tests (proptest)]
#
# NOTE: this is the stable-toolchain proptest layer, not the coverage-guided
# cargo-fuzz targets in {{crate}}/fuzz/ (those run nightly — see `fuzz-build`
# and `fuzz-run` below, and docs/FUZZING_GUARANTEES.md for what each layer
# guarantees).
fuzz:
    cd {{crate}} && cargo test --lib fuzz_tests::

# Run the spec-assertion unit tests that pin what the cargo-fuzz targets check.
#
# The fuzz targets are thin wrappers over apexchainx_calculator::fuzz_spec, so
# these tests exercise the same invariants on the stable toolchain, on fixed
# boundary vectors, without needing nightly or cargo-fuzz.
fuzz-spec:
    cd {{crate}} && cargo test --lib fuzz_spec::

# Compile the cargo-fuzz targets.        [CI: fuzz.yml / build-targets]
#
# The targets live in a separate workspace that `cargo test` and `just ci` do
# not build, so a target that stops compiling is otherwise invisible until the
# nightly campaign runs. Requires a nightly toolchain, cargo-fuzz, and a C++
# compiler for libfuzzer-sys.
fuzz-build:
    cd {{crate}}/fuzz && cargo +nightly fuzz build

# Run one coverage-guided fuzz campaign. `just fuzz-run compute_result 60`
fuzz-run target seconds="60":
    cd {{crate}}/fuzz && cargo +nightly fuzz run {{target}} -- -max_total_time={{seconds}}

# Run the parity checker against canonical golden vectors.  [CI: parity-check]
# Fails if any compute_result output diverges from the locked-in baseline.
# See apexchainx_calculator/test_snapshots/tests/parity_baseline.json.
parity-check:
    cd {{crate}} && cargo test --lib parity_tests::

# ------------------------------------------------------------ ts property tests ------

# Run the TypeScript property/regression test suite in tests/.
# These tests assert cross-language parity, simulation reproducibility,
# and behavioral invariants that cargo test does not cover.
ts-property-tests:
    npx tsx --test tests/*.test.ts

# ------------------------------------------------------------ offchain checks ------

# Run offchain budget/regression checks (event size, read cost, governance, metadata).
# Each script exits non-zero on assertion failure, so CI can gate on the recipe.
offchain-checks:
    npx tsx offchain/eventSizeRegression.ts
    npx tsx offchain/readCostRegression.ts
    npx tsx offchain/governanceConsistency.ts
    npx tsx offchain/contractMetadata.ts

# Regenerate and verify the committed persistence scenario fixtures (pers-store).
# Deterministic by construction (no randomness); the generator rewrites
# pers-store/fixtures/persistence-fixtures.json and exits non-zero when the
# committed file is stale, so staleness fails CI's offchain-checks job (#631).
pers-store-check:
    npx tsx pers-store/generate-fixtures.ts

# ------------------------------------------------------------ ts parity ------

# Regenerate the contract-derived artefacts the ts/ helpers are checked against.
#
# Executes the real contract in a Soroban Env and writes
# ts/generated/contractConstants.ts and ts/fixtures/contract-read-semantics.json.
# Both are committed; see docs/TS_PARITY_CONTRACT.md.
ts-fixtures:
    cd {{crate}} && cargo test --lib ts_parity_fixtures

# Type-check the off-chain TypeScript in ts/.          [CI: ts-parity]
ts-typecheck:
    npx tsc --noEmit

# Run the TypeScript parity suite against those artefacts.  [CI: ts-parity]
ts-parity: ts-typecheck
    npx tsx --test ts/parity/readSemanticsParity.test.ts

# Full TS parity gate: regenerate, fail on an uncommitted diff, then compare.
#
# The freshness check is the half that catches a Rust change which never
# reached TypeScript: if regenerating produces a diff, the contract moved and
# the artefacts were not committed.
ts-check: ts-fixtures
    #!/usr/bin/env bash
    set -euo pipefail
    if ! git diff --quiet -- ts/fixtures ts/generated; then
        echo ""
        echo "✗ Contract-derived TS artefacts are stale."
        echo "  A read semantic changed in Rust but ts/fixtures and ts/generated"
        echo "  were not regenerated and committed. The diff below IS the change:"
        echo ""
        git --no-pager diff -- ts/fixtures ts/generated
        echo ""
        echo "  Commit the regenerated files, then run 'just ts-parity' to see"
        echo "  which hand-written helper in ts/ still disagrees."
        echo "  See docs/TS_PARITY_CONTRACT.md."
        exit 1
    fi
    just ts-parity

# ---------------------------------------------------------------- lint ------

# Format the crate in place.
fmt:
    cd {{crate}} && cargo fmt

# Verify formatting without writing.     [CI: Format check]
fmt-check:
    cd {{crate}} && cargo fmt --check

# Clippy with warnings denied.           [CI: Clippy]
lint:
    cd {{crate}} && cargo clippy --all-targets -- -D warnings

# Type-check the crate.                  [CI: Cargo check]
check:
    cd {{crate}} && cargo check

# --------------------------------------------------------------- build ------

# Build natively.                        [CI: Build native]
build:
    cd {{crate}} && cargo build

# Build the WASM contract.               [CI: Build WASM]
wasm:
    cd {{crate}} && cargo build --target {{wasm_target}}

# Build the release WASM.                [CI: Provenance & Hashes]
wasm-release:
    cd {{crate}} && cargo build --target {{wasm_target}} --release

# Assert no_std compliance for wasm32.   [CI: WASM no-std compliance check]
no-std:
    cd {{crate}} && cargo check --target {{wasm_target}} --lib

# Fail when a .rs file in the crate is never declared as a module (#491).
# An undeclared file is invisible to cargo check/clippy/test and rots silently;
# see scripts/check-orphan-modules.sh.                    [CI: client-checks]
lint-orphans:
    ./scripts/check-orphan-modules.sh

# sha256 of the release WASM.            [CI: Generate hash]
hash: wasm-release
    #!/usr/bin/env bash
    set -euo pipefail
    # Workspace build — artifacts land in the ROOT target/, not {{crate}}/target/.
    wasm="target/{{wasm_target}}/release/{{crate}}.wasm"
    if [ ! -f "$wasm" ]; then
        echo "WASM file not found at $wasm" >&2
        exit 1
    fi
    # sha256sum on Linux/CI, shasum on macOS.
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$wasm" | awk '{print $1 "  {{crate}}.wasm"}'
    else
        shasum -a 256 "$wasm" | awk '{print $1 "  {{crate}}.wasm"}'
    fi

# Save release WASM hash to artifacts/.    [CI: Save hash for provenance]
hash-save: wasm-release
    #!/usr/bin/env bash
    set -euo pipefail
    wasm="target/{{wasm_target}}/release/{{crate}}.wasm"
    hash_file="artifacts/{{crate}}.wasm.sha256"
    mkdir -p artifacts
    if [ ! -f "$wasm" ]; then
        echo "WASM file not found at $wasm" >&2
        exit 1
    fi
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$wasm" | awk '{print $1 "  {{crate}}.wasm"}' > "$hash_file"
    else
        shasum -a 256 "$wasm" | awk '{print $1 "  {{crate}}.wasm"}' > "$hash_file"
    fi
    echo "Hash saved to $hash_file"
    cat "$hash_file"

# Verify release WASM hash against committed file. [CI: Verify hash provenance]
hash-verify: wasm-release
    #!/usr/bin/env bash
    set -euo pipefail
    wasm="target/{{wasm_target}}/release/{{crate}}.wasm"
    hash_file="artifacts/{{crate}}.wasm.sha256"
    if [ ! -f "$hash_file" ]; then
        echo "Hash file not found at $hash_file" >&2
        echo "Run 'just hash-save' to generate it." >&2
        exit 1
    fi
    if [ ! -f "$wasm" ]; then
        echo "WASM file not found at $wasm" >&2
        exit 1
    fi
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -c "$hash_file"
    else
        shasum -a 256 -c "$hash_file"
    fi
# ------------------------------------------------------- dependency-hygiene -----

# Check for unused dependencies with cargo-machete.              [CI: Unused dependency gate]
machete:
    cargo install cargo-machete --locked 2>/dev/null; cd {{crate}} && cargo machete

# Check for unused dependencies with cargo-udeps.                [CI: Unused dependency gate]
# Requires nightly Rust. Pin nightly-2026-02-01: newer nightlies (>= 1.96)
# break ethnum 1.5.0, which cargo-udeps builds as a transitive dependency.
# rustup auto-installs the toolchain on first use.
udeps:
    cargo install cargo-udeps --locked 2>/dev/null; cd {{crate}} && cargo +nightly-2026-02-01 udeps --all-targets
# --------------------------------------------------------------- tooling -----
# ----------------------------------------------------------- release ------

# Minimal release candidate validation (fast).  [CI: Release Replay]
# Runs format, clippy, no-std, core tests, topic-stability, and WASM build.
# Use release-replay-full for fuzz + full test suite.
release-replay:
    npx --yes tsx scripts/release-replay.ts

release-replay-full:
    npx --yes tsx scripts/release-replay.ts --full

# --------------------------------------------------------------- tooling ------
# Generate a ship-review note from CHANGELOG.md.
# Pass a version tag to summarise a released block: just release-summary 0.3.0
# Defaults to the [Unreleased] block when no argument is given.
release-summary version="Unreleased":
    npx --yes tsx tooling/releaseSummary.ts --version {{version}}

# ----------------------------------------------------------- snapshots -----

# Normalize test snapshot artifacts.        [CI: Normalize snapshot artifacts]
# Strips volatile fields (timestamp, elapsed_ms, generated_at) and sorts keys.
# Run this after making contract changes that affect snapshot outputs.
normalize-snapshots:
    npx --yes tsx tools/normalize-snapshots.ts

# Regenerate snapshots: run tests then normalize. [CI: E2E Tests + Normalize]
# Use this when contract behavior changes and snapshots need updating.
regenerate-snapshots: normalize-snapshots
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{crate}} && cargo test --lib
    just normalize-snapshots

# Verify snapshots are normalized (dry-run check).
# Exits with error if snapshots need normalization without modifying files.
verify-snapshots:
    npx --yes tsx tools/normalize-snapshots.ts
    @if git diff --quiet apexchainx_calculator/test_snapshots/; then \
        echo "✓ Snapshots are normalized"; \
    else \
        echo "✗ Snapshots need normalization. Run 'just normalize-snapshots'"; \
        git diff apexchainx_calculator/test_snapshots/; \
        exit 1; \
    fi

# ----------------------------------------------------------------- all ------

# Remove build artifacts.
clean:
    cd {{crate}} && cargo clean

# Everything CI gates on, in CI's order. Run before opening a PR.
ci: fmt-check lint check no-std lint-orphans test fuzz fuzz-spec parity-check ts-check wasm machete udeps verify-snapshots
    @echo "✓ local CI equivalent passed"
