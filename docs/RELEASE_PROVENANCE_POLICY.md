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

### 2.2 SHA-256 Checksum Provenance (One-Line Commands)

The `justfile` provides contributor-friendly recipes for hash management:

**Generate and save hash to `artifacts/`:**
```bash
just hash-save
```

**Verify hash against committed file:**
```bash
just hash-verify
```

**Display hash without saving:**
```bash
just hash
```

These commands handle cross-platform differences (Linux `sha256sum` vs macOS `shasum`) automatically and ensure the hash file is stored in the correct location (`artifacts/apexchainx_calculator.wasm.sha256`).

### 2.3 Verification Policy

CI validation checks that the compiled WASM checksum matches the committed provenance artifact. Contributors should run `just hash-verify` before committing to ensure their local build matches the committed hash.

If the hash verification fails:
1. Check if contract code changed — if so, run `just hash-save` to update the committed hash
2. Check if toolchain changed — ensure `rust-toolchain.toml` is pinned
3. Check for non-deterministic builds — verify no timestamp or random values in contract

---

## 3. Snapshot Check-in Guidelines

1. **Deterministic Normalization**: All test snapshot outputs must normalize timestamps, OS file separators (`/` vs `\`), and line endings (`LF` vs `CRLF`).
2. **Atomic Commits**: Snapshot updates must be checked in alongside the code change that altered contract output or state structure.
3. **No Drift in CI**: Pull requests with uncommitted or non-reproducible snapshot changes will fail the CI `snapshot-check` job.

### Contributor Workflow

Use the `justfile` recipes for snapshot management:

- `just normalize-snapshots` — Normalize existing snapshots in place
- `just regenerate-snapshots` — Run tests then normalize (full regeneration)
- `just verify-snapshots` — Verify snapshots are normalized (dry-run check)

Before committing snapshot changes, run `just verify-snapshots` to ensure they match the normalized POSIX standard.

---

## 4. Compliance Checklist for Release PRs

- [ ] WASM output compiled with pinned toolchain (`rust-toolchain.toml`).
- [ ] SHA-256 hash verified and saved in `artifacts/`.
- [ ] Snapshot files normalized to POSIX LF line endings.
- [ ] All contract tests and API compatibility checks pass cleanly.
