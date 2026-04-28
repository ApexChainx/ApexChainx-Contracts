/**
 * changelogLint.ts
 * Issue #159 — validates that interface-affecting contract changes are
 * surfaced correctly in CHANGELOG.md before a PR merges.
 */

import { readFileSync } from "fs";

/** Patterns that signal an interface-affecting change in a diff. */
const INTERFACE_PATTERNS = [
  /pub fn \w+/,        // new or renamed public function
  /#\[contracttype\]/, // new contract type
  /DataKey::\w+/,      // new storage key
  /Error::\w+/,        // new error variant
];

/** Expected changelog tag for interface changes. */
const INTERFACE_TAG = "[interface]";

export interface LintResult {
  ok: boolean;
  missing: string[];
}

/**
 * Checks that every interface-affecting line in `diff` has a corresponding
 * `[interface]` entry in the changelog.
 */
export function lintChangelog(diff: string, changelogPath: string): LintResult {
  const changelog = readFileSync(changelogPath, "utf8");
  const missing: string[] = [];

  for (const pattern of INTERFACE_PATTERNS) {
    const match = diff.match(pattern);
    if (!match) continue;

    const tag = `${INTERFACE_TAG} ${match[0].trim()}`;
    if (!changelog.includes(tag)) {
      missing.push(tag);
    }
  }

  return { ok: missing.length === 0, missing };
}

/** CLI entry point: node changelogLint.js <diff-file> <changelog> */
if (require.main === module) {
  const [, , diffFile, changelogFile] = process.argv;
  const diff = readFileSync(diffFile, "utf8");
  const result = lintChangelog(diff, changelogFile);
  if (!result.ok) {
    console.error("Missing changelog entries:\n" + result.missing.join("\n"));
    process.exit(1);
  }
  console.log("Changelog OK");
}
