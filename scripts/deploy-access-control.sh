#!/usr/bin/env bash
# Build and deploy the SLA calculator, then initialize its admin and operator roles.
#
# Usage: SOURCE=<identity> ADMIN=<address> OPERATOR=<address> [NETWORK=testnet] ./scripts/deploy-access-control.sh
#
# Issue #758 - the upgrade path. When the contract is already deployed, pass
# UPGRADE=1 and CONTRACT_ID: the script then installs the new WASM, points
# the existing contract at it and calls `migrate()` so the stored schema
# version is brought forward.
#
# The roles are deliberately left alone by an upgrade: `migrate()` only moves
# the stored version forward, and re-initializing the contract would reset the
# admin and operator that the upgrade is supposed to preserve. The procedure
# (pause, install, migrate, verify) is the one in
# `docs/UPGRADE_PLAYBOOK.md`.
#
# Upgrade usage:
#   SOURCE=<identity> CONTRACT_ID=<id> UPGRADE=1 [NETWORK=testnet] ./scripts/deploy-access-control.sh
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:?Set SOURCE to a Stellar CLI identity}"
WASM="target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm"

cargo build --release --target wasm32-unknown-unknown -p apexchainx_calculator

# ── Upgrade path (Issue #758) ────────────────────────────────────────────────
if [ "${UPGRADE:-0}" = "1" ]; then
  CONTRACT_ID="${CONTRACT_ID:?Set CONTRACT_ID to the contract to upgrade}"

  # The WASM hash is printed by the CLI; pick the 64 hex characters out of
  # its output so the script does not depend on the exact wording.
  if [ -z "${WASM_HASH:-}" ]; then
    WASM_HASH="$(stellar contract install --wasm "$WASM" --source "$SOURCE" --network "$NETWORK" \
      | grep -oE '[0-9a-f]{64}' | tail -n 1)"
  fi
  : "${WASM_HASH:?Could not determine the WASM hash; pass WASM_HASH=...}"

  echo "Upgrading contract: $CONTRACT_ID (wasm $WASM_HASH)"

  # Point the existing contract at the new WASM, then move its stored schema
  # version forward. `migrate()` is idempotent, so re-running the upgrade
  # against an already-current contract is a safe no-op.
  stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
    -- update_current_contract_wasm --wasm_hash "$WASM_HASH"

  stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
    -- migrate

  echo "Migration state after the upgrade:"
  stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
    -- get_migration_state

  exit 0
fi

ADMIN="${ADMIN:?Set ADMIN to the admin address}"
OPERATOR="${OPERATOR:?Set OPERATOR to the operator address}"

CONTRACT_ID="$(stellar contract deploy --wasm "$WASM" --source "$SOURCE" --network "$NETWORK")"
echo "Deployed contract: $CONTRACT_ID"

stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
  -- initialize --admin "$ADMIN" --operator "$OPERATOR"
