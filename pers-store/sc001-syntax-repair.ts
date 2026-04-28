// Closes #121 — [SC-001] Repair merged test-suite delimiter and syntax errors

import * as fs from "fs";
import * as path from "path";

interface RepairResult {
  file: string;
  removed: number;
  ok: boolean;
}

const STRAY_DELIMITER = /^\s*\}\s*$/;
const BLOCK_OPEN = /\{/g;
const BLOCK_CLOSE = /\}/g;

function countDelimiters(src: string): { open: number; close: number } {
  return {
    open: (src.match(BLOCK_OPEN) ?? []).length,
    close: (src.match(BLOCK_CLOSE) ?? []).length,
  };
}

function removeTrailingStrayDelimiters(src: string): { src: string; removed: number } {
  const lines = src.split("\n");
  let removed = 0;
  const { open, close } = countDelimiters(src);
  let excess = close - open;

  for (let i = lines.length - 1; i >= 0 && excess > 0; i--) {
    if (STRAY_DELIMITER.test(lines[i])) {
      lines.splice(i, 1);
      removed++;
      excess--;
    }
  }
  return { src: lines.join("\n"), removed };
}

function repairFile(filePath: string): RepairResult {
  const raw = fs.readFileSync(filePath, "utf8");
  const { src, removed } = removeTrailingStrayDelimiters(raw);
  if (removed > 0) fs.writeFileSync(filePath, src, "utf8");
  return { file: path.basename(filePath), removed, ok: removed >= 0 };
}

function main(): void {
  const target = process.argv[2];
  if (!target) {
    console.error("Usage: ts-node sc001-syntax-repair.ts <file>");
    process.exit(1);
  }
  const result = repairFile(target);
  console.log(`[SC-001] ${result.file}: removed ${result.removed} stray delimiter(s). ok=${result.ok}`);
}

main();
