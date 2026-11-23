# SLAError Domain Convention

> **Issue:** [#659](https://github.com/ApexChainx/ApexChainx-Contracts/issues/659)
> **Applies to:** `SLAError` in `lib.rs`, `error_responses.rs`

## Problem

`SLAError` accumulates variants from every feature area (initialization,
auth, governance, validation, calculation, migration) in a single flat enum.
Without a grouping convention, new variants risk conceptual collisions, and
backends' match logic grows unreadably over time.

## Convention: numeric ranges by domain

Each error domain owns a reserved numeric range. New variants **must** be
added within their domain range. The `#[repr(u32)]` discriminant is the
stable wire value consumed by backends.

| Range | Domain | Variants |
|---|---|---|
| 1–9 | **Lifecycle / Init** | `AlreadyInitialized`, `NotInitialized`, `VersionMismatch`, `ContractPaused`, `ConfigFrozen` |
| 10–19 | **Auth / Governance** | `Unauthorized`, `NoPendingTransfer`, `ProposalExpired`, `AdminRenounced` |
| 20–29 | **Config Validation** | `ConfigNotFound`, `InvalidThreshold`, `InvalidPenalty`, `InvalidReward`, `InvalidSeverity`, `SeverityNotInSet`, `InvalidInput`, `RetentionLimitOutOfRange` |
| 30–39 | **Calculation** | `DuplicateOutageInput`, `InvalidPenaltyAmount`, `InvalidRewardAmount`, `OutageRecalcLimit` |
| 40–49 | **Reserved** | (future use) |

> **Note:** The existing discriminant values in the codebase pre-date this
> convention and do not conform to the ranges above. They are **frozen** —
> existing values must never change because they are the stable wire
> representation. All **new** variants added after this document must follow
> the range convention.

## Current variant catalogue

```
// Lifecycle / Init
AlreadyInitialized  = 1
NotInitialized      = 2
VersionMismatch     = 5
ContractPaused      = 6
ConfigFrozen        = 16

// Auth / Governance
Unauthorized        = 3
NoPendingTransfer   = 7
ProposalExpired     = 20
AdminRenounced      = 21

// Config Validation
ConfigNotFound      = 4
InvalidThreshold    = 8
InvalidPenalty      = 9
InvalidReward       = 10
InvalidSeverity     = 11
RetentionLimitOutOfRange = 12
InvalidInput        = 17
SeverityNotInSet    = 18

// Calculation
DuplicateOutageInput   = 13
InvalidPenaltyAmount   = 14
InvalidRewardAmount    = 15
OutageRecalcLimit      = 19
```

## Adding a new variant (checklist)

1. Identify the domain from the table above.
2. Pick the next unused discriminant within that domain's range.
3. Add the variant to `SLAError` in `lib.rs` with a doc comment citing the
   issue number that introduced it.
4. Add a corresponding `is_*` helper in `error_responses.rs`.
5. Update this document's catalogue table.
6. Add or update a backend guidance note in `docs/FAILURE_TAXONOMY.md`.
7. The `test_error_domain_catalogue` test in `error_responses.rs` must be
   updated to include the new variant — CI will fail otherwise.

## Stability guarantee

Once a discriminant value is assigned and merged to `main`, it is
**permanently frozen**. Renaming the variant is allowed (it is a Rust symbol
only); changing the `u32` value is a breaking wire-format change and requires
a major version bump.