# Configuration Freeze Audit Metadata

## Current behavior

`freeze_config` stores whether configuration is frozen. The `cfg_frz` and
`cfg_unfrz` events identify the caller in their topics, but carry no timestamp
or reason, and the contract exposes no freeze metadata query. The existing
events are useful for detecting transitions, but do not provide a durable
record of why or when a configuration freeze was requested.

## Proposed audit record

An implementation of issue #683 should persist metadata for the most recent
freeze transition:

| Field | Meaning |
|---|---|
| `actor` | Admin address that initiated the freeze |
| `timestamp` | Ledger timestamp when the freeze took effect |
| `reason` | Optional caller-supplied explanation |

The record should be queryable while the configuration is frozen. A successful
unfreeze should emit its lifecycle event and clear the current freeze record;
historical transitions remain available in the Soroban event stream. Repeated
freeze or unfreeze calls that do not change state should not create audit
transitions.

## Event compatibility

Keep the existing `cfg_frz` and `cfg_unfrz` event topic and payload shapes
stable for existing consumers. Publish a separately named, versioned audit
event containing the transition timestamp and optional reason, with the actor
identified in its topics. Document its ordering relative to the existing
lifecycle event and update the event schema catalog and compatibility tests.

## Verification

Governance tests for the implementation should verify that the actor,
timestamp, and reason are persisted on freeze; the audit event is emitted with
those values; unfreeze clears the current metadata and emits its event; and
no-op transitions do not emit duplicate lifecycle or audit events.

This document records the issue's proposed design only. It does not implement
the behavior or satisfy issue #683's acceptance criteria.