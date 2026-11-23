#!/usr/bin/env bash
# =============================================================================
# ApexChainx Contracts — Governance Deployment Script
# =============================================================================
#
# Builds and deploys the SLA calculator, initialises governance roles
# (admin + operator), and optionally executes a post-deploy governance
# smoke-check to verify role storage is correct.
#
# Usage (fresh deploy):
#   SOURCE=<identity> ADMIN=<address> OPERATOR=<address> [NETWORK=testnet] \
#     ./scripts/deploy-governance.sh
#
# Usage (governance role transfer — propose + accept two-step):
#   GOVERNANCE_OP=transfer-admin \
#   CONTRACT_ID=<id> SOURCE=<identity> NEW_ADMIN=<address> \
#     ./scripts/deploy-governance.sh
#
#   GOVERNANCE_OP=accept-admin \
#   CONTRACT_ID=<id> SOURCE=<new-admin-identity> \
#     ./scripts/deploy-governance.sh
#
#   GOVERNANCE_OP=cancel-admin-proposal \
#   CONTRACT_ID=<id> SOURCE=<admin-identity> \
#     ./scripts/deploy-governance.sh
#
#   GOVERNANCE_OP=transfer-operator \
#   CONTRACT_ID=<id> SOURCE=<admin-identity> NEW_OPERATOR=<address> \
#     ./scripts/deploy-governance.sh
#
#   GOVERNANCE_OP=accept-operator \
#   CONTRACT_ID=<id> SOURCE=<new-operator-identity> \
#     ./scripts/deploy-governance.sh
#
#   GOVERNANCE_OP=set-operator \
#   CONTRACT_ID=<id> SOURCE=<admin-identity> NEW_OPERATOR=<address> \
#     ./scripts/deploy-governance.sh
#
#   GOVERNANCE_OP=renounce-admin \
#   CONTRACT_ID=<id> SOURCE=<admin-identity> \
#     ./scripts/deploy-governance.sh
#
# Usage (verify governance state):
#   GOVERNANCE_OP=verify \
#   CONTRACT_ID=<id> \
#     ./scripts/deploy-governance.sh
#
# Environment variables:
#   SOURCE          Stellar CLI identity (key name or secret seed)
#   ADMIN           Initial admin address (fresh deploy only)
#   OPERATOR        Initial operator address (fresh deploy only)
#   CONTRACT_ID     Existing contract ID (governance operations only)
#   NEW_ADMIN       Address to propose as new admin
#   NEW_OPERATOR    Address to propose/set as new operator
#   NETWORK         Stellar network (default: testnet)
#   GOVERNANCE_OP   Operation to perform (default: deploy)
#   SKIP_BUILD      Set to 1 to skip WASM build step
# =============================================================================
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:?Set SOURCE to a Stellar CLI identity}"
GOVERNANCE_OP="${GOVERNANCE_OP:-deploy}"
WASM="target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm"

# ── Build step (shared across all operations that need the WASM) ────────────
if [ "${SKIP_BUILD:-0}" != "1" ] && [ "$GOVERNANCE_OP" = "deploy" ]; then
  echo "Building WASM..."
  cargo build --release --target wasm32-unknown-unknown -p apexchainx_calculator
  echo "Build complete: $WASM"
fi

# ── Helper: invoke contract and pretty-print output ─────────────────────────
invoke() {
  local contract_id="$1"
  shift
  stellar contract invoke \
    --id "$contract_id" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    -- "$@"
}

# ── Helper: read-only query (no source required) ─────────────────────────────
query() {
  local contract_id="$1"
  shift
  stellar contract invoke \
    --id "$contract_id" \
    --network "$NETWORK" \
    -- "$@"
}

# ── Fresh deploy ─────────────────────────────────────────────────────────────
if [ "$GOVERNANCE_OP" = "deploy" ]; then
  ADMIN="${ADMIN:?Set ADMIN to the initial admin address}"
  OPERATOR="${OPERATOR:?Set OPERATOR to the initial operator address}"

  echo "Deploying contract to $NETWORK..."
  CONTRACT_ID="$(stellar contract deploy \
    --wasm "$WASM" \
    --source "$SOURCE" \
    --network "$NETWORK")"
  echo "Deployed contract: $CONTRACT_ID"

  echo "Initializing admin=$ADMIN operator=$OPERATOR..."
  invoke "$CONTRACT_ID" initialize \
    --admin "$ADMIN" \
    --operator "$OPERATOR"
  echo "Contract initialized."

  echo ""
  echo "=== Governance state after deploy ==="
  invoke "$CONTRACT_ID" get_admin || true
  invoke "$CONTRACT_ID" get_operator || true

  echo ""
  echo "CONTRACT_ID=$CONTRACT_ID"
  exit 0
fi

# All remaining operations require CONTRACT_ID.
CONTRACT_ID="${CONTRACT_ID:?Set CONTRACT_ID to the deployed contract ID}"

# ── Verify governance state ─────────────────────────────────────────────────
if [ "$GOVERNANCE_OP" = "verify" ]; then
  echo "=== Governance state for $CONTRACT_ID ==="
  echo "Admin:"
  query "$CONTRACT_ID" get_admin

  echo ""
  echo "Operator:"
  query "$CONTRACT_ID" get_operator

  echo ""
  echo "Pending admin proposal:"
  query "$CONTRACT_ID" get_pending_admin || echo "(none)"

  echo ""
  echo "Pending operator proposal:"
  query "$CONTRACT_ID" get_pending_operator || echo "(none)"

  echo ""
  echo "Audit state:"
  query "$CONTRACT_ID" get_audit_state || true
  exit 0
