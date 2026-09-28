import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { EventDispatcher, correlationId } from "../eventDispatch";
import { EVENT_VERSION, EVENT_VERSIONS } from "../generated/contractConstants";
import { discoverSources, withoutComments } from "../../scripts/check-parity-coverage";

test("per-name dispatch isolates versions and supports explicit historical decoders", () => {
  const dispatcher = new EventDispatcher<string>();
  dispatcher.register("paused", EVENT_VERSIONS.paused, () => "pause v1");
  dispatcher.register("set_int", EVENT_VERSIONS.set_int, () => "settle v2");
  dispatcher.register("set_int", "v1", () => "historical settle");
  assert.equal(dispatcher.dispatch(["paused", "v1", "admin"], []), "pause v1");
  assert.equal(dispatcher.dispatch(["set_int", "v2", "high"], []), "settle v2");
  assert.equal(dispatcher.dispatch(["set_int", "v1", "high"], []), "historical settle");
  assert.throws(() => dispatcher.dispatch(["paused", EVENT_VERSION, "admin"], []));
  assert.throws(() => dispatcher.dispatch(["unknown", "v1", "admin"], []));
  assert.throws(() => dispatcher.dispatch(["set_int", "v2"], []));
});

test("generated event versions agree with the Rust per-name registry", () => {
  const source = readFileSync("apexchainx_calculator/src/event_schema.rs", "utf8");
  const entries = [...source.matchAll(/\(\s*"([a-z_]+)",\s*(?:crate::)?EVENT_\w+,\s*(\d+),\s*(\d+)\s*,?\s*\)/g)];
  assert.ok(entries.length > 20);
  assert.deepEqual(Object.fromEntries(entries.map(m => [m[1], `v${m[2]}`])), EVENT_VERSIONS);
  const generation = Number(source.match(/EVENT_ABI_GENERATION: u32 = (\d+)/)![1]);
  assert.equal(EVENT_VERSION, `v${generation}`);
  for (const entry of entries) assert.ok(Number(entry[2]) <= Number(entry[3]) && Number(entry[3]) <= generation);
  const lib = readFileSync("apexchainx_calculator/src/lib.rs", "utf8");
  const storagePolicy = source.match(/EVENT_ABI_TO_STORAGE_VERSION: &\[u32\] = &\[([^\]]+)\]/)![1].split(",").map(Number);
  const resultPolicy = source.match(/EVENT_ABI_TO_SCHEMA_VERSION: &\[u32\] = &\[([^\]]+)\]/)![1].split(",").map(Number);
  assert.ok(Number(lib.match(/const STORAGE_VERSION: u32 = (\d+)/)![1]) >= storagePolicy[generation - 1]);
  assert.ok(Number(lib.match(/const RESULT_SCHEMA_VERSION: u32 = (\d+)/)![1]) >= resultPolicy[generation - 1]);
});

test("correlation IDs preserve the ledger and reproduce the Rust golden vector", () => {
  assert.equal(correlationId("hello", 42), 0x0000002a4f9f2cabn);
  for (const ledger of [0, 1, 42, 0xffffffff]) {
    const id = correlationId("OUT001", ledger);
    assert.equal(id >> 32n, BigInt(ledger));
    assert.equal(id, correlationId("OUT001", ledger));
    assert.equal(id & 0xffffffffn, correlationId("OUT001", 0));
  }
  assert.notEqual(correlationId("OUT_A", 42), correlationId("OUT_B", 42));
  assert.throws(() => correlationId("OUT001", -1));
  assert.throws(() => correlationId("OUT001", 2 ** 32));
});

test("event emitters resolve topic versions by their own event name", () => {
  let emitters = 0;
  for (const [file, contents] of Object.entries(discoverSources("apexchainx_calculator/src"))) {
    const source = withoutComments(contents);
    let audited = 0;
    for (const match of source.matchAll(/\.publish\(\s*\(\s*((?:(?:crate|event_schema)::)?EVENT_\w+|event_name|expiry_event)(?:\.clone\(\))?,\s*([^,]+),/g)) {
      emitters++;
      audited++;
      assert.equal(match[2].replace(/\s+/g, ""), `${match[2].includes("crate::") ? "crate::" : ""}event_schema::event_version(${match[1]})`, file);
    }
    assert.equal(audited, [...source.matchAll(/\.publish\(/g)].length, `${file}: every publish site must be audited`);
  }
  assert.ok(emitters > 0, "emit-site audit unexpectedly missed the event surface");
});
