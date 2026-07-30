# Release Summary Format

This document describes every section of a ship-review note produced by
`tooling/releaseSummary.ts` and explains what a maintainer should do with each
section during release triage.

The tool generates a note automatically from `CHANGELOG.md`. You can also
copy the template at the bottom of this file and fill it in by hand.

---

## How to generate a note

```bash
# Summarise the next (Unreleased) block — most common during PR review:
just release-summary

# Summarise a specific released version:
just release-summary 0.3.0

# Write to a file instead of stdout:
npx --yes tsx tooling/releaseSummary.ts --version 0.3.0 --out /tmp/review-0.3.0.md
```

---

## Section reference

### 1. New Public API Surface

All entries from the `### Added` category of the target changelog block.

Each entry represents a **new on-chain entrypoint, event, or type** visible to
callers. Review each one for:

| Check | Rationale |
|-------|-----------|
| Test coverage | Every new entrypoint should have at least one test in `tests/` or `apexchainx_calculator/src/tests.rs`. |
| Doc comment | The Rust `///` doc comment should match the changelog description. |
| Auth gating | Confirm whether the function should be admin-only, operator-only, or open. |
| Unintended surface | Make sure helper functions aren't accidentally exported via `pub`. |

---

### 2. Breaking Changes

All entries that contain the literal text `(breaking)` in the changelog. These
may come from `### Changed`, `### Removed`, or, rarely, `### Added`.

⚠️ **Any breaking change requires at minimum a minor-version bump for
pre-1.0 contracts and a major-version bump for stable contracts.**

| Check | Rationale |
|-------|-----------|
| Backend adapter compat | Cross-reference `docs/CONTRACT_API_COMPATIBILITY.md`. |
| Version negotiation | `get_version_info` response must reflect the new version. |
| Migration path | If storage layout changed, document how old entries are handled. |
| Event schema | If an event topic or payload changed, update `docs/EVENT_TOPIC_COMPATIBILITY.md`. |

---

### 3. Other Changes (non-breaking)

All `### Changed`, `### Fixed`, `### Removed`, and `### Security` entries that
are not marked `(breaking)`.

Review lightly for unintended scope creep and confirm that fixed bugs have a
corresponding regression test.

---

### 4. Storage Impact

Entries flagged by the tool because their text matches one or more of the
following patterns:

| Signal | Meaning |
|--------|---------|
| `storage_version` / `schema_version` | Layout or versioning change |
| `migrate` | Migration function touched |
| `DataKey` | New or renamed storage key |
| `prune_history` | Compaction / retention change |
| `retention_limit` | Configurable cap modified |
| `on-chain` | Explicit on-chain state reference |

> **Why this matters:** Soroban contracts cannot roll back on-chain state. A
> storage layout mistake requires a new contract deployment and a migration.

| Check | Rationale |
|-------|-----------|
| No unbounded growth | New maps or vectors must have an enforced cap. |
| Version bump | `get_storage_version` should return a new value if layout changed. |
| `migrate` updated | If the layout changed, the migration path must handle old state. |
| Retention respected | Any new history-style structures must respect `get_retention_limit`. |

---

### 5. Linked Tickets & Issues

All `#NNN` GitHub issue references and `SC-NNN` ticket references extracted
from the changelog block. Use this to verify that all referenced issues are
actually closed (or explicitly deferred) before tagging a release.

---

### 6. Open Questions

Free-form section for the reviewer to capture anything that needs discussion
before the release is approved:

- Known risks or concerns
- Items waiting on a separate PR
- Questions for the security reviewer
- Deferred work that must be tracked as a follow-up issue

---

### 7. Reviewer Sign-off

Table for reviewers to record their name, date, and any notes. At least one
maintainer with merge rights must sign off before a release tag is pushed.

---

## Manual template

Copy this template when you need a ship-review note for a change that is not
yet in CHANGELOG.md, or when you prefer to annotate inline.

```markdown
# Ship-Review Note: v<VERSION> — <TITLE>

> Manually authored on <DATE>.

---

## 1. New Public API Surface

- 

**Review checklist:**
- [ ] Tests present
- [ ] Doc comments accurate
- [ ] Auth gating correct
- [ ] No unintended public surface

---

## 2. Breaking Changes

_None_ / ⚠️ list breaking entries here

**Review checklist:**
- [ ] Backend adapter compatibility confirmed
- [ ] `get_version_info` updated
- [ ] Migration path documented

---

## 3. Other Changes (non-breaking)

- 

---

## 4. Storage Impact

_None_ / list storage-affecting entries here

**Review checklist:**
- [ ] No unbounded storage growth
- [ ] `get_storage_version` bumped if layout changed
- [ ] `migrate` function updated if required
- [ ] Retention limits respected

---

## 5. Linked Tickets & Issues

#, SC-

---

## 6. Open Questions

- [ ] ?

---

## 7. Reviewer Sign-off

| Reviewer | Date | Notes |
|----------|------|-------|
| | | |
```

---

## Relationship to other documents

| Document | How it relates |
|----------|---------------|
| `CHANGELOG.md` | Primary source; the tool parses this file |
| `docs/RELEASE_PROVENANCE_POLICY.md` | WASM hash and snapshot sign-off requirements |
| `docs/CONTRACT_API_COMPATIBILITY.md` | Breaking-change compatibility matrix |
| `docs/EVENT_TOPIC_COMPATIBILITY.md` | Event schema backward-compatibility rules |
| `tooling/releaseChecklist.ts` | Pre-tag gate checks (cargo test, WASM size, …) |
| `.github/PULL_REQUEST_TEMPLATE.md` | PR-level checklist; complementary to this note |
