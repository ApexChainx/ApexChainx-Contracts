# Correlation ID contract (#677)

The `set_int:v2` payload carries a trailing `u64` correlation ID. Its high
32 bits are the supplied ledger sequence; its low 32 bits are FNV-1a-32 over
raw outage-symbol bytes. Repeated outage/ledger pairs produce the same ID.
Different ledgers cannot collide. Different outages within one ledger can
hash-collide, and IDs are not unique across networks or contracts.

Use it as a tracing hint. Join on network, contract, full outage ID and ledger;
do not use the compact ID for authorization or deduplication. Zero is valid.

SLA contract events keep three topics: name, per-name version, context.
The obsolete `correlation_event_topics` helper remains removed, as on main.
Historical `set_int:v1` uses the old encoding
and needs its own decoder.

See [the encoding and migration guide](docs/EVENT_VERSION_MIGRATION.md) for the
exact algorithm, Rust/TS golden vector, per-event dispatch, and coordinated
ABI/storage/result-schema versions. Tests assert determinism, structural ledger
recovery, distinct-ledger separation, and representative outage fingerprints.
