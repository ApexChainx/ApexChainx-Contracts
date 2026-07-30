# Snapshot Normalization & Portability Standard

This document details the normalization procedures used to ensure test snapshots are portable and reproducible across different operating systems (Windows CRLF vs Linux LF, file path separators `\` vs `/`, timestamp offsets, and environmental variables).

---

## 1. Normalization Rules

When snapshot outputs are generated during contract tests:

1. **Line Endings**: Convert all Windows CRLF (`\r\n`) to POSIX LF (`\n`).
2. **File Paths**: Normalize Windows backslashes (`\`) to forward slashes (`/`).
3. **Ledger Timestamps**: Replace dynamic ledger timestamps with deterministic mock timestamps (`1700000000`) during snapshot comparison.
4. **ANSI Escapes**: Strip terminal color ANSI codes from test execution output logs.
5. **Volatile Fields**: Strip non-semantic fields (`timestamp`, `elapsed_ms`, `generated_at`) and sort JSON keys for deterministic output.

---

## 2. Contributor-First Workflow

The `justfile` provides three recipes for snapshot management:

### `just normalize-snapshots`

Normalize existing snapshot artifacts in place. Strips volatile fields and sorts keys. Run this after making contract changes that affect snapshot outputs.

```bash
just normalize-snapshots
```

### `just regenerate-snapshots`

Regenerate snapshots from scratch: runs the full test suite then normalizes the output. Use this when contract behavior changes significantly and snapshots need complete updating.

```bash
just regenerate-snapshots
```

### `just verify-snapshots`

Verify that snapshots are normalized without modifying files (dry-run check). Exits with error if snapshots need normalization. Use this in CI or before committing.

```bash
just verify-snapshots
```

---

## 3. CI Integration

In CI, the `.github/workflows/ci.yml` pipeline automatically:
1. Runs the E2E test suite (`cargo test --lib`)
2. Normalizes snapshot artifacts using `npx tsx tools/normalize-snapshots.ts`
3. Uploads normalized snapshots as artifacts for review

Contributors should run `just verify-snapshots` before pushing to ensure their snapshots match the normalized POSIX standard.
