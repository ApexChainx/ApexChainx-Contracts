# Release Artifacts

This directory contains provenance artifacts for release builds, including cryptographic hashes of compiled WASM binaries.

## WASM Hash File

The `apexchainx_calculator.wasm.sha256` file contains the SHA-256 checksum of the release WASM binary. This file is used for:

- Verifying build reproducibility across different environments
- Ensuring CI builds match local development builds
- Providing auditable provenance for releases

## Generating the Hash File

To generate the hash file after building the release WASM:

```bash
just hash-save
```

This command:
1. Builds the release WASM (`cargo build --target wasm32-unknown-unknown --release`)
2. Computes the SHA-256 checksum
3. Saves it to `artifacts/apexchainx_calculator.wasm.sha256`

## Verifying the Hash

To verify your local build matches the committed hash:

```bash
just hash-verify
```

This will fail if your build produces a different WASM binary than the committed hash indicates.

## When to Update the Hash

Update the committed hash file when:
- Contract code changes that affect the WASM output
- Dependencies change that affect the build
- The toolchain version is intentionally updated

Do NOT update the hash for:
- Non-deterministic build issues (fix those instead)
- Temporary local changes not meant for commit
- Changes that should be reverted
