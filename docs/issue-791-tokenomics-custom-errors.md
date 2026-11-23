# Issue #791: Add custom errors instead of require strings in Tokenomics

Status: not applicable in this repository snapshot.

A repository-wide search confirms there is no `Tokenomics` contract or module in
this checkout. The current workspace contains the Soroban SLA calculator
(`apexchainx_calculator`) and the staking contract (`apexchainx_staking`); neither
is a Tokenomics module.

The existing contracts already use typed `#[contracterror]` enums (e.g.
`ContractError` in `apexchainx_calculator` and `StakingError` in
`apexchainx_staking`) rather than `require`-style string panics, which is the
Soroban-native equivalent of Solidity custom errors.

This file is included to keep the contributor PR flow tied to the upstream issue
and to explicitly document the repository mismatch for issue tracking.

Fixes #791
