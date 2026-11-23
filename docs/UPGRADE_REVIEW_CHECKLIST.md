# SC-103 – Admin-Facing Contract Upgrade Review Checklist

> **Audience:** Contract admins, multisig signers, governance reviewers.
>
> **Purpose:** A structured, non-technical checklist for reviewing a proposed
> contract upgrade before signing the upgrade transaction. This document
> assumes the reviewer is **not** the developer who wrote the upgrade,
> and may not be deeply familiar with the Rust codebase.

## Table of Contents

- [When to use this checklist](#when-to-use-this-checklist)
- [Pre-review: Gather context](#pre-review-gather-context)
- [Part 1 — What is changing?](#part-1--what-is-changing)
- [Part 2 — Storage & migration safety](#part-2--storage--migration-safety)
- [Part 3 — Access control & roles](#part-3--access-control--roles)
- [Part 4 — Event & API compatibility](#part-4--event--api-compatibility)
- [Part 5 — Pause & emergency recovery](#part-5--pause--emergency-recovery)
- [Part 6 — CI & test evidence](#part-6--ci--test-evidence)
- [Part 7 — Deployment plan](#part-7--deployment-plan)
- [Sign-off](#sign-off)

---

## When to use this checklist

Use this checklist when:

- A new Wasm binary of `apexchainx_calculator` (or any future contract
  crate) is proposed for deployment.
- An upgrade transaction is pending in a multisig wallet and needs
  signer approval.
- A storage migration (`migrate()`) will be executed as part of or
  immediately after the upgrade.

**Do not use this checklist** for configuration-only changes (calling
`set_config`, `set_custom_severity`, `freeze_config`, etc.) — those
are covered by the existing admin workflows.

---

## Pre-review: Gather context

Before evaluating the upgrade itself, collect:

- [ ] **PR number and link** — the pull request that contains the
  contract changes.
- [ ] **Git diff summary** — a high-level list of files changed
  (ask the developer for `git diff --stat origin/main...HEAD`).
- [ ] **STORAGE_VERSION bump?** — Was `STORAGE_VERSION` incremented?
  If yes, migration is required. If no, the upgrade is
  backwards-compatible at the storage level.
- [ ] **RESULT_SCHEMA_VERSION bump?** — Was the result schema version
  incremented? If yes, the backend bridge (`apexchainx-be`) must be
  updated in lockstep.
- [ ] **EVENT_VERSION bump?** — Was the event version symbol bumped?
  If yes, all backend indexers must be updated.
- [ ] **CHANGELOG entry** — Does the PR include a `CHANGELOG.md` entry
  describing the user-visible change?

---

## Part 1 — What is changing?

Answer these questions with the developer's help:

| Question | Answer | Notes |
|----------|--------|-------|
| What new features or bug fixes are included? | | |
| Does this change modify how `calculate_sla` computes results? | Yes / No | If Yes → high-risk; require audit replay |
| Does this change add or remove storage keys? | Yes / No | If Yes → migration required |
| Does this change modify event payloads? | Yes / No | If Yes → backend re-index may be needed |
| Does this change modify role requirements? | Yes / No | If Yes → document new auth rules |
| Does this change affect pause behaviour? | Yes / No | If Yes → verify all write functions still honour pause |

- [ ] **I understand what this upgrade changes** at a functional level.

---

## Part 2 — Storage & migration safety

If `STORAGE_VERSION` was NOT bumped:

- [ ] **No new persistent storage keys were added.** (Adding a new
  `const KEY: Symbol` and writing to it without bumping
  `STORAGE_VERSION` is a bug.)

If `STORAGE_VERSION` WAS bumped:

- [ ] **A migration path exists** — the `migrate()` function includes
  a new arm for the version transition (e.g., `v1 → v2`).
- [ ] **Migration is idempotent** — calling `migrate()` twice does not
  corrupt state (the PR includes a test for this).
- [ ] **Migration preserves existing history** — the `HISTORY_KEY`
  entries from the old version are not lost or truncated.
- [ ] **Migration preserves existing config** — the `CONFIG_KEY` and
  `CUSTOM_CONFIG_KEY` maps survive the migration intact.
- [ ] **Rollback plan exists** — if the migration fails, what is the
  recovery path? (e.g., restore from pre-migration backup, re-deploy
  old binary.)

---

## Part 3 — Access control & roles

- [ ] **No new admin-gated function was added without explicit review.**
  Admin-gated functions are irreversible (or hard to reverse) and must
  be carefully scrutinised.
- [ ] **No role check was removed or weakened.** If a function
  previously required `require_admin()` and now does not, that is a
  breaking security change.
- [ ] **The `renounce_admin` path is still documented.** After
  renouncing, no admin-gated function will ever succeed — this is
  terminal and must be clearly communicated.
- [ ] **Operator role boundaries are unchanged.** The operator can call
  `calculate_sla` but cannot change config, pause, or modify roles.

---

## Part 4 — Event & API compatibility

- [ ] **No event was removed.** Removing an event breaks all backend
  indexers. Events may be deprecated (with a coexistence period) but
  never silently dropped.
- [ ] **No event payload field was removed or reordered.** Fields may
  be appended to the end but never removed or reordered without a
  `RESULT_SCHEMA_VERSION` bump.
- [ ] **`get_result_schema()` returns the correct vocabulary.** A
  backend calling this function must receive symbols that match the
  events actually emitted.
- [ ] **`get_failure_schema()` includes all error codes.** If a new
  error variant was added to `SLAError`, it must appear in
  `get_failure_schema()`.
- [ ] **`get_contract_metadata()` reflects the new version.** The
  `storage_version` and `result_schema_version` fields must match
  the on-chain reality.

---

## Part 5 — Pause & emergency recovery

- [ ] **The contract can be paused before the upgrade.** Verify
  `pause()` still works on the current deployed binary.
- [ ] **The upgraded contract can still be paused.** Verify `pause()`
  works on the new binary (test this on testnet before mainnet).
- [ ] **Pause blocks all write functions.** `calculate_sla`,
  `set_config`, `migrate`, and governance functions must all reject
  calls while paused.
- [ ] **Read-only functions work while paused.** `get_config_snapshot`,
  `get_history`, `healthcheck`, and similar read-only views must
  continue to function so monitoring dashboards stay live.
- [ ] **`unpause` clears the pause state completely.** After unpausing,
  `is_paused()` returns `false` and all write functions resume.

---

## Part 6 — CI & test evidence

Verify that the PR's CI pipeline is green and includes:

- [ ] **`cargo test`** — all existing tests pass, and new tests cover
  the changed behaviour.
- [ ] **`cargo clippy -- -D warnings`** — no new lint warnings.
- [ ] **`cargo fmt -- --check`** — formatting is consistent.
- [ ] **`cargo check --target wasm32-unknown-unknown --lib`** — the
  Wasm build succeeds (no `std` leakage).
- [ ] **Fuzz tests pass** (if applicable) — run the fuzz target for at
  least the CI duration.
- [ ] **Backend parity tests pass** — coordinate with the `apexchainx-be`
  team to re-run their contract-event snapshot tests.

**Ask the developer for:**

- [ ] A link to the CI run showing all-green.
- [ ] A summary of test coverage for the changed code paths.

---

## Part 7 — Deployment plan

- [ ] **Testnet deployment completed.** The new binary was deployed to
  testnet and smoke-tested:
  - [ ] `initialize` succeeds (or contract is already initialised).
  - [ ] `migrate` succeeds (if `STORAGE_VERSION` was bumped).
  - [ ] `calculate_sla` produces expected results with test data.
  - [ ] `healthcheck` returns `ready: true`.
  - [ ] Backend bridge (`apexchainx-be`) successfully connects and
    reads the event stream.
- [ ] **Mainnet deployment order is documented.**
  1. Deploy new Wasm binary.
  2. Call `migrate` (if needed).
  3. Verify `healthcheck` returns `ready: true`.
  4. Notify backend team to restart their bridge.
  5. Monitor for 1 hour before considering the upgrade complete.
- [ ] **Rollback plan is documented.** If the upgrade causes
  unexpected behaviour:
  - [ ] Re-deploy the previous Wasm binary (hash: `_________`).
  - [ ] Call `migrate` is NOT needed (the old binary reads the old
    storage version).
  - [ ] Notify backend team.
- [ ] **Emergency contacts are listed.**
  - Developer: `_________`
  - Backend lead: `_________`
  - Multisig signers: `_________`

---

## Sign-off

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Upgrade proposer (developer) | | | |
| Contract admin reviewer | | | |
| Backend team representative | | | |
| Security reviewer (if applicable) | | | |

**By signing, each reviewer confirms they have completed their portion
of this checklist and believe the upgrade is safe to deploy.**

---

## References

- [CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md](CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md) —
  additional checks when deploying across multiple contract crates.
- [SECURITY.md](../SECURITY.md) — vulnerability reporting policy.
- [CONTRIBUTING.md § SC-098](../CONTRIBUTING.md#sc-098-security-review-checklist-for-privileged-changes) —
  developer-focused security review for privileged changes.
- [CONTRIBUTING.md § SC-099](../CONTRIBUTING.md#sc-099-event-topic--payload-schema-contributor-safety-checklist) —
  developer-focused event compatibility checklist.
