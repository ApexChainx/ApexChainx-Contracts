// SC-050: Code-accurate future multi-contract roadmap for the repo
// Generates a roadmap doc that distinguishes current state from planned future crates.

import * as fs from "fs";
import * as path from "path";

interface RoadmapEntry {
  crate: string;
  status: "active" | "planned" | "not-started";
  description: string;
  blockedBy?: string;
}

const ROADMAP: RoadmapEntry[] = [
  {
    crate: "apexchainx_calculator",
    status: "active",
    description: "Deterministic SLA calculation, config, stats, history, and governance.",
  },
  {
    crate: "payment_escrow",
    status: "planned",
    description: "Holds and releases funds tied to SLA outcomes.",
    blockedBy: "Backend integration design not finalized",
  },
  {
    crate: "multi_party_settlement",
    status: "not-started",
    description: "Coordinates settlement across multiple counterparties.",
    blockedBy: "Depends on payment_escrow",
  },
];

function renderRoadmap(entries: RoadmapEntry[]): string {
  const lines: string[] = [
    "# Multi-Contract Roadmap",
    "",
    "This document reflects the **current repo state** and planned future crates.",
    "Only `apexchainx_calculator` is checked in. Everything else is future work.",
    "",
    "| Crate | Status | Description | Blocked By |",
    "|-------|--------|-------------|------------|",
  ];
  for (const e of entries) {
    lines.push(`| \`${e.crate}\` | ${e.status} | ${e.description} | ${e.blockedBy ?? "—"} |`);
  }
  lines.push("", "_Last generated: " + new Date().toISOString().slice(0, 10) + "_", "");
  return lines.join("\n");
}

const outPath = path.resolve(__dirname, "../docs/ROADMAP.md");
fs.writeFileSync(outPath, renderRoadmap(ROADMAP));
console.log(`Roadmap written to ${outPath}`);
