# ApexChainx Cross-Contract Deployment Compatibility Checklist

To ensure safe upgrades, verified contract dependencies, and reliable cross-contract invocations across Soroban networks, all protocol deployments must verify the items in this checklist.

## 1. Pre-Deployment Interface Verification
* [ ] **Wasm Hash Verification**: Confirm the compiled Wasm bytecode hash matches across local release builds and staging artifacts.
* [ ] **Interface Compatibility**: Verify that function signatures, parameter ordering, and return types match expected host contract definitions.
* [ ] **Storage Key Partitioning**: Ensure new or upgraded contracts adhere to the `RESERVED_KEYS_POLICY.md` storage key prefixes to avoid collision.

## 2. Upgrade & Migration Posture
* [ ] **Version Negotiation**: Verify that version negotiation modules (`version.rs` or equivalent) correctly return the expected protocol version.
* [ ] **State Migration Plan**: If contract storage schemas are altered, verify that a dedicated migration entrypoint exists and has been tested against historical snapshots.
* [ ] **Event Schema Stability**: Confirm that event topics and payload field orderings comply with `EVENT_COMPATIBILITY_POLICY.md` (append-only fields).

## 3. Post-Deployment Verification
* [ ] **Initialization Guard**: Ensure initialization functions (`initialize`) can only be executed once and are protected against re-initialization.
* [ ] **Smoke Testing**: Execute post-deployment smoke tests on testnet/mainnet verifying core calculation and governance interactions.