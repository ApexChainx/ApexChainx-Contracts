/** Reconcile the public manifest, real entrypoints, and explicit Rust fixture inventory. */
import assert from "node:assert/strict";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const sourceRoot = "apexchainx_calculator/src";
export function withoutComments(source: string): string {
  return source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");
}
function bodyAt(source: string, start: number): string {
  const open = source.indexOf("{", start);
  let depth = 1, end = open + 1;
  for (; end < source.length && depth; end++) {
    if (source[end] === "{") depth++;
    if (source[end] === "}") depth--;
  }
  assert.equal(depth, 0, "unbalanced source declaration");
  return source.slice(open + 1, end - 1);
}

export function reconcileMethods(lib: string, generator: string, fixture: Record<string, unknown>, implementations = lib) {
  const source = withoutComments(lib);
  const implementationSource = withoutComments(implementations);
  const manifest = [...source.matchAll(/methods\.push_back\(\s*method\(\s*"([^"]+)"/g)].map(m => m[1]);
  const entrypoints = [...implementationSource.matchAll(/#\[contractimpl\]\s*impl\s+\w+\s*\{/g)]
    .flatMap(m => [...bodyAt(implementationSource, m.index!).matchAll(/pub\s+fn\s+(\w+)\s*\(/g)].map(m => m[1]));
  assert.ok(manifest.length && entrypoints.length, "public surface must not be empty");
  assert.equal(new Set(manifest).size, manifest.length, "duplicate manifest method");
  assert.deepEqual([...new Set(entrypoints)].sort(), [...manifest].sort(), "manifest must match contract entrypoints");
  const generatorSource = withoutComments(generator);
  const inventory = generatorSource.match(/const PARITY_METHOD_COVERAGE[^=]*=\s*&\[([\s\S]*?)\];/);
  assert.ok(inventory, "missing explicit parity coverage inventory");
  const entries = [...inventory[1].matchAll(/\(\s*"([^"]+)",\s*"(fixture|skip)",\s*"([^"]+)"\s*,?\s*\)/g)];
  assert.deepEqual(entries.map(m => m[1]).sort(), [...manifest].sort(), "every manifest method needs exactly one fixture or explicit skip");
  const generationEntry = generatorSource.match(/fn\s+generate_ts_parity_fixtures\s*\([^)]*\)\s*\{/);
  assert.ok(generationEntry, "missing executable fixture generator");
  const generationBody = bodyAt(generatorSource, generationEntry.index!);
  for (const [, method, mode, detail] of entries) {
    assert.ok(detail.trim(), `${method}: empty coverage justification`);
    if (mode === "fixture") {
      assert.ok(Object.hasOwn(fixture, detail), `${method}: missing fixture target ${detail}`);
      assert.match(generationBody, new RegExp(`client\\.${method}\\s*\\(`), `${method}: fixture generator must invoke the contract method`);
      const value = fixture[detail];
      assert.ok(value && (Array.isArray(value) ? value.length : Object.keys(value as object).length), `${method}: empty fixture target`);
    }
  }
  const skipped = entries.filter(m => m[2] === "skip").length;
  return { methods: manifest.length, covered: entries.length - skipped, skipped };
}

/** Pin the complete wire declaration, including field names, types and enum variants. */
export function contractTypes(sources: Record<string, string>): Record<string, string> {
  const result: Record<string, string> = {};
  for (const file of Object.keys(sources).sort()) {
    const source = withoutComments(sources[file]);
    let discovered = 0;
    for (const match of source.matchAll(/#\[contracttype(?:\([^\]]*\))?\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?(?:struct|enum)\s+(\w+)\s*\{/g)) {
      discovered++;
      result[`${file}:${match[1]}`] = bodyAt(source, match.index!).replace(/\s+/g, "");
    }
    assert.equal(discovered, [...source.matchAll(/#\[contracttype(?:\([^\]]*\))?\]/g)].length,
      `${file}: unrecognized contracttype declaration; extend the inventory parser explicitly`);
  }
  assert.ok(Object.keys(result).length, "no contract types discovered");
  return result;
}

export function discoverSources(root: string, relative = ""): Record<string, string> {
  const result: Record<string, string> = {};
  for (const entry of readdirSync(resolve(root, relative), { withFileTypes: true })) {
    const name = relative ? `${relative}/${entry.name}` : entry.name;
    if (entry.isDirectory()) Object.assign(result, discoverSources(root, name));
    else if (entry.isFile() && name.endsWith(".rs")) result[name] = readFileSync(resolve(root, name), "utf8");
  }
  return result;
}

export function main() {
  const sources = discoverSources(sourceRoot);
  const snapshot = "ts/parity/contract-types.json";
  const types = contractTypes(sources);
  if (process.argv.includes("--write-types")) {
    writeFileSync(snapshot, JSON.stringify(types, null, 2) + "\n");
    console.log("Updated type inventory. Review changed fields and their fixtures/skips before committing.");
  }
  assert.deepEqual(types, JSON.parse(readFileSync(snapshot, "utf8")), "contracttype surface changed: review fixtures/skips and explicitly refresh the type inventory");
  const counts = reconcileMethods(sources["lib.rs"], sources["ts_parity_fixtures.rs"], JSON.parse(readFileSync("ts/fixtures/contract-read-semantics.json", "utf8")), Object.values(sources).join("\n"));
  console.log(JSON.stringify({ ...counts, contractTypes: Object.keys(types).length }));
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main();
