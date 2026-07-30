# Devcontainer Setup

> Reproducible development environment for ApexChainx smart contracts.
> Works with GitHub Codespaces and VS Code Dev Containers.

## What's Included

| Tool | Purpose |
|------|---------|
| Rust (latest) | Contract compilation and testing |
| `wasm32-unknown-unknown` target | Soroban WASM builds |
| `just` command runner | One-command dev workflows |
| Node.js LTS | TypeScript tooling scripts |
| `cargo clippy` | Linting on save |
| `rust-analyzer` | IDE support |

## Getting Started

### GitHub Codespaces

1. Click **Code** → **Codespaces** → **Create codespace on main**
2. Wait for the environment to build (~3–5 minutes first time)
3. The post-create script runs `just bootstrap` automatically
4. Run `just ci` to verify everything works

### VS Code (Local)

1. Install [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
2. Open the repo folder in VS Code
3. Click **Reopen in Container** when prompted
4. Run `just ci` to verify

## Daily Workflow

```bash
# Before committing
just fmt              # Auto-format code
just lint             # Run clippy
just check            # Type-check

# Before opening a PR
just ci               # Full CI pipeline locally

# Fast release validation
just release-replay   # Minimal validation (fast)

# Generate a release summary
just release-summary  # From [Unreleased] in CHANGELOG
```

## CI Parity

The devcontainer mirrors the CI environment (`ubuntu-latest`, Rust stable,
`wasm32-unknown-unknown` target). Run `just ci` to reproduce the exact
sequence that CI gates on before opening a PR.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `cargo: command not found` | Rebuild the devcontainer |
| WASM target missing | Run `just bootstrap` |
| `npx` not found | Ensure Node.js feature was installed |
| Build errors after pull | Run `cargo clean && just ci` |
