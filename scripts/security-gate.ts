// SC-049: Pre-merge security gate for governance and stateful contract changes.
// Scans staged Rust files for common patterns that require manual security sign-off.

import { execSync } from "child_process";
import * as fs from "fs";

interface Rule {
  id: string;
  pattern: RegExp;
  message: string;
}

const RULES: Rule[] = [
  { id: "AUTH-01", pattern: /fn\s+\w+\s*\(.*env.*\)(?![^{]*require_auth)/, message: "Function may be missing require_auth before state write" },
  { id: "STOR-01", pattern: /storage\(\)\.set\(/, message: "Direct storage write — confirm key is namespaced" },
  { id: "GOV-01",  pattern: /set_admin|set_operator/,                       message: "Privileged role change — confirm two-step flow is used" },
  { id: "CFG-01",  pattern: /set_config/,                                   message: "Config mutation — confirm range validation present" },
  { id: "EVT-01",  pattern: /fn\s+\w+\s*\(.*env.*\)(?![^}]*events\(\))/, message: "State-mutating function may be missing event emission" },
];

function scanFile(path: string): number {
  const src = fs.readFileSync(path, "utf8");
  let hits = 0;
  for (const rule of RULES) {
    if (rule.pattern.test(src)) {
      console.log(`  [${rule.id}] ${path}: ${rule.message}`);
      hits++;
    }
  }
  return hits;
}

function getStagedRustFiles(): string[] {
  try {
    return execSync("git diff --cached --name-only --diff-filter=ACM")
      .toString().trim().split("\n").filter((f) => f.endsWith(".rs"));
  } catch { return []; }
}

const files = getStagedRustFiles();
if (files.length === 0) { console.log("No staged Rust files — nothing to check."); process.exit(0); }

console.log(`Scanning ${files.length} staged file(s) for security patterns...\n`);
const total = files.reduce((n, f) => n + scanFile(f), 0);
console.log(`\n${total} pattern hit(s) found. Review before merging.`);
if (total > 0) process.exit(1);
