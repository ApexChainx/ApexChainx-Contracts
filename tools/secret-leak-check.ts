#!/usr/bin/env ts-node
/**
 * Issue #122 — Static check that CI workflows never echo secrets.
 *
 * GitHub Actions masks known secret values in logs, but masking is best-effort:
 * it breaks on transformed values (base64, substrings, JSON-encoded) and does
 * nothing for a full environment dump. The cheapest defence is to never write
 * secret material to the log in the first place. This lints the workflow YAML
 * for the patterns that do so and fails CI on a hit.
 *
 * Rules:
 *   echo-secrets  `echo`/`printf` of a `${{ secrets.* }}` expression.
 *   env-dump      a bare `env` / `printenv` shell command, which prints every
 *                 variable — including any secret mapped in via `env:`.
 *   echo-secret-env  `echo`/`printf` of a shell variable that the same step
 *                 mapped from `${{ secrets.* }}` (the indirect leak).
 *
 * Usage:
 *   npx --yes tsx tools/secret-leak-check.ts
 *   npx --yes ts-node --transpile-only tools/secret-leak-check.ts
 *   npx --yes tsx tools/secret-leak-check.ts --dir .github/workflows
 *
 * Suppress a reviewed false positive with an inline comment on the same line:
 *   run: echo "not a secret"   # secret-leak-check:allow explain why
 *
 * Exit codes: 0 = clean, 1 = at least one finding.
 */

const fs = require('fs');
const path = require('path');

const ALLOW_MARKER = 'secret-leak-check:allow';
const DEFAULT_DIR = path.join('.github', 'workflows');

interface Rule {
  id: string;
  reason: string;
  test: (line: string) => boolean;
}

/** `${{ secrets.NAME }}` — tolerant of spacing. */
const SECRETS_EXPR = /\$\{\{\s*secrets\.[A-Za-z_][A-Za-z0-9_]*\s*\}\}/;
const ECHOES = /\b(echo|printf)\b/;

const RULES: Rule[] = [
  {
    id: 'echo-secrets',
    reason: 'echo/printf of a ${{ secrets.* }} expression writes the secret into the build log',
    test: (line) => ECHOES.test(line) && SECRETS_EXPR.test(line),
  },
  {
    id: 'env-dump',
    // Only a bare `env` / `printenv` COMMAND. Never the YAML `env:` mapping key,
    // which is legitimate and common — hence the trailing-colon exclusion.
    reason: 'a bare `env`/`printenv` dumps every variable, including secrets mapped via env:',
    test: (line) => {
      const s = stripRunPrefix(line).trim();
      if (s.endsWith(':')) return false; // `env:` YAML key
      return /^(env|printenv)(\s*(\||>|>>|&&|;).*)?$/.test(s);
    },
  },
];

/** `run: env` → `env`, so a one-line run step is checked like a script line. */
function stripRunPrefix(line: string): string {
  const m = line.match(/^\s*-?\s*run:\s*(.*)$/);
  return m ? m[1] : line;
}

/** Shell variables the step mapped from a secret, e.g. `TOKEN: ${{ secrets.T }}`. */
function collectSecretEnvVars(lines: string[]): Set<string> {
  const names = new Set<string>();
  for (const line of lines) {
    const m = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(.*)$/);
    if (m && SECRETS_EXPR.test(m[2])) names.add(m[1]);
  }
  return names;
}

/** echo of a secret-derived variable: `echo $TOKEN` / `echo "${TOKEN}"`. */
function echoesSecretVar(line: string, secretVars: Set<string>): string | null {
  if (!ECHOES.test(line)) return null;
  for (const name of secretVars) {
    const ref = new RegExp('\\$\\{?' + name + '\\b');
    if (ref.test(line)) return name;
  }
  return null;
}

function listWorkflowFiles(dir: string): string[] {
  if (!fs.existsSync(dir)) return [];
  return fs
    .readdirSync(dir)
    .filter((f: string) => f.endsWith('.yml') || f.endsWith('.yaml'))
    .map((f: string) => path.join(dir, f))
    .sort();
}

function main(): void {
  const dirArg = process.argv.indexOf('--dir');
  const dir = dirArg !== -1 ? process.argv[dirArg + 1] : DEFAULT_DIR;

  const files = listWorkflowFiles(dir);
  if (files.length === 0) {
    console.error(`secret-leak-check: no workflow files found in ${dir}`);
    process.exit(1);
  }

  const findings: string[] = [];

  for (const file of files) {
    const lines: string[] = fs.readFileSync(file, 'utf8').split('\n');
    const secretVars = collectSecretEnvVars(lines);

    lines.forEach((line: string, i: number) => {
      if (line.includes(ALLOW_MARKER)) return;

      for (const rule of RULES) {
        if (rule.test(line)) {
          findings.push(`${file}:${i + 1}  [${rule.id}] ${rule.reason}\n    ${line.trim()}`);
          return;
        }
      }

      const leaked = echoesSecretVar(line, secretVars);
      if (leaked) {
        findings.push(
          `${file}:${i + 1}  [echo-secret-env] echoes $${leaked}, which this workflow maps from ${'${{ secrets.* }}'}\n    ${line.trim()}`,
        );
      }
    });
  }

  console.log(`secret-leak-check: scanned ${files.length} workflow file(s) in ${dir}`);

  if (findings.length > 0) {
    console.error(`\n✗ ${findings.length} potential secret leak(s):\n`);
    findings.forEach((f) => console.error(`  ${f}\n`));
    console.error(
      `Do not print secrets to the build log. Pass them directly to the command that needs\n` +
        `them, or use a file/masked input. If a hit is a reviewed false positive, append\n` +
        `\`# ${ALLOW_MARKER} <reason>\` to that line.`,
    );
    process.exit(1);
  }

  console.log('✓ no secret-leaking patterns found');
  process.exit(0);
}

main();
