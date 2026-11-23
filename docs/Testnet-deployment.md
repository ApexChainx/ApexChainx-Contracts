Testnet deployment is manual-only with no continuous devnet
Repo Avatar
ApexChainx/ApexChainx-Contracts
Context
testnet-deploy.yml is a workflow_dispatch job; there is no scheduled/nightly deploy to a permanent devnet instance.

Problem
An hour-coordinated deploy is a release artifact no automated integration relies on. Parity and offchain scripts must run against simulation-grade environments, so real Testnet behavior (rent, fees, horizon latency) is only exercised on demand. If Testnet drift breaks the contract, nothing short of a human noticing will flag it.

Proposed approach
Add a scheduled (or post-merge) deploy plus a health-check workflow that runs the parity/read-cost suite against the deployed instance and files artifacts.