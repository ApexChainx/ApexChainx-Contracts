# Error and Result Conventions

This note describes recommended error and result patterns for modules under
`apexchainx_calculator/src/sla_operations/`.

## Principles

- Keep helpers pure and deterministic.
- Report invalid inputs explicitly instead of silently clamping unless clamping
  is the documented purpose of the helper.
- Avoid panics for caller-controlled inputs.
- Keep error types small, copyable, and free of allocation.
- Preserve `no_std` compatibility.

## Choosing a Result Shape

Use an `Option` when the only useful outcome is presence or absence.

```rust
pub const fn normalize_duration(start: u64, end: u64) -> Option<u64> {
    end.checked_sub(start)
}
```

Use a dedicated error enum when callers need to distinguish failure modes.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundsError {
    Inverted,
    Empty,
    OutOfRange,
}
```

Use `Result<T, E>` for validation and fallible construction. Use `Option<T>` for
lookups, optional calculations, and cases where every failure has the same
meaning.

## Error Type Guidelines

- Derive `Clone`, `Copy`, `Debug`, `Eq`, and `PartialEq` when possible.
- Use descriptive variant names that describe the violated constraint.
- Avoid embedding formatted strings or allocated messages.
- Keep variants stable because callers may match on them.
- Do not implement `std::error::Error` in `no_std` workspace helpers.

## Saturation and Overflow

Use saturating or checked arithmetic deliberately:

- Use `checked_*` when overflow should be reported to the caller.
- Use `saturating_*` when a bounded maximum is part of the contract.
- Document the chosen behavior in the function or type documentation.
- Avoid wrapping arithmetic unless the wrap is explicitly intended.

## Boundary Behavior

Document how helpers handle:

- Zero totals or empty slices.
- Inverted or equal bounds.
- Values exactly equal to a threshold.
- Maximum integer inputs.

Prefer inclusive or exclusive semantics that are stated explicitly rather than
implied by the implementation.

## Naming

- `validate_*` functions return `Result<(), Error>`.
- `is_*` functions return `bool` and should not hide invalid inputs that callers
  must distinguish.
- `try_*` functions return `Result` or `Option` when construction can fail.
- `normalize_*` functions should document whether they reject or clamp invalid
  ranges.

## Documentation

For every fallible public helper, state:

- What inputs are rejected.
- Which error variant is returned for each rejection.
- Whether any arithmetic saturates or is checked.
- Whether the behavior depends on ordering or uniqueness.
