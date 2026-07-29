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

## 2. Reserved Event-Topic Symbols
Event topics follow a namespaced dot-notation format (`domain.action.status`) to allow reliable frontend indexing and automated analytics ingestion.

* `apexchainx.config.*`: Configuration updates, parameter changes, and admin rotations.
* `apexchainx.governance.*`: Proposal submissions, votes cast, and execution events.
* `apexchainx.calculator.*`: Core computation events, state transitions, and calculation logs.
* `apexchainx.system.*`: Pause states, emergency shutdowns, and telemetry heartbeats.

* **Rule**: Event topics must never exceed 3 hierarchical segments. The root namespace `apexchainx` is strictly reserved for core protocol events.