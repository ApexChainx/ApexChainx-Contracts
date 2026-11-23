# Off-Chain Scripts

> **Issue:** [#650](https://github.com/ApexChainx/ApexChainx-Contracts/issues/650)

Off-chain validation scripts live in the repo root and are invoked via `npm run test:offchain`.
This document records the purpose, output, cost bounds, and CI gating status of each script.

## Script catalogue

| npm script | File | What it checks | CI-gated? |
|---|---|---|---|
| `test:offchain` | runs all below | All off-chain validations | Yes |
| `test:parity` | `ts/parity.ts` | TypeScript ↔ contract output fixture parity | Yes |
| `test:governance` | `offchain/governanceConsistency.ts` | Governance event ordering and state consistency | Yes |
| `test:eventSize` | `offchain/eventSizeRegression.ts` | Event payload size stays within budget | Yes |
| `test:readCost` | `offchain/readCostRegression.ts` | Read-path cost regression against baseline | Yes |
| `test:metadata` | `offchain/contractMetadata.ts` | Contract metadata shape matches declared surface | No |

## Cost bounds

| Script | Bound | Action on breach |
|---|---|---|
| `eventSizeRegression` | Each event payload ≤ declared byte budget (see `offchain/eventSizeRegression.ts`) | CI fails; update budget with documented justification |
| `readCostRegression` | Per-endpoint read cost ≤ baseline + 10% | CI fails; update baseline with regression note |

## Ownership

Off-chain scripts are owned by the backend/contract interface team. Changes
to the contract surface that affect event shapes or read costs must update
the corresponding script baseline and this document in the same PR.