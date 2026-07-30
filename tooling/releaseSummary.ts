/**
 * releaseSummary.ts
 *
 * Generates a structured ship-review note from CHANGELOG.md for use during
 * maintainer release triage. The output covers:
 *
 *  - New public API surface (Added entries)
 *  - Breaking changes (Changed entries flagged "(breaking)")
 *  - All other behavioural changes (Changed, Fixed, Removed, Security)
 *  - Storage impact: entries that mention storage keys, schema versions, or
 *    migration functions, which need extra scrutiny on-chain
 *  - Test surface: entries cross-referenced with the test file list
 *  - Open questions placeholder so reviewers can annotate the note
 *
 * Usage (run from repo root):
 *   npx --yes tsx tooling/releaseSummary.ts [--version <tag>] [--out <file>]
 *
 * Options:
 *   --version <tag>   Summarise a specific released version (e.g. "0.3.0").
 *                     Defaults to the [Unreleased] block.
 *   --out <file>      Write the summary to a file instead of stdout.
 *
 * See docs/RELEASE_SUMMARY_FORMAT.md for the full field descriptions.
 */

import { readFileSync, writeFileSync, existsSync } from "fs";
import { join, resolve } from "path";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** One logical entry parsed from a changelog section bullet. */
export interface ChangeEntry {
  /** Raw bullet text, minus the leading `- `. */
  text: string;
  /** Issue / SC ticket references extracted from the text. */
  refs: string[];
  /** True when the text contains "(breaking)". */
  breaking: boolean;
  /** True when the entry is likely to touch on-chain storage. */
  storageImpact: boolean;
}

/** All entries for a single category (`Added`, `Changed`, …). */
export interface ChangeSection {
  category: "Added" | "Changed" | "Fixed" | "Removed" | "Security" | string;
  entries: ChangeEntry[];
}

/** Parsed representation of one changelog version block. */
export interface VersionBlock {
  /** Version tag, e.g. "0.3.0" or "Unreleased". */
  version: string;
  /** Optional release title extracted from the heading (after the dash). */
  title: string | null;
  sections: ChangeSection[];
}

