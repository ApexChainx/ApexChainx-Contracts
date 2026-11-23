# Fuzzing Triage & Cadence (#630)

How crashes found by the nightly fuzz workflow (`fuzz.yml`) turn into fixes,
and how the corpus stays healthy.

## Cadence

| Time | Who | What |
|------|-----|------|
| Every night (02:00 UTC) | `fuzz.yml` schedule | Runs the 5 targets (`compute_result`, `validate_config`, `config_mutation_sequences`, `governance_sequences`, `history_state_machine`), dedups crash artifacts by `sha256`, prunes the corpus, files one issue per new unique crash, and persists corpus + report markers back to `main` |
| Next working day, before 12:00 | Assignee (see [Maintenance](#maintenance)) | Triage every `Fuzz crash:` issue raised overnight |
| Each working day | The assignee | Reproduce, root-cause, and either land a fix or file a downstream issue on the owning contract |

> While CI is temporarily disabled (`fuzz.yml` triggers commented out per #684),
> the scheduled run does not fire. The workflow below describes the behaviour
> that resumes the moment the schedule is re-enabled.

## Who triages

- **`apexchainx_calculator` targets** — owned by the calculator maintainers
  (`docs/MODULE_OWNERSHIP.md`).
- **Cross-contract targets** (`config_mutation_sequences`,
  `governance_sequences`, `history_state_machine`) — the maintainer of whatever
  surface they exercise (config writes, governance, history).

## Triage checklist (per `Fuzz crash:` issue)

1. **Reproduce** from the artifact, or by running the repro command in the
   issue body (`cargo +nightly fuzz run <target> -- -runs=1 <file>`).
2. **Root-cause.** When the crash is a spec disagreement, resolve it through
   `docs/FUZZING_GUARANTEES.md` § "Which statement is authoritative" first —
   never silently favor one side.
3. **Fix** and add the crashing input as a corpus seed so the bug class stays
   under test.
4. **Delete the report marker** for that crash SHA
   (`apexchainx_calculator/fuzz/.fuzz-reports/<target>/<sha12>`) in the fix
   commit. The nightly job only files a crash while its marker is absent, so a
   fixed bug stops being re-filed. Close the issue with a reference to the fix
   commit.

## Corpus hygiene

- **Dedup.** Each nightly run drops artifacts whose `sha256` already has a
  report marker (duplicate across nights) or appears twice in the same run.
- **Prune.** `corpus/<target>` is capped at 2000 seeds per target; entries
  that duplicate an existing hash are removed and the oldest seeds beyond the
  cap are dropped deterministically.
- **Review.** The corpus diff is committed to `main` nightly; reviewers see
  exactly what changed (`git diff` on `apexchainx_calculator/fuzz/corpus/`).

## Corpus add/remove rule

Always add a seed for a crash you fix, and remove a seed for a bug that no
longer reproduces. The corpus is coverage archaeology — trim it the way you
would trim tests.