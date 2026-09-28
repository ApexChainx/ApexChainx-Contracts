import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { main, reconcileMethods, contractTypes, discoverSources } from "../../scripts/check-parity-coverage";

test("contract manifest and type surface have explicit parity coverage", () => main());
const lib = '#[contractimpl] impl Contract { pub fn read() {} pub fn write() {} fn manifest() { methods.push_back(method("read", false)); methods.push_back(method("write", true)); } }';
const inventory = 'const PARITY_METHOD_COVERAGE: &[(&str, &str, &str)] = &[("read", "fixture", "reads"), ("write", "skip", "No mutation mirror")]; fn generate_ts_parity_fixtures() { client.read(); }';
test("new methods, stale coverage and missing fixtures cannot pass", () => {
  assert.deepEqual(reconcileMethods(lib, inventory, { reads: [1] }), { methods: 2, covered: 1, skipped: 1 });
  assert.throws(() => reconcileMethods(lib.replace('pub fn read()', 'pub fn new_method() {} pub fn read()'), inventory, { reads: [1] }));
  assert.throws(() => reconcileMethods(lib.replace('pub fn read()', 'pub fn new_method() {} pub fn read()').replace('methods.push_back', 'methods.push_back(method("new_method", false)); methods.push_back'), inventory, { reads: [1] }));
  assert.throws(() => reconcileMethods(lib, inventory.replace('("write", "skip", "No mutation mirror")', ''), { reads: [1] }));
  assert.throws(() => reconcileMethods(lib, inventory, {}));
  assert.throws(() => reconcileMethods(lib, inventory.replace('client.read();', ''), { reads: [1] }));
  assert.throws(() => reconcileMethods(lib, inventory.replace('client.read();', '/* client.read(); */'), { reads: [1] }));
  assert.throws(() => reconcileMethods(lib, inventory.replace('client.read();', '') + 'fn unrelated_test() { client.read(); }', { reads: [1] }));
  assert.throws(() => reconcileMethods(lib, inventory.replace('No mutation mirror', ' '), { reads: [1] }));
  assert.throws(() => reconcileMethods(lib, inventory, { reads: [] }));
  assert.throws(() => reconcileMethods(lib, inventory, { reads: [1] }, lib + '#[contractimpl] impl Contract { pub fn added_in_module() {} }'));
});
test("new contracttype fields and type changes alter the inventory", () => {
  const before = contractTypes({ "lib.rs": '#[contracttype] #[derive(Clone)] pub struct Wire { pub count: u32 }' });
  for (const fields of ['pub count: u64', 'pub count: u32, pub added: bool']) {
    assert.notDeepEqual(contractTypes({ "lib.rs": `#[contracttype] pub struct Wire { ${fields} }` }), before);
  }
  assert.throws(() => contractTypes({ "new.rs": '#[contracttype] pub struct New(pub u32);' }), /unrecognized contracttype/);
});

test("#678 nested Rust modules are discovered instead of silently omitted", () => {
  mkdirSync("validation-results/acceptance-cases", { recursive: true });
  const root = mkdtempSync("validation-results/acceptance-cases/surface-");
  mkdirSync(`${root}/nested/deeper`, { recursive: true });
  writeFileSync(`${root}/nested/deeper/wire.rs`, '#[contracttype] pub struct Added { pub value: u32 }');
  const types = contractTypes(discoverSources(root));
  assert.deepEqual(Object.keys(types), ["nested/deeper/wire.rs:Added"]);
});
