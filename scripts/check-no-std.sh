#!/usr/bin/env bash
# Compile the production dependency graph against a target with no std library.
# Text scans incorrectly reject cfg(test) imports and miss transitive std usage.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cargo check --locked --manifest-path "$REPO_ROOT/apexchainx_calculator/Cargo.toml" \
  --target wasm32-unknown-unknown --lib