/** Final ship-review note ready for rendering. */
export interface ReleaseSummary {
  version: string;
  title: string | null;
  generatedAt: string;
  /** Functions / types added to the public API. */
  newApiSurface: ChangeEntry[];
  /** All entries marked "(breaking)". */
  breakingChanges: ChangeEntry[];
  /** Non-breaking Changed / Fixed / Removed / Security entries. */
  otherChanges: ChangeEntry[];
  /** Entries with probable storage impact (migration, schema, DataKey). */
  storageImpactEntries: ChangeEntry[];
  /** Ticket / issue references extracted across all entries. */
  allRefs: string[];
  /** Rendered Markdown note. */
  markdown: string;
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

const STORAGE_SIGNALS = [
  /storage[_ ]version/i,
  /schema[_ ]version/i,
  /migrate\b/i,
  /DataKey/,
  /prune_history/,
  /retention[_ ]limit/i,
  /on[-\s]?chain/i,
];

const REF_PATTERN = /(?:#|SC-)[\w-]+\d+/g;

function parseEntry(raw: string): ChangeEntry {
  const text = raw.replace(/^[-*]\s*/, "").trim();
  const refs = [...(text.match(REF_PATTERN) ?? [])];
  const breaking = /\(breaking\)/i.test(text);
  const storageImpact = STORAGE_SIGNALS.some((p) => p.test(text));
  return { text, refs, breaking, storageImpact };
}

/**
 * Splits raw changelog markdown into per-version blocks and parses each
 * into sections and entries.
 */
export function parseChangelog(content: string): VersionBlock[] {
  const blocks: VersionBlock[] = [];
  // Each version begins with "## [" at the start of a line.
  const versionHeadingRe = /^## \[([^\]]+)\](?:\s*[—–-]\s*(.+))?$/m;
  // Split on "## [" lines (keep the delimiter in subsequent segments).
  const rawBlocks = content.split(/(?=^## \[)/m).filter((b) => b.trim());

  for (const raw of rawBlocks) {
    const headingMatch = raw.match(versionHeadingRe);
    if (!headingMatch) continue;

    const version = headingMatch[1].trim();
    const title = headingMatch[2]?.trim() ?? null;

    // Everything after the heading line.
    const body = raw.slice(raw.indexOf("\n") + 1);

    // Split into category sub-sections ("### Added", "### Changed", …).
    const sections: ChangeSection[] = [];
    const categoryBlocks = body.split(/^### /m).filter((s) => s.trim());

    for (const catRaw of categoryBlocks) {
      const lines = catRaw.split("\n");
      const category = lines[0].trim();
      const entries: ChangeEntry[] = [];

      for (const line of lines.slice(1)) {
        const trimmed = line.trim();
        if (trimmed.startsWith("-") || trimmed.startsWith("*")) {
          entries.push(parseEntry(trimmed));
        }
      }

      if (entries.length > 0) {
        sections.push({ category, entries });
      }
    }

    blocks.push({ version, title, sections });
  }

  return blocks;
}

/** Finds the block for the requested version (case-insensitive). */
export function findBlock(
  blocks: VersionBlock[],
  version: string
): VersionBlock | undefined {
  return blocks.find(
    (b) => b.version.toLowerCase() === version.toLowerCase()
  );
}

// ---------------------------------------------------------------------------
// Summary builder
// ---------------------------------------------------------------------------

/**
 * Builds the structured ship-review note for a single version block.
 */
export function buildSummary(block: VersionBlock): ReleaseSummary {
  const newApiSurface: ChangeEntry[] = [];
  const breakingChanges: ChangeEntry[] = [];
  const otherChanges: ChangeEntry[] = [];
  const storageImpactEntries: ChangeEntry[] = [];
  const refSet = new Set<string>();

  for (const section of block.sections) {
    for (const entry of section.entries) {
      entry.refs.forEach((r) => refSet.add(r));

      if (entry.storageImpact) storageImpactEntries.push(entry);

      if (section.category === "Added") {
        newApiSurface.push(entry);
      } else if (entry.breaking) {
        breakingChanges.push(entry);
      } else {
        otherChanges.push(entry);
      }

      // Breaking entries also bubble up from Added (rare but possible).
      if (entry.breaking && section.category === "Added") {
        breakingChanges.push(entry);
      }
    }
  }

  const allRefs = [...refSet].sort();
  const generatedAt = new Date().toISOString();

  const markdown = renderMarkdown({
    version: block.version,
    title: block.title,
    generatedAt,
    newApiSurface,
    breakingChanges,
    otherChanges,
    storageImpactEntries,
    allRefs,
    markdown: "", // filled below
  });

  return {
    version: block.version,
    title: block.title,
    generatedAt,
    newApiSurface,
    breakingChanges,
    otherChanges,
    storageImpactEntries,
    allRefs,
    markdown,
  };
}

// ---------------------------------------------------------------------------
// Markdown renderer
// ---------------------------------------------------------------------------

function bulletList(entries: ChangeEntry[], emptyMsg: string): string {
  if (entries.length === 0) return `_${emptyMsg}_\n`;
  return entries.map((e) => `- ${e.text}`).join("\n") + "\n";
}

/**
 * Renders the structured summary as a Markdown ship-review note following
 * the template described in docs/RELEASE_SUMMARY_FORMAT.md.
 */
export function renderMarkdown(s: Omit<ReleaseSummary, "markdown">): string {
  const versionLabel = s.version === "Unreleased"
    ? "Unreleased (next)"
    : `v${s.version}`;
  const titleLine = s.title ? ` — ${s.title}` : "";
  const refs = s.allRefs.length > 0
    ? s.allRefs.join(", ")
    : "_none detected_";

  const storageNote =
    s.storageImpactEntries.length > 0
      ? s.storageImpactEntries.map((e) => `- ${e.text}`).join("\n")
      : "_No entries flagged for storage review._";

  return `# Ship-Review Note: ${versionLabel}${titleLine}

> Auto-generated by \`tooling/releaseSummary.ts\` on ${s.generatedAt}.
> Fill in the **Open Questions** and **Reviewer Sign-off** sections before merging.

---

## 1. New Public API Surface

${bulletList(s.newApiSurface, "No new public API in this block.")}
**Review checklist:**
- [ ] Each new function has a corresponding test in \`tests/\` or \`apexchainx_calculator/src/tests.rs\`
- [ ] Doc comments are present and accurate
- [ ] No unintended public surface exposed

---

## 2. Breaking Changes

${s.breakingChanges.length === 0
    ? "_No breaking changes in this block._\n"
    : "⚠️ **Breaking changes require a major or minor version bump.**\n\n" +
      bulletList(s.breakingChanges, "")}
**Review checklist:**
- [ ] Backend adapter compatibility confirmed (\`docs/CONTRACT_API_COMPATIBILITY.md\`)
- [ ] Version negotiation response updated (\`get_version_info\`)
- [ ] Migration path documented if storage layout changed

---

## 3. Other Changes (non-breaking)

${bulletList(s.otherChanges, "No other changes in this block.")}

---

## 4. Storage Impact

${storageNote}

**Review checklist:**
- [ ] No new unbounded storage growth paths introduced
- [ ] Storage version bumped if on-chain layout changed (\`get_storage_version\`)
- [ ] \`migrate\` function updated if required
- [ ] Retention limits respected (\`get_retention_limit\` / \`set_retention_limit\`)

---

## 5. Linked Tickets & Issues

${refs}

---

## 6. Open Questions

_Replace this section with any concerns, risks, or items needing clarification before ship._

- [ ] ?

---

## 7. Reviewer Sign-off

| Reviewer | Date | Notes |
|----------|------|-------|
| | | |

---

_See [docs/RELEASE_SUMMARY_FORMAT.md](../docs/RELEASE_SUMMARY_FORMAT.md) for field descriptions._
`;
}

// ---------------------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------------------

if (require.main === module) {
  const args = process.argv.slice(2);

  let versionArg = "Unreleased";
  let outFile: string | null = null;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--version" && args[i + 1]) {
      versionArg = args[++i];
    } else if (args[i] === "--out" && args[i + 1]) {
      outFile = args[++i];
    } else if (args[i] === "--help" || args[i] === "-h") {
      console.log(
        "Usage: npx ts-node tooling/releaseSummary.ts [--version <tag>] [--out <file>]\n" +
        "  --version  Version block to summarise (default: Unreleased)\n" +
        "  --out      Write output to file instead of stdout"
      );
      process.exit(0);
    }
  }

  const changelogPath = resolve(join(process.cwd(), "CHANGELOG.md"));
  if (!existsSync(changelogPath)) {
    console.error(`CHANGELOG.md not found at ${changelogPath}`);
    process.exit(1);
  }

  const content = readFileSync(changelogPath, "utf8");
  const blocks = parseChangelog(content);

  if (blocks.length === 0) {
    console.error("No version blocks found in CHANGELOG.md");
    process.exit(1);
  }

  const block = findBlock(blocks, versionArg);
  if (!block) {
    const available = blocks.map((b) => b.version).join(", ");
    console.error(
      `Version "${versionArg}" not found. Available: ${available}`
    );
    process.exit(1);
  }

  const summary = buildSummary(block);

  if (outFile) {
    writeFileSync(outFile, summary.markdown, "utf8");
    console.log(`Release summary written to ${outFile}`);
  } else {
    process.stdout.write(summary.markdown);
  }
}
