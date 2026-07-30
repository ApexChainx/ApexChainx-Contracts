# WASM Binary Reproducibility Policy

> **Issue:** [#246](https://github.com/ApexChainx/ApexChainx-Contracts/issues/246)
> **Status:** Active
> **Applies to:** All release WASM artifacts produced by this repository

This document defines the release WASM binary governance policy: what inputs are
used to build each release artifact, how checksums are recorded, and how
maintainers and external reviewers can independently verify reproducibility.

---

## Table of Contents

- [1. Purpose & Scope](#1-purpose--scope)
- [2. Build Input Surface](#2-build-input-surface)
- [3. Artifact Checksum Provenance](#3-artifact-checksum-provenance)
- [4. CI Release Workflow](#4-ci-release-workflow)
- [5. Maintainer One-Step Verification](#5-maintainer-one-step-verification)
- [6. Reproducibility Assumptions & Known Gaps](#6-reproducibility-assumptions--known-gaps)
- [7. Compliance Checklist for Release PRs](#7-compliance-checklist-for-release-prs)
- [8. Related Documents](#8-related-documents)

---

## 1. Purpose & Scope

Every release of the ApexChainx smart contracts deploys a WASM binary
(`apexchainx_calculator.wasm`) to the Stellar network via Soroban. This binary
is the deployed bytecode path and must be reviewable with a clear provenance
trail.

This policy:

- **Records** the exact inputs used to produce each release WASM artifact.
- **Publishes** a cryptographic SHA-256 checksum alongside every release.
- **Documents** the reproducibility assumptions known to affect byte-for-byte
  reproduction.
- **Provides** a one-step verification command for maintainers.

---

## 2. Build Input Surface

The following inputs are the authoritative set for any release WASM build.
Every release tag records these inputs so an independent reviewer can reproduce
the artifact byte-for-byte.

### 2.1 Pinned Toolchain

| Input | Value | Source |
|-------|-------|--------|
| Rust toolchain channel | `1.94.1` | [`rust-toolchain.toml`](../rust-toolchain.toml) |
| WASM target | `wasm32-unknown-unknown` | CI workflow (`release-hash.yml`) |

The Rust toolchain version is pinned in the repository root via
`rust-toolchain.toml`. CI and local builds use the same channel. Changing the
toolchain version requires a version-bump PR that updates this file and the CI
workflow's `dtolnay/rust-toolchain` version reference.

### 2.2 Dependency Lockfile

| Input | Value | Source |
|-------|-------|--------|
| Cargo lockfile | [`Cargo.lock`](../Cargo.lock) | Committed to repository |

The `Cargo.lock` file is committed to version control and pins every transitive
dependency at an exact version and checksum. The release WASM is built from
these pinned dependencies — not from the latest compatible versions.

### 2.3 Core SDK & Build Configuration

| Input | Value | Source |
|-------|-------|--------|
| Soroban SDK version | `21.1.0` (declared) / `21.7.7` (resolved) | [`apexchainx_calculator/Cargo.toml`](../apexchainx_calculator/Cargo.toml), `Cargo.lock` |
| Crate type | `cdylib` | `Cargo.toml` |
| Build profile | `--release` | CI workflow |
| Target | `wasm32-unknown-unknown` | CI workflow |
| Cargo resolver | `2` | workspace `Cargo.toml` |

### 2.4 Source Code

The complete source tree at the release tag is part of the build input. This
includes:

- All `.rs` source files under `apexchainx_calculator/src/`
- The workspace `Cargo.toml`
- The crate `Cargo.toml`

### 2.5 Build Command

The canonical build command used by CI and expected for local reproduction:

```bash
cargo build --release --target wasm32-unknown-unknown
```

Run from the `apexchainx_calculator/` directory.

---

## 3. Artifact Checksum Provenance

### 3.1 SHA-256 Manifest

Every release produces a `manifest.sha256` file containing the SHA-256 hash
of the WASM binary:

```
<sha256_hex>  apexchainx_calculator.wasm
```

This manifest is:

1. **Attached to the GitHub Release** via `softprops/action-gh-release@v2`
2. **Uploaded as a workflow artifact** named `release-hash-manifest` (retained
   for 90 days)
3. **Self-verified** in CI — the workflow copies the WASM, runs
   `sha256sum -c manifest.sha256`, and fails the job on mismatch

### 3.2 WASM Artifact Location

The release WASM binary is produced at:

```
apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm
```

The CI workflow logs the final artifact size (in bytes) during the build step
for quick visual verification.

### 3.3 Checksum Computation (Local)

To compute the SHA-256 checksum locally:

```bash
sha256sum apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm
```

Alternatively, use the Node.js attestation tool:

```bash
npx ts-node tooling/wasmAttestation.ts
```

---

## 4. CI Release Workflow

The release hash workflow (`.github/workflows/release-hash.yml`) runs on every
`v*` tag push. It performs these steps in order:

| Step | Action | Purpose |
|------|--------|---------|
| 1 | Checkout source at tag | Pin source tree |
| 2 | Install Rust `1.94.1` + `wasm32-unknown-unknown` | Pin toolchain |
| 3 | Restore Cargo cache | Speed up rebuild |
| 4 | `cargo build --release --target wasm32-unknown-unknown` | Produce WASM binary |
| 5 | `sha256sum` → `manifest.sha256` | Generate checksum |
| 6 | `sha256sum -c manifest.sha256` | Self-verify checksum |
| 7 | Upload manifest + WASM as workflow artifact | Persist for 90 days |
| 8 | Attach manifest + WASM to GitHub Release | Publish provenance |

### 4.1 Predictable Hash Emission

The CI emits the artifact hash in a **predictable format**:

- **File name:** `manifest.sha256`
- **Format:** `<hex-64-chars>  apexchainx_calculator.wasm`
- **Encoding:** ASCII, LF line ending
- **Algorithm:** SHA-256 (as produced by GNU `sha256sum`)

Backend automation and security scanners can consume this file directly.

---

## 5. Maintainer One-Step Verification

A maintainer can verify that a local build reproduces the published artifact
with a single command:

```bash
cargo build --release --target wasm32-unknown-unknown \
  && sha256sum -c <(curl -sL https://github.com/ApexChainx/ApexChainx-Contracts/releases/latest/download/manifest.sha256)
```

**What this does:**

1. Builds the WASM binary using the pinned toolchain
2. Downloads the published manifest from the latest GitHub Release
3. Verifies the local checksum matches the published one

**Expected result:** `apexchainx_calculator.wasm: OK`

If the verification fails, check:

- Rust toolchain version matches `rust-toolchain.toml` (`rustc --version`)
- `Cargo.lock` is clean and matches the release tag
- Build is performed from the `apexchainx_calculator/` directory inside the
  repository root

---

## 6. Reproducibility Assumptions & Known Gaps

### 6.1 Assumptions That Hold

| Assumption | Status |
|------------|--------|
| Toolchain is pinned via `rust-toolchain.toml` | ✅ Enforced |
| `Cargo.lock` is committed | ✅ Enforced |
| Build command is standardised | ✅ Enforced |
| CI self-verifies the checksum | ✅ Enforced |
| Release artifacts include the manifest | ✅ Enforced |

### 6.2 Known Gaps

| Gap | Impact | Mitigation |
|-----|--------|------------|
| Cargo build is not fully deterministic across OS/host triple | Same toolchain + same lockfile on different host OS may produce a different WASM due to path-based hashing in some proc-macros | CI builds on `ubuntu-latest` (Linux); local verification should use Linux or the Soroban CLI's `--verify` mode |
| No content-addressable build cache published | Reviewers must rebuild from source | The build is fast (< 2 min with cache) and CI publishes the artifact directly |
| Cargo registry state not captured | If a crate is yanked after a release, `Cargo.lock` alone may not be sufficient to reproduce | The Soroban ecosystem publishes all SDK crates; no history of yanked SDK releases |

### 6.3 Future Improvements

- [ ] Publish a Docker-based reproducible build environment (Dockerfile with
  pinned Rust image)
- [ ] Integrate `cargo supply-chain` or `cargo-vet` for dependency auditing
- [ ] Add a `reproduce.sh` script that automates the full verification flow

---

## 7. Compliance Checklist for Release PRs

Before merging a PR that will be tagged for release, confirm:

- [ ] `rust-toolchain.toml` channel matches CI workflow toolchain version
- [ ] `Cargo.lock` is committed and up to date (`cargo update` was run
  intentionally if dependencies changed)
- [ ] `cargo build --release --target wasm32-unknown-unknown` succeeds locally
- [ ] Local SHA-256 checksum is computed and noted in the PR description
- [ ] All contract tests pass (`cargo test`)
- [ ] No uncommitted changes to source files or `Cargo.lock`
- [ ] The release tag will trigger the `release-hash.yml` workflow

---

## 8. Related Documents

- [Release Artifact Provenance Policy](RELEASE_PROVENANCE_POLICY.md) —
  General provenance, snapshot check-in, and WASM compilation rules
- [Security Policy](../SECURITY.md) — Vulnerability reporting and supported
  versions
- [CI Workflow: release-hash.yml](../.github/workflows/release-hash.yml) —
  The CI job that implements this policy
- [`rust-toolchain.toml`](../rust-toolchain.toml) — Pinned Rust toolchain
- [WASM Attestation Tool](../tooling/wasmAttestation.ts) — Node.js script for
  checksum computation and verification
