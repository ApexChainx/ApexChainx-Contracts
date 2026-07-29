# Standalone Contract API Compatibility Verifier

This document outlines the API compatibility protocol and verification suite for backend adapters integrating with the `apexchainx_calculator` Soroban contract.

---

## 1. Objective

Backend services (`apexchainx-be`) require stable, non-breaking contract method signatures and event schemas. The Standalone API Compatibility Verifier ensures that:
1. Public contract entry points (`calculate_sla`, `get_version_info`, `get_config`, `set_config`, `pause`, `unpause`) preserve their argument types and return schemas.
2. Version negotiation protocols return expected semantic version tags.
3. Event topic signatures (`"sla_calc"`, `"config_set"`, `"pause"`) maintain backwards compatibility.

---

## 2. Public API Signature Specification

| Function | Parameters | Return Type | Access Level |
| :--- | :--- | :--- | :--- |
| `get_version_info` | `env: Env` | `VersionInfo` | Public |
| `calculate_sla` | `env: Env, severity: u32, mttr: u32` | `SlaResult` | Public |
| `get_config` | `env: Env` | `Config` | Public |
| `set_config` | `env: Env, admin: Address, config: Config` | `()` | Admin-Gated |

---

## 3. Verifier Execution

Run the standalone compatibility verifier test suite:
```bash
cargo test --package apexchainx_calculator --lib api_compatibility_tests
```

Any breaking signature change will fail compilation or trigger assertion errors before PR merge.
