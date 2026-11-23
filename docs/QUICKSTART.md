## Developer Quickstart

### Prerequisites

- **Rust** (see `rust-toolchain.toml` for the pinned version)
- **`just`** — task runner: `cargo install just`
- **Node.js ≥ 18** — for off-chain parity scripts: `npm install`
- **Stellar CLI** — for contract build: `cargo install stellar-cli --locked`

### First-time setup

```bash
git clone https://github.com/ApexChainx/ApexChainx-Contracts.git
cd ApexChainx-Contracts
npm install          # install off-chain script dependencies
rustup show          # verify toolchain from rust-toolchain.toml
```

### Common `just` targets

| Command | What it does |
|---|---|
| `just build` | Compile all Soroban WASM contracts |
| `just test` | Run the full Rust test suite |
| `just parity-check` | Validate TypeScript ↔ contract output parity |
| `just lint` | Run `cargo clippy` and `cargo fmt --check` |
| `just fuzz` | Run fuzz targets (requires nightly Rust) |

### Off-chain npm scripts

| Command | What it checks |
|---|---|
| `npm run test:offchain` | All off-chain validation scripts |
| `npm run test:parity` | TS ↔ contract parity fixtures |
| `npm run test:governance` | Governance consistency assertions |

### Pre-push validation loop

```bash
just lint && just test && just parity-check
```

Run this before every `git push` to catch issues locally rather than in CI.