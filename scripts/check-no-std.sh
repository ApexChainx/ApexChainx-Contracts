#!/usr/bin/env bash
# =============================================================================
# no_std Compatibility Lint
# =============================================================================
#
# Scans all Rust source files in the apexchainx_calculator crate for accidental
# `std` imports.  The contract is declared `#![no_std]` and the WASM target
# (wasm32-unknown-unknown) does not ship std — but a `use std::…` or
# `extern crate std;` statement can accidentally survive in test helpers or
# cfg-gated blocks and go unnoticed until the next WASM build in CI.
#
# This script acts as an early, deterministic gate that fails the build before
# the expensive WASM compilation step, giving contributors a clear signal.
#
# Usage:
#   ./scripts/check-no-std.sh           # scan from repo root
#   ./scripts/check-no-std.sh --fix     # print fix hints (informational only)
#
# Exit codes:
#   0 – no std imports found
#   1 – one or more std imports found
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CRATE_DIR="$REPO_ROOT/apexchainx_calculator"

# Patterns that indicate a direct std dependency.
# We deliberately keep the list narrow to avoid false positives on doc
# comments or string literals.
PATTERNS=(
    '^[[:space:]]*(pub([(].*[)])?[[:space:]]+)?use[[:space:]]+std::'
    '^[[:space:]]*extern[[:space:]]+crate[[:space:]]+std[[:space:]]*;'
)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

FOUND=0
MATCHES=""

for pattern in "${PATTERNS[@]}"; do
    while IFS= read -r line; do
        if [ -n "$line" ]; then
            FOUND=$((FOUND + 1))
            MATCHES+="$line"$'\n'
        fi
    done < <(grep -rn --color=never "$pattern" "$CRATE_DIR/src/" 2>/dev/null || true)
done

if [ "$FOUND" -gt 0 ]; then
    echo -e "${RED}==========================================================================${NC}"
    echo -e "${RED}  no_std COMPATIBILITY LINT FAILED — std imports detected${NC}"
    echo -e "${RED}==========================================================================${NC}"
    echo ""
    echo "The following lines import from std, which is forbidden in this"
    echo "no_std contract crate:"
    echo ""
    echo "$MATCHES"
    echo ""
    echo -e "${YELLOW}Fix:${NC}"
    echo "  - Replace 'use std::…' with imports from 'core::' or 'alloc::'."
    echo "  - If the import is inside a #[cfg(test)] block that genuinely"
    echo "    needs std, gate the test with #[cfg(feature = \"std\")] and"
    echo "    add the feature to Cargo.toml."
    echo "  - If the import is inside a comment or string, wrap it so it"
    echo "    doesn't match the regex (e.g. split the line)."
    echo ""
    echo -e "${YELLOW}Why:${NC}"
    echo "  The contract is #![no_std] and targets wasm32-unknown-unknown."
    echo "  Accidental std imports cause opaque WASM compilation failures"
    echo "  that are hard to diagnose. This lint catches them early."
    echo ""
    exit 1
fi

echo -e "${GREEN}no_std compatibility lint passed — no std imports found.${NC}"
exit 0
