# Storage Key Migration Checklist

> **When to use:** Any PR that adds a new `const *_KEY: Symbol` in
> `apexchainx_calculator/src/lib.rs`, removes an existing one, or changes
> what a key stores.

Storage keys are permanent on-chain identifiers. A key added today must be
readable (or safely absent) by every future contract version. Work through
the questions below before marking your PR ready for review.

---

## Checklist

- [ ] **Key is registered.** The new constant is defined in the
  `// Storage Keys` block at the top of `lib.rs` and follows the 9-character
  `symbol_short!` limit. Its doc comment explains what it stores and
  references the originating issue number.

- [ ] **Key fits the naming convention.** Check
  [`docs/RESERVED_KEYS_POLICY.md`](RESERVED_KEYS_POLICY.md) — the symbol
  prefix matches the correct category (e.g. `HIST` for history, `VER` for
  version, `CFG` for config). No existing key uses the same symbol string.

- [ ] **Default / absent state is handled.** Every read site uses
  `.get(&KEY).unwrap_or(default)` or returns a meaningful error when the
  key is absent. The contract must behave correctly on deployments that
  pre-date this key.

- [ ] **Migration arm added.** If existing live deployments will not have
  this key, a `if current == N { ... }` arm is added to `migrate()` in
  `lib.rs` that writes the key's initial value and bumps `STORAGE_VERSION`.
  `STORAGE_VERSION` is incremented in the same commit.

- [ ] **`init_missing_storage_defaults` updated.** If the key must be
  present after a fresh `initialize()` call, it is written there too.

- [ ] **Tests cover the migration path.** At minimum: a test that calls
  `migrate()` on a contract state that lacks the new key, then reads the
  key and asserts it has the expected default value.

---

## Quick decision table

| Scenario | Action required |
|----------|----------------|
| New key, fresh deploy only | Handle absent gracefully; no migration arm needed |
| New key, must exist on upgraded contract | Add migration arm + bump `STORAGE_VERSION` |
| Key renamed | Treat as remove + add; migration arm copies old → new, then clears old |
| Key removed | Migration arm deletes it; document removal in `CHANGELOG.md` |
