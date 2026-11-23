# Event ABI generation 2 (#675, #677)

The global event ABI is `v2` / generation 2, storage version is 5, and result
schema version is 2. The #497 minimum-schema co-bump guard remains active.
Storage migration 4 -> 5 is an explicit admin acknowledgement with no data
rewrite. Older migration steps still run first; repeat migration is a no-op.
The result field layout is unchanged in this coordinated schema release.

`event_schema::EVENT_SCHEMAS` is the registry of event name, topic symbol,
per-name version and last changed ABI generation. Every publish site resolves
topic[1] using `event_version(topic[0])`. Unknown names fail closed. The three
topics remain `(name, per-name version, context)`.

Only `set_int` advances to `v2` in this release because its trailing correlation
ID changes encoding. Other names retain `v1`. `get_contract_info` continues to
advertise the **global** ABI generation for deployment compatibility, not the
version to substitute into each event.

Consumers must use `(name, version)` dispatch. `ts/eventDispatch.ts` supports
explicit concurrent historical/current handlers and rejects unregistered
pairs. Use the generated `EVENT_VERSIONS` table when registering current
handlers; retain a `set_int:v1` decoder for historical replay.

For later breaking changes: bump only the affected registry row's per-name
version, advance its last-changed ABI generation, bump the global ABI symbol
and generation, extend the co-bump policy and migrate storage/result schemas.
Regenerate TS constants and fixtures, update consumers, and review the type
inventory. Never overwrite a historical decoder with different semantics.

## Correlation ID contract

The `u64` encodes `(u64(ledger_sequence) << 32) | fnv1a32(outage_symbol_bytes)`.
FNV-1a-32 starts at 2166136261 and multiplies by 16777619 modulo 2^32 after
XORing each raw ASCII symbol byte. There is no debug-format prefix or suffix.
Big-endian bytes 0..4 contain the supplied ledger sequence, including zero
and `u32::MAX`; bytes 4..8 contain the outage fingerprint.

- Different ledger sequences **cannot** collide, regardless of outage.
- Repeated submissions of the same outage in the same ledger share an ID.
- Different outages in the same ledger **can** hash-collide. This compact
  tracing hint does not promise injectivity over arbitrary outage symbols.
- It is not unique across networks or contracts. Join using network,
  contract, full outage ID and ledger; never use the ID for authorization or
  deduplication. Zero is not a reserved sentinel.

The Rust and TS tests share the vector `hello`, ledger 42 ->
`0x0000002a4f9f2cab`. TS uses `bigint`. The `set_int` payload carries the ID;
SLA topics remain three elements. Correlation travels in the payload; the
obsolete four-topic helper remains removed.

Upgrades from storage v3 chain through v4 actor-attribution compatibility first. Existing v4 actor metadata is preserved by the v5 stamp.
