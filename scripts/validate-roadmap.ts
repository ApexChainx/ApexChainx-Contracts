// SC-050: Validates that the repo's documented roadmap matches actual checked-in crates.
// Fails if docs claim a crate exists that isn't present, keeping the roadmap code-accurate.

import * as fs from "fs";
import * as path from "path";

interface CrateEntry {
  name: string;
  status: "active" | "planned";
  dir: string;
}

// Source of truth: what the roadmap claims
const ROADMAP: CrateEntry[] = [
  { name: "apexchainx_calculator",        status: "active",  dir: "apexchainx_calculator" },
  { name: "payment_escrow",        status: "planned", dir: "payment_escrow" },
  { name: "multi_party_settlement",status: "planned", dir: "multi_party_settlement" },
];

const ROOT = path.resolve(__dirname, "..");

function validate(entries: CrateEntry[]): void {
  let errors = 0;

  for (const entry of entries) {
    const exists = fs.existsSync(path.join(ROOT, entry.dir));

    if (entry.status === "active" && !exists) {
      console.error(`[FAIL] '${entry.name}' is marked active but directory '${entry.dir}' not found`);
      errors++;
    }

    if (entry.status === "planned" && exists) {
      console.warn(`[WARN] '${entry.name}' is marked planned but '${entry.dir}' already exists — update status to active`);
    }

    if (entry.status === "active" && exists) {
      console.log(`[ OK] '${entry.name}' — active and present`);
    }

    if (entry.status === "planned" && !exists) {
      console.log(`[ OK] '${entry.name}' — planned, not yet checked in`);
    }
  }

  if (errors) {
    console.error(`\n${errors} roadmap inconsistency(ies) found.`);
    process.exit(1);
  } else {
    console.log("\nRoadmap is consistent with repo state.");
  }
}

validate(ROADMAP);
