# ApexChainx Reserved Keys & Event Topics Policy

To ensure forward compatibility, maintainable contract upgrades, and secure storage migrations, all Soroban smart contracts within `ApexChainx-Contracts` must adhere to strict symbol and storage key conventions.

## 1. Reserved Storage Keys
Storage keys are strictly partitioned by prefix to prevent data collision between configuration, governance, history, and telemetry states.

| Prefix / Symbol | Category | Description |
| :--- | :--- | :--- |
| `Config` / `CFG_*` | Configuration | Core immutable and mutable operational parameters. |
| `Gov` / `GOV_*` | Governance | Voting thresholds, admin authorities, and timelocks. |
| `Hist` / `HIST_*` | History / Ledger | Historical state snapshots and audit trails. |
| `Telemetry` / `TEL_*` | Telemetry | System metrics, counters, and monitoring states. |
| `Version` / `VER_*` | Version Control | Contract schema versions and protocol negotiation markers. |

* **Rule**: Direct string literals for storage keys are strictly prohibited outside of central symbol mapping modules. Use the defined enum/constant registries.

## 1a. Key additions are machine-checked (#602)
The set of on-chain instance-storage keys is a schema contract for backend
migrations. Two guards keep it honest:

* `api_stability::storage_key_symbols()` lists the authoritative set (now 22
  keys) and `assess_stability()` fails CI on any drift from that count.
* `storage_key_invariant_tests::test_storage_key_set_pinned_to_version_snapshot_or_newer`
  pins the set observed at each `STORAGE_VERSION` and fails CI if the set
  changes without a matching `STORAGE_VERSION` bump. **Adding or removing a
  key is a schema change**: bump `STORAGE_VERSION` (and update the pinned
  snapshot) in the same commit, and announce the key here and in the
  release notes.

## 2. Reserved Event-Topic Symbols
Event topics follow a namespaced dot-notation format (`domain.action.status`) to allow reliable frontend indexing and automated analytics ingestion.

* `apexchainx.config.*`: Configuration updates, parameter changes, and admin rotations.
* `apexchainx.governance.*`: Proposal submissions, votes cast, and execution events.
* `apexchainx.calculator.*`: Core computation events, state transitions, and calculation logs.
* `apexchainx.system.*`: Pause states, emergency shutdowns, and telemetry heartbeats.

* **Rule**: Event topics must never exceed 3 hierarchical segments. The root namespace `apexchainx` is strictly reserved for core protocol events.