# Correlation ID — resolution (#565, #566)

## The problem

`correlation_event_topics` (src/event_correlation.rs) accepted a correlation id
and returned the standard 3-topic tuple without it. The doc claimed the id is
"carried in the event payload instead", but `publish_settlement_intent_event`
had no id either — the id had no home. A helper that accepts a value and
discards it is a trap: callers wire the ID in, observe no effect, and assume
correlation is working when it is not.

## Resolution

The correlation ID's home is the **`set_int` (settlement intent) payload**:

- `publish_settlement_intent_event` (and its mirror in src/calculation.rs)
  append `correlation_id` as a trailing 10th field, after the canonical
  `recorded_at` field (#566).
- `correlation_event_topics` and the `CORRELATION_TOPIC` constant were
  **deleted** (#565): topics follow the fixed 3-topic layout
  `(name, version, context)` from src/event_schema.rs, so encoding the id in a
  topic would break that schema; a no-op helper adds nothing.
- `generate_correlation_id` now mixes the outage symbol bytes (FNV-1a over
  `Symbol::to_string()`) with the ledger sequence, so distinct outages in the
  same ledger never collide while same-outage replays still reproduce the id
  (#564).

The home is documented in exactly one place: src/event_schema.rs § `set_int`.

## Schema impact

Additive change (trailing field). Per src/event_schema.rs § "Schema
Versioning", additive trailing fields are NOT considered breaking and do not
require a version bump; the `EVENT_ABI_GENERATION` co-bump invariant (#497)
applies only to breaking event changes and remains satisfied and enforced.

## Test changes

- src/event_correlation.rs: added `test_distinct_outages_in_same_ledger_never_collide`
  and `test_same_outage_in_same_ledger_still_collides`; removed the two tests
  that asserted the no-op `correlation_event_topics` behavior.
- src/coordination_harness.rs: Scenario 4 and the full multi-contract workflow
  now assert determinism and cross-outage distinctness instead of the no-op
  topic helper.
- set_int payload tests (src/event_state_tests.rs, src/payload_versioning_tests.rs,
  src/tests.rs) decode the 10-field tuple and assert the carried id equals
  `generate_correlation_id`'s output.