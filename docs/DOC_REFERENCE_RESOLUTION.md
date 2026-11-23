# Doc Reference Resolution and the Stale-Documentation Check

> **Issue:** [#679](https://github.com/ApexChainx/ApexChainx-Contracts/issues/679)
> **Status:** Findings recorded; doc-lint pass not yet implemented
> **Applies to:** `calculator.md`, root-level and `docs/` markdown, CI workflows

This document records the stale references found in the repository's markdown and
the mechanical check that would catch them. It is the reference for the doc-lint
work described in
[#679](https://github.com/ApexChainx/ApexChainx-Contracts/issues/679).

**Related documents:**
- [`PUBLIC_API_MANIFEST_VERIFICATION.md`](PUBLIC_API_MANIFEST_VERIFICATION.md) - the same drift class on the contract API surface
- [`link-check`](../tools/link-check.ts) - the existing local link checker

---

## 1. What exists today

Two checks are already wired into `client-checks` in `.github/workflows/ci.yml`:

| Check | Scope | Limitation |
| --- | --- | --- |
| `tools/link-check.ts` | Markdown link targets resolve to existing files | Checks paths only. Never inspects code identifiers. |
| `scripts/check-orphan-modules.sh` | Every `.rs` file in the crate is declared as a module | Rust only. Says nothing about documentation. |

Neither resolves a **symbol** named in prose. An inline-code identifier such as
`event::publish_sla_calc` in a markdown file is never checked against the crate,
so it can name a function that does not exist for as long as the file is
committed.

## 2. Stale references confirmed in `calculator.md`

`calculator.md` is a prose sketch of `calculate_sla`. Every reference below was
checked against `apexchainx_calculator/src/*.rs`.

| Reference in `calculator.md` | Resolves? | Actual state |
| --- | --- | --- |
| `Error::DivisionByZero` | **No** | No such variant anywhere in the crate. |
| `Error::InconsistentPaymentStatus` | **No** | No such variant anywhere in the crate. |
| `event::publish_sla_calc` | **No** | Publisher methods live on a builder in `event_publisher.rs`: `EventPublisher::sla_calc`, `EventPublisher::settlement_intent`. There is no free function `publish_sla_calc`. |
| `history::append_and_prune` | **No** | `history.rs` exposes `append_entry`, `prune_oldest`, `prune_history`. No `append_and_prune`. |
| `calculation::compute_result(&env, outage_id, &severity, mttr_minutes, &config)` | **Wrong signature** | Real signature takes no `&Env`, no `&severity`, and adds two trailing arguments: `compute_result(outage_id, mttr_minutes, cfg, config_version_hash, recorded_at)`. |
| `metrics::increment_stats(&env, &result.status, result.amount, &severity)` | **Wrong signature** | Real signature is `increment_stats(env: &Env, met: bool, reward: i128, penalty: i128)`. |
| `calculate_sla(env, outage_id, severity, mttr_minutes)` | **Wrong signature** | The contract method is `calculate_sla(env, caller, outage_id, severity, mttr_minutes)`; the `caller` auth parameter is missing. |

Three referenced symbols do not exist at all, and three more exist under different
names or signatures. The file reads as authoritative but would mislead any
contributor who treats it as a description of the real code path.

This is the concrete instance of the staleness class named in
[#679](https://github.com/ApexChainx/ApexChainx-Contracts/issues/679), which also
cites the `get_retention_metrics` phantom recorded in
[`PUBLIC_API_MANIFEST_VERIFICATION.md`](PUBLIC_API_MANIFEST_VERIFICATION.md).

## 3. Blocker: CI does not currently run

Any doc-lint added to `.github/workflows/` will not execute on this repository as
it stands. The `on:` triggers are commented out at the top of `ci.yml`:

```yaml
# CI temporarily disabled - triggers commented out below.
# on:
#   push:
#     branches: [main]
#   pull_request:
#     branches: [main]
```

No workflow in `.github/workflows/` fires on `push` or `pull_request`, so
`client-checks` and every other job are dormant. The issue's requirement that the
lint be "wired into pre-push + CI" therefore has a prerequisite: the triggers must
be restored. That is a separate change from the doc-lint itself and is called out
here so it is not discovered late.

## 4. Check required to close #679

Acceptance criteria from the issue, and what each still needs:

| Criterion | State |
| --- | --- |
| Doc references resolve to code | **Not met.** Section 2 lists seven unresolved references in one file. |
| Stale `calculator.md`-style content fails | **Not met.** No check inspects code identifiers. |
| Lint wired into pre-push + CI | **Not met.** No such job exists, and CI triggers are disabled (section 3). |

The proposed pass, per the issue, extracts backticked identifiers from `docs/` and
the root markdown and asserts each resolves to a symbol, function, type, or event
in the crate. Two design notes for whoever implements it:

- **Scope the extraction.** Backticked text in these documents includes CLI flags,
  file names, env-var names, and prose. A naive "every backticked token must be a
  symbol" rule will produce a wall of false positives on day one and will be
  disabled. A curated prefix set, or an explicit allowlist, keeps the signal.
- **Reuse the existing source-scan pattern.** As in
  [`PUBLIC_API_MANIFEST_VERIFICATION.md`](PUBLIC_API_MANIFEST_VERIFICATION.md), a
  `std::fs` walk over `apexchainx_calculator/src` resolves identifiers without new
  dependencies, which keeps the `cargo machete` and `cargo udeps` gates green.

Until the check exists, the references in section 2 remain live inaccuracies in
`calculator.md`.
