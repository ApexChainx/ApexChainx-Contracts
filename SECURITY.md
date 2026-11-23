# Security Policy

## Supported Versions

Use this section to tell people about which versions of your project are currently being supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| v1.0.x  | :white_check_mark: |
| < v1.0  | :x:                |

## Scope

This security policy applies to all smart contracts and related software within the ApexChainx-Contracts repository.

## Dependency Updates

Dependency updates that may introduce security or compatibility impacts
should use the **Dependency Security Review** issue template before
opening a pull request.

This helps maintainers evaluate security, compatibility, licensing,
and supply-chain risks before changes are merged.

## Release Validation Checklist

Before promoting any release candidate to production, the following
validation gates MUST pass (enforced by the
`release-validation.yml` CI workflow):

### Build & Artifact Integrity

- [ ] **Native build succeeds** — `cargo build` completes without errors
- [ ] **WASM build succeeds** — `cargo build --target wasm32-unknown-unknown --release` completes
- [ ] **WASM artifact hash is deterministic** — `sha256sum` of the WASM binary is recorded in the release validation report
- [ ] **Artifact hash self-consistency verified** — `sha256sum -c` passes on the recorded hash
- [ ] **no_std compliance** — `./scripts/check-no-std.sh` passes (no accidental `std::` imports)
- [ ] **WASM no-std compliance** — `cargo check --target wasm32-unknown-unknown --lib` passes

### Code Quality

- [ ] **Formatting** — `cargo fmt -- --check` confirms formatting compliance
- [ ] **Linting** — `cargo clippy --all-targets -- -D warnings` produces no warnings
- [ ] **No unused dependencies** — `cargo machete` and `cargo udeps` pass

### Testing

- [ ] **All unit tests pass** — `cargo test --lib` reports `test result: ok`
- [ ] **Fuzz tests pass** — `cargo test --lib fuzz_tests::` reports `test result: ok`

### Smoke-Test Expectations

- [ ] **Contract initializes** — `initialize(admin, operator)` succeeds
- [ ] **Configuration works** — `set_config` with valid parameters succeeds
- [ ] **SLA calculation works** — `calculate_sla` with valid inputs returns expected result
- [ ] **Stats query works** — `get_stats` returns valid statistics
- [ ] **History pruning works** — `prune_history` with valid parameters succeeds

### Reporting

- [ ] **Validation report generated** — `release-validation-report` artifact contains the full report
- [ ] **Artifact hash recorded** — WASM hash is included in the report for downstream verification

### Promoting a Release

1. Ensure the `Release Validation` CI workflow passes on the target commit
2. Download the `release-validation-report` artifact from the CI run
3. Verify the WASM artifact hash matches any previous release audit trail
4. Create a GitHub Release with the WASM artifact and `manifest.sha256`
5. Update `CHANGELOG.md` with the release notes

---
### Binary Provenance

Release WASM artifacts are governed by the [WASM Binary Reproducibility Policy](docs/WASM_REPRODUCIBILITY_POLICY.md). Every release includes a SHA-256 manifest that links the deployed bytecode to its exact build inputs. Maintainers and security reviewers should verify the published checksum against a local build before deploying.

## Reporting a Vulnerability

We take the security of our smart contracts seriously. If you believe you have found a security vulnerability in our contracts, please report it to us as described below.

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please email your findings to [security@apexchainx.example.com]. 

### Secure Reporting Method

You can encrypt your communication using our PGP key:
```
-----BEGIN PGP PUBLIC KEY BLOCK-----
...
-----END PGP PUBLIC KEY BLOCK-----
```
*(Please contact us for the actual PGP key if needed, or use the email provided above securely.)*

### Response Timeline

- We will acknowledge receipt of your vulnerability report within 48 hours.
- We will provide a more detailed response to your report within 7 days, including whether we have reproduced the issue and an estimated timeline for remediation.
- We aim to resolve critical issues within 14 days of confirmation.

## Disclosure Policy

We ask that you do not disclose the vulnerability publicly until we have had a chance to address it. We will notify you when the issue has been patched and is safe to disclose. We appreciate your cooperation in keeping our ecosystem secure.
