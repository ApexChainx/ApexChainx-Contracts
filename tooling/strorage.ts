#!/usr/bin/env -S npx tsx
/**
 * SC-510: Release Summary Generator (#280)
 *
 * Generates a maintainership-friendly summary of contract changes from the
 * CHANGELOG.md [Unreleased] section, suitable for release review triage.
 *
 * Usage:
 *   npx tsx tooling/release-summary.ts
 *   npx tsx tooling/release-summary.ts --json     # JSON output for CI
 *   npx tsx tooling/release-summary.ts --check    # Verify changelog format
 *
 * Output sections:
 *   - Public API Changes (Added / Changed / Removed)
 *   - Breaking Changes (explicitly flagged)
 *   - Schema Impact (RESULT_SCHEMA_VERSION, SLAResult changes)
 *   - Storage Impact (new keys, migration requirements)
 *   - Testing Surface (new tests, coverage changes)
 */

import * as fs from "fs";
import * as path from "path";

const CHANGELOG_PATH = path.resolve(__dirname, "..", "CHANGELOG.md");

interface ChangelogSection {
  heading: string;
  entries: string[];
}

interface ReleaseSummary {
  version: string;
  api_changes: {
    added: string[];
    changed: string[];
    removed: string[];
  };
  breaking_changes: string[];
  schema_impact: string[];
  storage_impact: string[];
  test_surface: string[];
}

function parseUnreleasedSection(content: string): ChangelogSection[] {
  const unreleasedMatch = content.match(/## \[Unreleased\]([\s\S]*?)(?=## \[|$)/);
  if (!unreleasedMatch) {
    return [];
  }

  const unreleased = unreleasedMatch[1];
  const sections: ChangelogSection[] = [];
  const sectionRegex = /### (Added|Changed|Fixed|Removed|Security)\n([\s\S]*?)(?=### |$)/g;
  let match;

  while ((match = sectionRegex.exec(unreleased)) !== null) {
    const heading = match[1];
    const entries = match[2]
      .split("\n")
      .filter((line) => line.startsWith("- "))
      .map((line) => line.replace(/^- /, "").trim());
    if (entries.length > 0) {
      sections.push({ heading, entries });
    }
  }

  return sections;
}

function generateSummary(): ReleaseSummary {
  if (!fs.existsSync(CHANGELOG_PATH)) {
    console.error(`CHANGELOG.md not found at ${CHANGELOG_PATH}`);
    process.exit(1);
  }

  const content = fs.readFileSync(CHANGELOG_PATH, "utf-8");
  const sections = parseUnreleasedSection(content);

  const summary: ReleaseSummary = {
    version: "Unreleased",
    api_changes: { added: [], changed: [], removed: [] },
    breaking_changes: [],
    schema_impact: [],
    storage_impact: [],
    test_surface: [],
  };

  for (const section of sections) {
    for (const entry of section.entries) {
      if (entry.includes("(breaking)")) {
        summary.breaking_changes.push(entry);
      }
      if (
        entry.includes("RESULT_SCHEMA_VERSION") ||
        entry.includes("schema_version") ||
        entry.includes("SLAResult")
      ) {
        summary.schema_impact.push(entry);
      }
      if (
        entry.includes("storage") ||
        entry.includes("Storage") ||
        entry.includes("STORAGE_") ||
        entry.includes("migration") ||
        entry.includes("Migration")
      ) {
        summary.storage_impact.push(entry);
      }

      switch (section.heading) {
        case "Added":
          summary.api_changes.added.push(entry);
          break;
        case "Changed":
          summary.api_changes.changed.push(entry);
          break;
        case "Removed":
          summary.api_changes.removed.push(entry);
          break;
      }
    }
  }

  return summary;
}

function formatTextSummary(summary: ReleaseSummary): string {
  const lines: string[] = [];
  lines.push("=".repeat(72));
  lines.push(`  ApexChainx Contracts — Release Summary`);
  lines.push(`  Version: ${summary.version}`);
  lines.push(`  Generated: ${new Date().toISOString().split("T")[0]}`);
  lines.push("=".repeat(72));
  lines.push("");

  if (summary.api_changes.added.length > 0) {
    lines.push("📦 Public API — Added:");
    for (const entry of summary.api_changes.added) {
      lines.push(`   + ${entry}`);
    }
    lines.push("");
  }

  if (summary.api_changes.changed.length > 0) {
    lines.push("🔄 Public API — Changed:");
    for (const entry of summary.api_changes.changed) {
      lines.push(`   ~ ${entry}`);
    }
    lines.push("");
  }

  if (summary.api_changes.removed.length > 0) {
    lines.push("🗑  Public API — Removed:");
    for (const entry of summary.api_changes.removed) {
      lines.push(`   - ${entry}`);
    }
    lines.push("");
  }

  if (summary.breaking_changes.length > 0) {
    lines.push("⚠️  Breaking Changes:");
    for (const entry of summary.breaking_changes) {
      lines.push(`   ! ${entry}`);
    }
    lines.push("");
  }

  if (summary.schema_impact.length > 0) {
    lines.push("🔷 Schema Impact:");
    for (const entry of summary.schema_impact) {
      lines.push(`   • ${entry}`);
    }
    lines.push("");
  }

  if (summary.storage_impact.length > 0) {
    lines.push("💾 Storage Impact:");
    for (const entry of summary.storage_impact) {
      lines.push(`   • ${entry}`);
    }
    lines.push("");
  }

  lines.push("─".repeat(72));
  lines.push("  Backend Integration Checklist:");
  lines.push("  [ ] Re-run backend parity tests against contract snapshots");
  lines.push("  [ ] Update backend SLAResult deserialisation if schema changed");
  lines.push("  [ ] Verify event topic stability tests pass");
  lines.push("  [ ] Confirm get_version_info() handshake unchanged");
  lines.push("─".repeat(72));
  lines.push("");
  lines.push("Full changelog: CHANGELOG.md");
  lines.push("Maintenance policies: docs/CONTRACT_MAINTENANCE_POLICY.md");

  return lines.join("\n");
}

// --- Main ---

const args = process.argv.slice(2);
const summary = generateSummary();

if (args.includes("--json")) {
  console.log(JSON.stringify(summary, null, 2));
} else if (args.includes("--check")) {
  // Verify changelog has required sections
  const hasPublicApi =
    summary.api_changes.added.length > 0 ||
    summary.api_changes.changed.length > 0 ||
    summary.api_changes.removed.length > 0;
  if (!hasPublicApi) {
    console.log("⚠️  No public API changes detected in [Unreleased] section.");
    console.log("   This is expected for documentation-only releases.");
  } else {
    console.log("✅ Changelog format valid.");
    console.log(
      `   ${summary.api_changes.added.length} added, ${summary.api_changes.changed.length} changed, ${summary.api_changes.removed.length} removed`,
    );
  }
  if (summary.breaking_changes.length > 0) {
    console.log(
      `⚠️  ${summary.breaking_changes.length} breaking change(s) detected — review required.`,
    );
    for (const bc of summary.breaking_changes) {
      console.log(`   ! ${bc}`);
    }
  }
} else {
  console.log(formatTextSummary(summary));
}
