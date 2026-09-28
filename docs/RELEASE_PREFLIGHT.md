# Release preflight (#674)

Run `npm ci` and `just release-preflight` from the repository root before
tagging. `just hash`, `hash-save`, `hash-verify`, the tooling release checklist,
and Release Validation all require this gate. A matching WASM hash alone is
not release approval.

Copy `validation-results/preflight/release-ticket.md` into the release ticket
and attach the `release-validation-report` workflow artifact. The artifact is
uploaded even after failure and contains per-check logs, the exact candidate
commit, nightly run metadata, artifact URLs, and applicable waivers.

Release checklist:

- [ ] Latest scheduled `fuzz.yml` run on main is completed and green, at most
  48 hours old, with retained `fuzz-evidence-*` artifacts. Its commit must be
  an ancestor of the candidate. A newer pending/red run cannot be hidden by
  selecting an older green run. This is a nightly health gate, not proof that
  every candidate-only change has already run through a nightly campaign.
- [ ] Rust fixtures and generated constants were regenerated and are committed.
  Tracked changes and untracked generated files both fail freshness.
- [ ] No open `Fuzz crash:` triage issues remain, or the exact-candidate nightly
  waiver explains the debt. Open issues and their URLs are saved in
  `fuzz-triage-debt.json`; a later green nightly does not erase an unresolved crash.
- [ ] Method/type inventory and TS typecheck/parity pass. Record covered and
  explicitly skipped method counts; skips are not behavioral parity coverage.
- [ ] Offchain event/read budgets, governance/metadata checks and persistence
  fixture freshness pass.
- [ ] Evidence artifact and run URL are attached to the release ticket.
- [ ] Any fuzz waiver has explicit review, bounded scope, and an expiry.

Missing runs, unrelated history, unavailable APIs and absent proof fail closed.
The nightly schedule and release workflow must be enabled in GitHub; disabling
them prevents a normal green release preflight.

## Waivers

`release/waivers.json` is an array, empty by default. Only the nightly health
gate accepts a waiver. Freshness, typecheck, parity and budgets cannot be
waived by this mechanism. Example record (replace every placeholder):

```json
{
  "gate": "nightly-fuzz",
  "candidate": "<full candidate commit SHA>",
  "runId": 123456,
  "reason": "Impact, mitigations and why this release can proceed",
  "approvedBy": "reviewing-maintainer",
  "ticket": "https://github.com/owner/repo/issues/123",
  "expiresAt": "2026-10-01T00:00:00Z"
}
```

The reviewer and ticket are auditable records, not automated identity
verification: maintainers must review the waiver commit through the normal
repository review process. Protect changes to this file accordingly.

To avoid a candidate-SHA/self-reference problem, commit the reviewed waiver
on a separate approval branch **after** freezing the candidate. Run Release
Validation on the candidate/tag with `waiver_ref` set to that immutable
approval commit. The workflow saves the approval commit and fetched registry
with its evidence. Locally, set `RELEASE_WAIVERS_FILE` to that reviewed JSON
file. Do not edit the candidate or use a wildcard candidate/run.

An expired, incomplete or mismatched record fails. No record can waive the
complete absence of a nightly run or a run on unrelated history.
