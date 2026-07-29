# Release Artifact Provenance Policy

This document defines the release artifact provenance requirements, WASM compilation hash verification rules, and snapshot check-in standards for the **ApexChainx Contracts** codebase.

---

## 1. Overview & Purpose

To maintain deterministic, auditable, and secure releases of Soroban smart contract WASM artifacts, all release outputs must conform to explicit build provenance guidelines:
- Every release `.wasm` binary must be compiled in a reproducible environment.
- Cryptographic SHA-256 checksums must be recorded and checked into `artifacts/` or GitHub release metadata.
- Snapshot test files must follow deterministic normalization rules to prevent environmental drift across local dev (Windows/macOS) and CI (Linux) environments.

---

## 2. Soroban WASM Build & Verification Workflow

### 2.1 Compilation Command
Contract WASM binaries must be compiled using standard Soroban tooling:
```bash
cargo build --target wasm32-unknown-unknown --release
```

### 2.2 SHA-256 Checksum Provenance
Generate and verify SHA-256 checksums for target outputs:
```bash
sha256sum target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm > artifacts/apexchainx_calculator.wasm.sha256
```

### 2.3 Verification Policy
CI validation checks that the compiled WASM checksum matches the committed provenance artifact:
```bash
sha256sum -c artifacts/apexchainx_calculator.wasm.sha256
```

---

## 3. Snapshot Check-in Guidelines

1. **Deterministic Normalization**: All test snapshot outputs must normalize timestamps, OS file separators (`/` vs `\`), and line endings (`LF` vs `CRLF`).
2. **Atomic Commits**: Snapshot updates must be checked in alongside the code change that altered contract output or state structure.
3. **No Drift in CI**: Pull requests with uncommitted or non-reproducible snapshot changes will fail the CI `snapshot-check` job.

---

## 4. Compliance Checklist for Release PRs

- [ ] WASM output compiled with pinned toolchain (`rust-toolchain.toml`).
- [ ] SHA-256 hash verified and saved in `artifacts/`.
- [ ] Snapshot files normalized to POSIX LF line endings.
- [ ] All contract tests and API compatibility checks pass cleanly.
