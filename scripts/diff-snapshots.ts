// SC-048: Classify snapshot diffs as expected or suspicious for PR review guidance.
// Reads two snapshot files and reports which changed fields are semantic vs volatile.

import * as fs from "fs";

const VOLATILE = new Set(["timestamp", "generated_at", "elapsed_ms", "run_id"]);

type JsonVal = string | number | boolean | null | JsonVal[] | { [k: string]: JsonVal };

function flatten(obj: JsonVal, prefix = ""): Record<string, JsonVal> {
  if (obj === null || typeof obj !== "object") return { [prefix]: obj };
  if (Array.isArray(obj)) {
    return obj.reduce<Record<string, JsonVal>>((acc, v, i) => {
      Object.assign(acc, flatten(v as JsonVal, `${prefix}[${i}]`));
      return acc;
    }, {});
  }
  return Object.entries(obj).reduce<Record<string, JsonVal>>((acc, [k, v]) => {
    Object.assign(acc, flatten(v as JsonVal, prefix ? `${prefix}.${k}` : k));
    return acc;
  }, {});
}

function classify(before: string, after: string) {
  const a = flatten(JSON.parse(fs.readFileSync(before, "utf8")));
  const b = flatten(JSON.parse(fs.readFileSync(after, "utf8")));
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  let semantic = 0, volatile = 0;
  for (const k of keys) {
    if (a[k] === b[k]) continue;
    const leaf = k.split(".").pop() ?? k;
    if (VOLATILE.has(leaf)) { volatile++; console.log(`  [volatile]  ${k}`); }
    else { semantic++; console.log(`  [SEMANTIC]  ${k}  ${JSON.stringify(a[k])} → ${JSON.stringify(b[k])}`); }
  }
  console.log(`\n${semantic} semantic change(s), ${volatile} volatile change(s).`);
  if (semantic === 0 && volatile > 0) console.log("Verdict: noise-only diff — safe to ignore.");
  else if (semantic > 0) console.log("Verdict: real change detected — review required.");
}

const [, , before, after] = process.argv;
if (!before || !after) { console.error("Usage: ts-node bench-snapshot-diff.ts <before.json> <after.json>"); process.exit(1); }
classify(before, after);
