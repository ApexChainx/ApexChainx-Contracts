# Snapshot Normalization & Portability Standard

This document details the normalization procedures used to ensure test snapshots are portable and reproducible across different operating systems (Windows CRLF vs Linux LF, file path separators `\` vs `/`, timestamp offsets, and environmental variables).

---

## 1. Normalization Rules

When snapshot outputs are generated during contract tests:

1. **Line Endings**: Convert all Windows CRLF (`\r\n`) to POSIX LF (`\n`).
2. **File Paths**: Normalize Windows backslashes (`\`) to forward slashes (`/`).
3. **Ledger Timestamps**: Replace dynamic ledger timestamps with deterministic mock timestamps (`1700000000`) during snapshot comparison.
4. **ANSI Escapes**: Strip terminal color ANSI codes from test execution output logs.

---

## 2. Portable Normalization Script

Run the snapshot normalization script locally before submitting pull requests:

```bash
node scripts/normalize-snapshots.js
```

In CI, the `.github/workflows/ci.yml` pipeline automatically verifies that all snapshot files match the normalized POSIX standard.
