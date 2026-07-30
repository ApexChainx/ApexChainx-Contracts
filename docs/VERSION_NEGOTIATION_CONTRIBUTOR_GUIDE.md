# Version Negotiation Protocol — Contributor Guide

This guide explains how to safely review and evolve the multi-contract version
negotiation protocol without breaking interoperability between contracts.

It complements [SC-502 in `docs/CONTRACT_MAINTENANCE_POLICY.md`](CONTRACT_MAINTENANCE_POLICY.md#sc-502-version-negotiation-protocol--contributor-note),
which covers the single-contract `get_version_info()` response shape. This
document covers the **negotiation protocol itself** — the logic that decides
whether a set of contracts may be deployed together.

---

## 1. Where the Protocol Lives

| Concern | Location |
|---------|----------|
| Protocol implementation | [`apexchainx_calculator/src/version_negotiation.rs`](../apexchainx_calculator/src/version_negotiation.rs) |
| Handshake payload | `VersionNegotiationInfo` |
| Negotiation entry point | `negotiate_contract_versions()` |
| Outcome type | `NegotiationOutcome` (`Compatible` / `Negotiated` / `Incompatible`) |
| Mismatch reporting | `VersionMismatchDetail`, `VersionNegotiationResult` |
| Version constants | `PROTOCOL_VERSION`, `MIN_COMPATIBLE_PROTOCOL` |
| Contract identity symbols | `CONTRACT_SLA_CALC`, `CONTRACT_PAY_ESCROW`, `CONTRACT_SETTLEMENT` |
| Expected discovery surface | `version_discovery_interfaces()` → `ver_info`, `mig_state`, `is_paused` |
| Protocol tests | `mod tests` at the bottom of the same file |

Any contract joining the ecosystem is expected to expose the same
`get_version_info()` shape and the discovery interfaces listed above. Because
peers are deployed independently and upgraded at different times, **the protocol
must be assumed to run against older and newer peers simultaneously.**

---

## 2. Compatibility Constraints

These constraints are what make the protocol safe to run across mixed-version
deployments. Treat them as invariants, not preferences.

### 2.1 The handshake payload is append-only

`VersionNegotiationInfo`, `VersionMismatchDetail`, and `VersionNegotiationResult`
are `#[contracttype]` structs crossing contract boundaries.

- **Never** remove, rename, reorder, or retype an existing field.
- New fields are appended at the end and must have a meaningful default for
  peers that do not yet populate them.
- A new field must never be *required* for a correct negotiation decision until
  every deployed peer emits it.

### 2.2 `NegotiationOutcome` variants are ordinal

The enum is compared by variant across contracts. Adding a variant in the middle
shifts the discriminants of everything after it and silently changes the meaning
of previously stored or in-flight values.

- New variants are appended at the end only.
- Existing variants are never removed or reordered.
- Consumers must treat an unknown outcome as **not** `Compatible`.

### 2.3 Negotiation must fail closed

`negotiate_contract_versions()` starts optimistic and downgrades. Any new rule
must only ever move the outcome toward `Incompatible` — never upgrade an already
failed negotiation back to `Compatible` or `Negotiated`. Once a mismatch is
recorded, the outcome stays failed for that run.

### 2.4 Version constants have directional rules

| Change | Allowed? | Requirement |
|--------|----------|-------------|
| Bump `PROTOCOL_VERSION` | ✅ | Additive protocol change only; leave `MIN_COMPATIBLE_PROTOCOL` alone so older peers still negotiate |
| Raise `MIN_COMPATIBLE_PROTOCOL` | ⚠️ Breaking | Intentionally drops older peers; requires a coordinated multi-contract release |
| Lower `MIN_COMPATIBLE_PROTOCOL` | ❌ | Re-admits peers that were already declared incompatible |
| Add a new contract identity symbol | ✅ | Must be unique and ≤ 9 chars (`symbol_short!`) |
| Change an existing contract identity symbol | ❌ | Breaks log correlation and mismatch attribution |

`MIN_COMPATIBLE_PROTOCOL` is the only knob that removes peers from the network.
Raising it is a deployment-blocking change by design and must be announced.

### 2.5 Summary symbols are a stable vocabulary

`compat`, `negoti`, and `incompt` are consumed by backends and dashboards. They
are part of the wire contract — do not reword them for readability. New outcomes
need new symbols, and all symbols must stay within the 9-character
`symbol_short!` limit.

### 2.6 Orthogonal state stays orthogonal

`is_paused` and `needs_migration` are reported but deliberately do **not**
influence the outcome (see `test_paused_contract_still_negotiates` and
`test_needs_migration_still_negotiates`). Version compatibility answers "can
these contracts talk to each other", not "is it a good moment to deploy".
Operational gating belongs in the deployment checklist, not in negotiation.

---

## 3. Classifying Your Change

| Change | Class | Action |
|--------|-------|--------|
| Appending a field to `VersionNegotiationInfo` | Additive | Bump `PROTOCOL_VERSION`; no `MIN_COMPATIBLE_PROTOCOL` change |
| Appending a `NegotiationOutcome` variant | Additive | Bump `PROTOCOL_VERSION`; document consumer fallback |
| Adding a discovery interface symbol | Additive | Only once every peer exposes it |
| Adding a stricter mismatch rule | **Breaking** | Previously valid deployments start failing; coordinated release |
| Raising `MIN_COMPATIBLE_PROTOCOL` | **Breaking** | Coordinated release across all contracts |
| Removing/reordering/retyping any field or variant | **Breaking** | Requires a major protocol version and a migration plan |
| Doc comments, tests, internal renames | None | Normal review |

Anything in the **Breaking** rows requires sign-off from a maintainer and
coordination with the `apexchainx-be` team before merge.

---

## 4. Review Requirements

A PR touching
[`apexchainx_calculator/src/version_negotiation.rs`](../apexchainx_calculator/src/version_negotiation.rs)
must satisfy every item below. Reviewers should refuse the PR if any is missing.

### Author checklist

- [ ] The PR description states the change class (Additive / Breaking / None)
      using the table in section 3.
- [ ] No existing field, variant, symbol, or constant was removed, reordered,
      renamed, or retyped.
- [ ] `PROTOCOL_VERSION` was bumped if the handshake shape or semantics changed.
- [ ] `MIN_COMPATIBLE_PROTOCOL` was left unchanged, or the PR explains which
      peers are being dropped and why.
- [ ] Negotiation still fails closed — no code path upgrades a recorded
      `Incompatible` back to a passing outcome.
- [ ] `is_paused` / `needs_migration` still do not affect the outcome.
- [ ] New tests cover the mixed-version case: this contract negotiating against
      a peer one version *older* and one version *newer*.
- [ ] The existing tests in `mod tests` still pass unmodified. Editing an
      existing assertion is itself a compatibility signal and must be justified.
- [ ] Downstream docs updated where the handshake flow is described
      (`docs/CODEX_CONTEXT.md`, `docs/CONTRACT_API_COMPATIBILITY.md`,
      `docs/CROSS_CONTRACT_DEPLOYMENT_CHECKLIST.md`).

### Reviewer checklist

- [ ] Re-derive the compatibility matrix by hand for the changed rule: for each
      pair of (our version, peer version) in {older, equal, newer}, confirm the
      expected outcome matches the tests.
- [ ] Confirm every `symbol_short!` literal is ≤ 9 characters.
- [ ] Confirm no new panic path — `negotiate_contract_versions()` must return a
      result rather than trap, so a coordinator can report the mismatch.
- [ ] Confirm mismatch attribution is correct: `contract_name` in a
      `VersionMismatchDetail` must name the contract that is out of range, and
      `required_min` the bound it violated.
- [ ] For breaking changes, confirm backend coordination is recorded in the PR.

### Verification

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test version_negotiation
```

---

## 5. Rollout Rules for Breaking Changes

A breaking protocol change is never a single PR:

1. **Introduce** the additive form (new field/variant) and ship it everywhere,
   keeping the old path authoritative.
2. **Coexist** for at least one release cycle so every deployed contract reports
   the new shape.
3. **Switch** the authoritative path and raise `MIN_COMPATIBLE_PROTOCOL` only
   after step 2 is confirmed on-network.

Skipping the coexistence window means peers mid-upgrade will negotiate
`Incompatible` and block deployments across the whole ecosystem.
