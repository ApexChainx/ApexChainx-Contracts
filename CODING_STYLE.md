# Coding Style for Soroban Symbol Short Names

Issue #112

This document defines the naming convention for Soroban `Symbol` short names used throughout this repository.

## Scope

Apply these rules whenever you define a short `Symbol` in Rust code, especially for:

- contract storage keys
- event names and topics
- status or state identifiers
- other compact symbolic values

## Conventions

1. Keep every short name at 9 characters or fewer.
2. Use lowercase letters only.
3. Use `_` as the only separator when needed.
4. Follow the `<domain>_<sub>` pattern whenever the name has more than one meaningful part.
5. Prefer compact, descriptive names over long or overly abstract ones.

## Recommended Pattern

Use a short domain prefix followed by a concise sub-name:

- `admin`
- `cfg_upd`
- `sla_calc`
- `pruned_a`

## Non-Compliant Examples

These should be avoided:

- `SLA_CALC` (uppercase)
- `cfg-update` (hyphen is not allowed)
- `settlementintent` (too long)
- `outage_status_code` (too long and overly verbose)

## Review Checklist

Before merging a new short-name symbol, confirm that it:

- [ ] is 9 characters or fewer
- [ ] uses lowercase letters
- [ ] uses `_` only when needed
- [ ] follows the `<domain>_<sub>` pattern where appropriate

## Rationale

Soroban `Symbol` values are compact and must remain short for compatibility and readability. Consistent naming reduces confusion and makes storage keys, event topics, and contract identifiers easier to scan and maintain.
