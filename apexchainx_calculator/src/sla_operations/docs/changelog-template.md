# SLA Operations Workspace Changelog Template

Use this template in pull requests that add or materially change modules under
`apexchainx_calculator/src/sla_operations/`.

## Module

- Name:
- Path:
- Owning issue:

## Change Type

- [ ] New helper module
- [ ] New validation module
- [ ] New fixture module
- [ ] New documentation
- [ ] Additive change to an existing module

## Summary

Describe the observable capability added by this change in one or two
sentences.

## Public API

List new or changed public items and their intended callers.

- Item:
- Purpose:

## Inputs and Outputs

Document accepted value ranges, units, and any optional or fallible results.

## Invariants

Record deterministic behavior that callers may rely on, including:

- Boundary behavior:
- Empty or zero-input behavior:
- Overflow or saturation behavior:
- Ordering or stability guarantees:

## Compatibility

- [ ] No changes to contract entrypoints
- [ ] No changes to storage keys or layout
- [ ] No changes to event schemas or topics
- [ ] No changes to core calculation semantics
- [ ] `no_std` compatibility preserved

## Integration Notes

Describe any necessary module registration or wiring. Keep changes outside the
workspace narrow and explain why each is required.

## Follow-Up Work

List known limitations or deliberately deferred extensions that are not part of
this change.