fi

# ── Two-step admin transfer: propose ────────────────────────────────────────
if [ "$GOVERNANCE_OP" = "transfer-admin" ]; then
  NEW_ADMIN="${NEW_ADMIN:?Set NEW_ADMIN to the proposed new admin address}"
  echo "Proposing admin transfer: $NEW_ADMIN on contract $CONTRACT_ID..."
  invoke "$CONTRACT_ID" propose_admin \
    --caller "$(stellar keys address "$SOURCE")" \
    --new_admin "$NEW_ADMIN"
  echo "Admin proposal submitted. The new admin must call accept-admin to complete the transfer."
  echo ""
  echo "Pending admin:"
  query "$CONTRACT_ID" get_pending_admin
  exit 0
fi

# ── Two-step admin transfer: accept ─────────────────────────────────────────
if [ "$GOVERNANCE_OP" = "accept-admin" ]; then
  echo "Accepting admin proposal on contract $CONTRACT_ID..."
  invoke "$CONTRACT_ID" accept_admin \
    --caller "$(stellar keys address "$SOURCE")"
  echo "Admin transfer accepted. New admin is now active."
  echo ""
  echo "Current admin:"
  query "$CONTRACT_ID" get_admin
  exit 0
fi

# ── Admin proposal: cancel ──────────────────────────────────────────────────
if [ "$GOVERNANCE_OP" = "cancel-admin-proposal" ]; then
  echo "Cancelling admin proposal on contract $CONTRACT_ID..."
  invoke "$CONTRACT_ID" cancel_admin_proposal \
    --caller "$(stellar keys address "$SOURCE")"
  echo "Admin proposal cancelled."
  exit 0
fi

# ── Two-step operator transfer: propose ─────────────────────────────────────
if [ "$GOVERNANCE_OP" = "transfer-operator" ]; then
  NEW_OPERATOR="${NEW_OPERATOR:?Set NEW_OPERATOR to the proposed new operator address}"
  echo "Proposing operator transfer: $NEW_OPERATOR on contract $CONTRACT_ID..."
  invoke "$CONTRACT_ID" propose_operator \
    --caller "$(stellar keys address "$SOURCE")" \
    --new_operator "$NEW_OPERATOR"
  echo "Operator proposal submitted. The new operator must call accept-operator to complete the handoff."
  echo ""
  echo "Pending operator:"
  query "$CONTRACT_ID" get_pending_operator
  exit 0
fi

# ── Two-step operator transfer: accept ──────────────────────────────────────
if [ "$GOVERNANCE_OP" = "accept-operator" ]; then
  echo "Accepting operator proposal on contract $CONTRACT_ID..."
  invoke "$CONTRACT_ID" accept_operator \
    --caller "$(stellar keys address "$SOURCE")"
  echo "Operator transfer accepted. New operator is now active."
  echo ""
  echo "Current operator:"
  query "$CONTRACT_ID" get_operator
  exit 0
fi

# ── Operator proposal: cancel ────────────────────────────────────────────────
if [ "$GOVERNANCE_OP" = "cancel-operator-proposal" ]; then
  echo "Cancelling operator proposal on contract $CONTRACT_ID..."
  invoke "$CONTRACT_ID" cancel_operator_proposal \
    --caller "$(stellar keys address "$SOURCE")"
  echo "Operator proposal cancelled."
  exit 0
fi

# ── Single-step operator set (break-glass) ──────────────────────────────────
if [ "$GOVERNANCE_OP" = "set-operator" ]; then
  NEW_OPERATOR="${NEW_OPERATOR:?Set NEW_OPERATOR to the new operator address}"
  echo "WARNING: set-operator is the legacy break-glass path. It bypasses the"
  echo "two-step consent flow and installs the new operator without their signature."
  echo "Use transfer-operator + accept-operator for routine rotations."
  echo ""
  read -r -p "Continue with single-step operator assignment? [y/N] " confirm
  if [ "${confirm}" != "y" ] && [ "${confirm}" != "Y" ]; then
    echo "Aborted."
    exit 1
  fi
  invoke "$CONTRACT_ID" set_operator \
    --caller "$(stellar keys address "$SOURCE")" \
    --new_operator "$NEW_OPERATOR"
  echo "Operator set (single-step). New operator: $NEW_OPERATOR"
  exit 0
fi

# ── Admin renounce ────────────────────────────────────────────────────────────
if [ "$GOVERNANCE_OP" = "renounce-admin" ]; then
  echo "WARNING: renounce-admin is PERMANENT and IRREVERSIBLE."
  echo "The contract will be permanently adminless after this operation."
  echo "All pending governance proposals will be cancelled."
  echo ""
  read -r -p "Type CONFIRM to permanently renounce admin authority: " confirm
  if [ "${confirm}" != "CONFIRM" ]; then
    echo "Aborted. You must type CONFIRM to proceed."
    exit 1
  fi
  invoke "$CONTRACT_ID" renounce_admin \
    --caller "$(stellar keys address "$SOURCE")"
  echo "Admin authority permanently renounced."
  exit 0
fi

echo "Unknown GOVERNANCE_OP: $GOVERNANCE_OP"
echo "Valid values: deploy, verify, transfer-admin, accept-admin, cancel-admin-proposal,"
echo "              transfer-operator, accept-operator, cancel-operator-proposal,"
echo "              set-operator, renounce-admin"
exit 1
