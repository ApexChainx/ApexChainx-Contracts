/**
 * SC-041: SDK compatibility and dependency update policy enforcer.
 * Reads Cargo.toml and validates pinned versions against policy rules.
 */

import * as fs from "fs";
import * as path from "path";

interface Policy {
  minSorobanSdk: string;
  minRustEdition: string;
  requirePinnedVersions: boolean;
}

const DEFAULT_POLICY: Policy = {
  minSorobanSdk: "21.0.0",
  minRustEdition: "2021",
  requirePinnedVersions: true,
};

function parseCargoToml(filePath: string): Record<string, string> {
  const content = fs.readFileSync(filePath, "utf8");
  const deps: Record<string, string> = {};
  const depSection = /\[dependencies\]([\s\S]*?)(\[|$)/.exec(content)?.[1] ?? "";
  for (const line of depSection.split("\n")) {
    const m = /^(\S+)\s*=\s*"([^"]+)"/.exec(line.trim());
    if (m) deps[m[1]] = m[2];
  }
  const editionMatch = /edition\s*=\s*"(\d+)"/.exec(content);
  if (editionMatch) deps["__edition__"] = editionMatch[1];
  return deps;
}

function semverAtLeast(actual: string, min: string): boolean {
  const parse = (v: string) => v.replace(/[^0-9.]/g, "").split(".").map(Number);
  const [a, b] = [parse(actual), parse(min)];
  for (let i = 0; i < 3; i++) {
    if ((a[i] ?? 0) > (b[i] ?? 0)) return true;
    if ((a[i] ?? 0) < (b[i] ?? 0)) return false;
  }
  return true;
}

function checkPolicy(cargoPath: string, policy: Policy = DEFAULT_POLICY): boolean {
  const deps = parseCargoToml(cargoPath);
  let ok = true;

  const sdkVer = deps["soroban-sdk"] ?? "0";
  if (!semverAtLeast(sdkVer, policy.minSorobanSdk)) {
    console.error(`❌ soroban-sdk ${sdkVer} < required ${policy.minSorobanSdk}`);
    ok = false;
  } else {
    console.log(`✅ soroban-sdk ${sdkVer} meets minimum ${policy.minSorobanSdk}`);
  }

  if (policy.requirePinnedVersions) {
    const unpinned = Object.entries(deps).filter(([k, v]) => k !== "__edition__" && /[\^~*]/.test(v));
    if (unpinned.length) {
      console.error(`❌ Unpinned deps: ${unpinned.map(([k]) => k).join(", ")}`);
      ok = false;
    } else {
      console.log("✅ All dependencies are pinned");
    }
  }

  return ok;
}

const cargoPath = path.resolve(process.argv[2] ?? "apexchainx_calculator/Cargo.toml");
const passed = checkPolicy(cargoPath);
process.exit(passed ? 0 : 1);
