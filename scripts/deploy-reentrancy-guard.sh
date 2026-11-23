#!/usr/bin/env bash
# Build and deploy the SLA calculator contract with ReentrancyGuard / cross-contract safety verification.
#
# Usage: SOURCE=<identity> ADMIN=<address> OPERATOR=<address> [NETWORK=testnet] ./scripts/deploy-reentrancy-guard.sh
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE="${SOURCE:?Set SOURCE to a Stellar CLI identity}"
ADMIN="${ADMIN:?Set ADMIN to the admin address}"
OPERATOR="${OPERATOR:?Set OPERATOR to the operator address}"
WASM="target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm"

cargo build --release --target wasm32-unknown-unknown -p apexchainx_calculator

CONTRACT_ID="$(stellar contract deploy --wasm "$WASM" --source "$SOURCE" --network "$NETWORK")"
echo "Deployed contract with ReentrancyGuard: $CONTRACT_ID"

stellar contract invoke --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
  -- initialize --admin "$ADMIN" --operator "$OPERATOR"
