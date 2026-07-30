# Release-Readiness Checklist — Contract Shape Changes

> **When to use this checklist**
>
> Open this document any time a PR touches the on-chain storage schema,
> event topics, or event payload fields in:
>
> - `apexchainx_calculator/src/lib.rs` (storage keys, `STORAGE_VERSION`,
>   `RESULT_SCHEMA_VERSION`, event emission sites)
> - `apexchainx_calculator/src/event_schema.rs` (event-name constants,
>   payload doc comments)
> - `apexchainx_calculator/src/storage_version.rs` (version helpers,
>   migration flag logic)
>
> A storage or event-shape change is **invisible to a casual reviewer but
> dangerous in production**: wrong field order, a missing version bump, or a
> silent schema drift can break backend indexers and settlement reconciliation
> without a compilation error or test failure.
>
> **Both the PR author and the reviewer** must sign off on every item below
> before merge.

---

## 1. Storage Schema Changes

### 1.1 New or renamed storage keys

- [ ] Every new storage key is listed in `docs/RESERVED_KEYS_POLICY.md`
      and follows the defined prefix conventions.
- [ ] The new key does not collide with any existing key (search for the
      `symbol_short!` literal across `src/`).
- [ ] If the key holds structured data (a struct or tuple), the type's
      field layout is documented alongside the key definition.

### 1.2 Storage version bump

- [ ] `STORAGE_VERSION` in `lib.rs` has been incremented if and only if
      the change is incompatible with the previous on-chain layout.
      Purely additive changes that old binaries can read safely do **not**
      require a bump.
- [ ] `storage_version.rs` — the version number in the module-level
      `# Version Lifecycle` doc comment is updated to match.
- [ ] `initialize()` stamps the new `STORAGE_VERSION` constant.
- [ ] `get_migration_state()` correctly reports `needs_migration: true`
      when the stored version is older than the binary's constant.

### 1.3 Migration path

- [ ] A `migrate()` entrypoint exists, is admin-gated, and transforms
      all affected storage keys to the new layout.
- [ ] `migrate()` emits a `migrate_done` event with `(old_version,
      new_version)` in the payload.
- [ ] Migration is tested against a snapshot that represents the
      **previous** storage layout (not just the current one).
- [ ] `migration_flag` (`MIGRATED` key) is set to `true` at the end of a
      successful migration and is checked in `get_migration_state()`.
- [ ] The migration is idempotent: running it twice produces no error and
      leaves storage in a consistent state.

### 1.4 Required tests

- [ ] `cargo test --lib -- storage_version` — both
      `test_storage_version_unset_returns_none` and
      `test_migration_flag_defaults_to_false` still pass.
- [ ] A new test demonstrates round-trip correctness under the new layout.
- [ ] If a migration entrypoint was added or changed, at least one test
      exercises the full `initialize → migrate → get_migration_state`
      sequence.

---

## 2. Event Schema Changes

### 2.1 Topic constants

- [ ] No existing `EVENT_*` constant in `event_schema.rs` has been
      renamed or had its `symbol_short!` value changed.
      Renaming a topic constant is a **breaking change**.
- [ ] Any new event constant has been appended to the `names` array in
      `test_event_names_are_distinct` (in `event_schema.rs`) so the
      uniqueness test covers it.
- [ ] Every new event constant has at least one emission site in the same
      PR — a constant with no emission site is dead code.
- [ ] The 3-topic layout (`topic[0]` = name, `topic[1]` = version,
      `topic[2]` = context) is preserved. New metadata goes in the
      **payload**, not in the topics array.

### 2.2 Payload fields

- [ ] New fields are **appended** to the end of the payload tuple.
      Inserting, removing, or reordering fields is a **breaking change**.
- [ ] No field's Soroban type has changed (e.g. `u32` → `i128`).
      Type changes are breaking.
- [ ] Every emission site for the changed event emits the same tuple
      shape (field count and types are identical across all `env.events()
      .publish(...)` calls for that event).
- [ ] The event catalog doc comment in `event_schema.rs` (the
      `# Event Catalog` section) is updated to reflect the new payload
      signature: `(field: Type, ...)`.
- [ ] The corresponding row in `docs/EVENT_TOPIC_COMPATIBILITY.md` is
      updated if the payload or context type changed.

