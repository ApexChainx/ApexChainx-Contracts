# Rust Toolchain Pinning for Reproducible Builds

## Problem Statement

Previously, all GitHub Actions workflows used `dtolnay/rust-toolchain@stable`, which floats to whatever the latest stable Rust release is at the moment of CI execution. This caused several critical issues:

### Issues with Floating Toolchain

1. **Non-Reproducible Builds**: Two builds a week apart could produce different WASM bytecode
2. **Hash Manifest Breakage**: SHA-256 manifest produced by `release-hash.yml` would change unexpectedly
3. **Bump Night Volatility**: Changes to wasm-encoder or soroban-sdk re-exports after Rust releases would alter output
4. **Broken Reproducibility Promise**: Issue #8 promised reproducible builds, but floating toolchain violated this

### Example Failure Scenario

```
Week 1 (Rust 1.94.0):
  cargo build --release → apexchainx_calculator.wasm
  SHA-256: abc123...

Week 2 (Rust 1.94.1 released):
  cargo build --release → apexchainx_calculator.wasm
  SHA-256: def456... ← DIFFERENT!
```

This breaks:
- Deployment verification
- Audit trail integrity
- Reproducible build guarantees
- CI/CD stability

---

## Solution

Pin the Rust toolchain to a specific version across all workflows and local development.

### Changes Made

#### 1. Created `rust-toolchain.toml` at Repository Root

```toml
[toolchain]
channel = "1.94.1"
```

**Purpose**: 
- Cargo and rustup automatically respect this file
- Ensures local development matches CI environment
- Single source of truth for Rust version

**Benefits**:
- Developers automatically use correct version when running `cargo build`
- No manual configuration needed per developer
- Prevents "works on my machine" issues

#### 2. Updated GitHub Actions Workflows

**Changed from:**
```yaml
uses: dtolnay/rust-toolchain@stable
```

**Changed to:**
```yaml
uses: dtolnay/rust-toolchain@1.94.1
```

**Files Modified:**
- `.github/workflows/ci.yml`
- `.github/workflows/release-hash.yml`
- `.github/workflows/security.yml`

---

## Why Rust 1.94.1?

1. **Current Stable**: Latest stable release as of implementation (March 2026)
2. **Soroban SDK Compatibility**: Compatible with soroban-sdk v21.0.0
3. **Tested & Verified**: Already in use on development systems
4. **Recent Features**: Includes modern Rust features and optimizations

---

## Reproducibility Guarantees

With this change, the following are now **guaranteed**:

### ✅ Deterministic WASM Bytecode
Same source code + same toolchain = identical WASM output

### ✅ Stable SHA-256 Manifests
`release-hash.yml` will produce consistent hashes across builds:
```bash
# Build on Monday
sha256sum apexchainx_calculator.wasm
# abc123def456...

# Build on Friday (same code)
sha256sum apexchainx_calculator.wasm
# abc123def456... ← IDENTICAL
```

### ✅ CI/Local Parity
CI builds match local developer builds exactly

### ✅ Audit Trail Integrity
Historical builds remain reproducible for security audits

---

## Maintenance & Upgrades

### When to Update Rust Version

Update the pinned version when:
1. **Security patches**: Critical Rust security vulnerabilities
2. **Soroban SDK requirements**: New SDK version requires newer Rust
3. **Desired features**: Team decides to adopt new Rust features
4. **Scheduled maintenance**: Quarterly or semi-annual updates

### How to Update

1. **Test locally first:**
   ```bash
   # Update rust-toolchain.toml
   channel = "1.95.0"
   
   # Build and test
   cargo build --release --target wasm32-unknown-unknown
   cargo test
   ```

2. **Update all workflows:**
   ```yaml
   uses: dtolnay/rust-toolchain@1.95.0
   ```

