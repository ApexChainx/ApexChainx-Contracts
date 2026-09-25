#!/usr/bin/env bash
# =============================================================================
# Orphan Module Lint (issue #491)
# =============================================================================
#
# Fails when a `.rs` file under the apexchainx_calculator crate is never
# declared as a module. A file that exists on disk but is never declared is
# invisible to `cargo check` / `clippy` / `test`: it silently stops compiling
# (and running), so its tests, docs and safety guarantees rot without CI ever
# noticing — exactly what happened to `scratch_test.rs` (a leftover interactive
# experiment committed at the crate root) and to the src/ orphans tracked by
# issue #422.
#
# What this lint checks:
#   1. Crate-root `.rs` files (apexchainx_calculator/*.rs). A file sitting next
#      to Cargo.toml is never part of the module graph unless referenced via
#      `#[path]`/`include!`, so any such file is an orphan. Throwaway code
#      belongs in /tmp, the fuzz corpus, or a never-merged branch — not here.
#   2. `src/*.rs` files whose stem is not declared as a module in `src/lib.rs`
#      (e.g. `mod foo;` / `pub mod foo;` / `#[cfg(test)] mod foo;`).
#
# Subdirectories under src/ (defaults/, fixtures/, metrics/) are excluded: they
# have their own `mod.rs` structure and are declared as a unit from lib.rs.
# The fuzz/ workspace and target/ are excluded for the same reason.
#
# Usage:
#   ./scripts/check-orphan-modules.sh     # scan from repo root
#
# Exit codes:
#   0 – no undeclared `.rs` files (beyond the tracked allowlist)
#   1 – one or more undeclared `.rs` files found
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CRATE_DIR="$REPO_ROOT/apexchainx_calculator"
LIB_RS="$CRATE_DIR/src/lib.rs"

# ---------------------------------------------------------------------------
# Known orphans, tracked by companion issue #422 and restored in PR #545.
#
# These files are present under src/ but are not (yet) declared as modules,
# because they do not compile against the current contract API — #545 carries
# the fixes (pause() gained a reason argument, calculate_sla requires distinct
# outage ids, the generated client is lifetime-parameterised, symbol lengths
# were shortened, …). Declaring them here without those fixes would break the
# build, and duplicating #545's fixes would conflict with it.
#
# When #545 lands and declares these modules, this list becomes stale: the
# files will no longer be orphans, and this script will print a warning (and
# still pass) until the entries are removed.
# ---------------------------------------------------------------------------
ALLOWLIST=(
    auth_matrix_tests.rs
    event_ordering_tests.rs
    event_state_tests.rs
    outage_id_tests.rs
    payload_optimizer.rs
    payload_versioning_tests.rs
    policy.rs
    pruning_perf.rs
    threshold_config.rs
    topic_stability_tests.rs
)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

ORPHANS=()
STALE_ALLOWLIST=()

# --- Check 1: crate-root .rs files (the scratch_test.rs class) --------------
while IFS= read -r file; do
    [ -z "$file" ] && continue
    ORPHANS+=("$file")
done < <(find "$CRATE_DIR" -maxdepth 1 -name '*.rs' 2>/dev/null | sort)

# --- Check 2: src/*.rs files not declared in src/lib.rs ----------------------
# Extract declared module names: `mod foo;`, `pub mod foo;`, `pub(crate) mod foo;`
# (and the same after a `#[cfg(...)]` attribute line). Inline `mod foo { … }`
# blocks are not file modules and must NOT be counted.
DECLARED="$(grep -E '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?mod[[:space:]]+[a-zA-Z0-9_]+[[:space:]]*;' "$LIB_RS" \
    | sed -E 's/^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?mod[[:space:]]+([a-zA-Z0-9_]+)[[:space:]]*;.*/\3/')"

while IFS= read -r file; do
    [ -z "$file" ] && continue
    stem="$(basename "$file" .rs)"
    if ! grep -qx "$stem" <<<"$DECLARED"; then
        ORPHANS+=("$(basename "$file")")
    fi
done < <(find "$CRATE_DIR/src" -maxdepth 1 -name '*.rs' ! -name 'lib.rs' ! -name 'main.rs' 2>/dev/null | sort)

# --- Separate tracked orphans from new ones ----------------------------------
UNTRACKED=()
for name in "${ORPHANS[@]}"; do
    if printf '%s\n' "${ALLOWLIST[@]}" | grep -qx "$name"; then
        # Still an orphan → it is tracked; nothing to do. If it is now declared
        # it would not appear in ORPHANS at all, so nothing else here.
        :
    else
        UNTRACKED+=("$name")
    fi
done

# Warn about allowlist entries that are no longer orphans (now declared).
for name in "${ALLOWLIST[@]}"; do
    if ! printf '%s\n' "${ORPHANS[@]}" | grep -qx "$name"; then
        STALE_ALLOWLIST+=("$name")
    fi
done

# --- Report ----------------------------------------------------------------
if [ "${#UNTRACKED[@]}" -gt 0 ]; then
    echo -e "${RED}==========================================================================${NC}"
    echo -e "${RED}  ORPHAN MODULE LINT FAILED — undeclared .rs files found${NC}"
    echo -e "${RED}==========================================================================${NC}"
    echo ""
    echo "These .rs files exist in the crate but are never declared as a module,"
    echo "so cargo never compiles them and their tests never run:"
    echo ""
    for name in "${UNTRACKED[@]}"; do
        echo "  - $name"
    done
    echo ""
    echo -e "${YELLOW}Fix:${NC}"
    echo "  - Throwaway/scratch code does not belong in the crate at all: keep it"
    echo "    in /tmp, the fuzz corpus, or a branch that is never merged."
    echo "  - Real modules/tests: declare them in src/lib.rs, e.g."
    echo "        pub mod my_module;      # or  #[cfg(test)] mod my_module_tests;"
    echo "    and make sure the file compiles (an undeclared file has been"
    echo "    invisible to CI, so it may need fixes before it builds)."
    echo ""
    echo "  See issue #491 (scratch_test.rs cleanup) and issue #422 (the"
    echo "  tracked orphan modules)."
    echo ""
    exit 1
fi

if [ "${#STALE_ALLOWLIST[@]}" -gt 0 ]; then
    echo -e "${YELLOW}Warning: allowlist entries in scripts/check-orphan-modules.sh are no longer orphans${NC}"
    echo "  (they are now declared as modules) — remove them from ALLOWLIST:"
    for name in "${STALE_ALLOWLIST[@]}"; do
        echo "  - $name"
    done
    echo ""
fi

echo -e "${GREEN}Orphan module lint passed — every .rs file in the crate is declared as a module.${NC}"
exit 0