### 2.3 Version bump

- [ ] `EVENT_VERSION` (`"v1"`) has been incremented **if and only if**
      one of the following is true:
      - A field was **removed** from any payload.
      - A field's **type** changed.
      - Fields were **reordered**.
      Appending new fields or adding new event names does **not** require
      a bump.
- [ ] If the version was bumped, every `env.events().publish(...)` call
      uses the new version constant.
- [ ] `current_event_version()` returns the new version.
- [ ] `test_event_version_is_stable` in `event_schema.rs` has been
      updated to assert the new expected value.

### 2.4 Symbol deprecation

- [ ] If a result or severity symbol is being replaced, the
      Symbol Deprecation Protocol in `event_schema.rs` (Section
      `# Symbol Deprecation Protocol`) has been followed: old symbol
      enters the _introduction_ phase — both symbols emitted, old one
      marked deprecated in `get_result_schema()`.
- [ ] `RESULT_SCHEMA_VERSION` has been incremented if a previously
      deprecated symbol is being **removed** in this release.
- [ ] `deprecated_symbols` in the `SLAResultSchema` returned by
      `get_result_schema()` reflects the correct `deprecated_at` and
      `removed_at` values.

### 2.5 Required tests

- [ ] `cargo test --lib -- topic_stability_tests` passes.
- [ ] `cargo test --lib -- event_schema::tests` passes.
- [ ] For a breaking change (version bump), a test asserts that
      `EVENT_VERSION` matches the new expected symbol.
- [ ] Backend parity snapshot tests (`tests/contractResponseShapeParityFixtures.test.ts`)
      have been updated to reflect the new event structure.

---

## 3. Backend and Cross-Repo Coordination

- [ ] If event topics or payload fields changed, the `apexchainx-be`
      team has been notified before merge (open a tracking issue or
      comment on the PR).
- [ ] If a symbol is entering the deprecation lifecycle, a follow-up
      issue has been filed to track the removal milestone.
- [ ] `docs/EVENT_TOPIC_COMPATIBILITY.md` event catalog table is
      up-to-date.
- [ ] `docs/AUDIT_TRAIL.md` has a new entry recording the change,
      affected event names, and the reason.

---

## 4. Release Artifacts

- [ ] `CHANGELOG.md` has an entry under `[Unreleased]` in the
      appropriate section (`Added`, `Changed`, or `Removed`).
      Breaking changes are marked **(breaking)**.
- [ ] If `STORAGE_VERSION` was bumped, the entry names the new version
      number explicitly.
- [ ] If `EVENT_VERSION` was bumped, the entry names the new version
      symbol and which events were affected.

---

## 5. Final CI Gate

Run these locally before requesting review — they mirror the CI steps in
`.github/workflows/ci.yml` exactly (see `justfile` for convenience aliases):

```
just ci
```

Or individually:

```
just fmt-check    # cargo fmt --check
just lint         # cargo clippy --all-targets -- -D warnings
just check        # cargo check
just no-std       # cargo check --target wasm32-unknown-unknown --lib
just test         # cargo test --lib
just fuzz         # cargo test --lib fuzz_tests::
just wasm         # cargo build --target wasm32-unknown-unknown
```

All steps must be green before the PR can be approved.

---

## Related Documents

| Document | Covers |
|----------|--------|
| [`docs/EVENT_TOPIC_COMPATIBILITY.md`](EVENT_TOPIC_COMPATIBILITY.md) | Formal policy: topic layout, payload stability, versioning rules |
| [`docs/EVENT_COMPATIBILITY_POLICY.md`](EVENT_COMPATIBILITY_POLICY.md) | Append-only field ordering, type-stability guarantees |
| [`docs/RESERVED_KEYS_POLICY.md`](RESERVED_KEYS_POLICY.md) | Storage key namespace conventions |
| [`docs/AUDIT_TRAIL.md`](AUDIT_TRAIL.md) | Historical record of all schema changes |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) — SC-099 | Event-topic contributor safety checklist (pre-merge) |
| [`CONTRIBUTING.md`](../CONTRIBUTING.md) — SC-098 | Privileged-change security checklist (auth, config, storage) |