3. **Verify reproducibility:**
   ```bash
   # Build twice and compare hashes
   cargo clean
   cargo build --release --target wasm32-unknown-unknown
   sha256sum target/wasm32-unknown-unknown/release/*.wasm > hash1.txt
   
   cargo clean
   cargo build --release --target wasm32-unknown-unknown
   sha256sum target/wasm32-unknown-unknown/release/*.wasm > hash2.txt
   
   diff hash1.txt hash2.txt  # Should be identical
   ```

4. **Document the change:**
   - Update CHANGELOG.md
   - Note any breaking changes or new features enabled
   - Update this document with new version rationale

### Version Update Checklist

- [ ] Test new Rust version locally
- [ ] Verify WASM builds successfully
- [ ] Run full test suite
- [ ] Update `rust-toolchain.toml`
- [ ] Update all three workflow files
- [ ] Verify SHA-256 reproducibility
- [ ] Update documentation
- [ ] Create PR with clear rationale

---

## Verification

### Verify Local Toolchain

```bash
# Check current Rust version
rustc --version
# Should output: rustc 1.94.1 (e408947bf 2026-03-25)

# Cargo respects rust-toolchain.toml automatically
cargo --version
```

### Verify Reproducible Builds

```bash
# Build WASM twice and compare
cargo clean
cargo build --release --manifest-path apexchainx_calculator/Cargo.toml --target wasm32-unknown-unknown
sha256sum apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm

cargo clean
cargo build --release --manifest-path apexchainx_calculator/Cargo.toml --target wasm32-unknown-unknown
sha256sum apexchainx_calculator/target/wasm32-unknown-unknown/release/apexchainx_calculator.wasm

# SHA-256 hashes MUST match
```

### Verify CI Alignment

```bash
# Check workflow files
grep "rust-toolchain@" .github/workflows/*.yml

# All should show: dtolnay/rust-toolchain@1.94.1
```

---

## Impact Analysis

### Before Fix

| Aspect | Status |
|--------|--------|
| Build reproducibility | ❌ Not guaranteed |
| SHA-256 stability | ❌ Changes over time |
| CI/local parity | ❌ May differ |
| Audit trail | ❌ Unreliable |
| Deployment verification | ❌ Inconsistent |

### After Fix

| Aspect | Status |
|--------|--------|
| Build reproducibility | ✅ Guaranteed |
| SHA-256 stability | ✅ Stable |
| CI/local parity | ✅ Identical |
| Audit trail | ✅ Reliable |
| Deployment verification | ✅ Consistent |

---

## Related Issues

- **Issue #8**: Reproducibility promise - Now fulfilled
- **Release Hash Workflow**: SHA-256 manifests now stable
- **Security Audits**: Historical builds now reproducible

---

## Developer Experience

### No Action Required

Developers using `cargo` automatically get the correct Rust version:

```bash
# When you clone the repo and run cargo
git clone <repo>
cd ApexChainx-Contracts

# cargo automatically reads rust-toolchain.toml
cargo build
# rustup will download 1.94.1 if needed
```

### Manual Installation (if needed)

If you need to manually install Rust 1.94.1:

```bash
rustup install 1.94.1
rustup default 1.94.1
```

---

## Rollout Plan

1. **Immediate**: Pin to 1.94.1 (current stable)
2. **Verification**: Run CI and verify all workflows pass
3. **Documentation**: Update team on new reproducibility guarantees
4. **Monitoring**: Watch for any unexpected issues
5. **Future**: Schedule periodic Rust version reviews (quarterly)

---

## Conclusion

This change ensures:
- ✅ Reproducible builds across all environments
- ✅ Stable SHA-256 hash manifests
- ✅ CI/local development parity
- ✅ Fulfillment of reproducibility promises
- ✅ Reliable audit trail for security

The pinned toolchain eliminates a source of non-determinism and provides the foundation for trustworthy, verifiable builds.

---

**Last Updated**: 2026-06-21  
**Rust Version**: 1.94.1  
**Next Review**: 2026-09-21 (Quarterly)
