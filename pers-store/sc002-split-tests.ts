// Closes #122 — [SC-002] Split the monolithic test suite into focused submodules

interface TestBlock {
  module: string;
  lines: string[];
}

const MODULE_PATTERNS: Record<string, RegExp> = {
  governance: /admin|operator|pause|unpause|propose|accept|renounce/i,
  calculation: /calculate|sla|mttr|severity|threshold|reward|penalty/i,
  history: /history|prune|record/i,
  performance: /stress|1000|bulk|perf/i,
};

function classifyTest(name: string): string {
  for (const [module, pattern] of Object.entries(MODULE_PATTERNS)) {
    if (pattern.test(name)) return module;
  }
  return "misc";
}

function splitTestBlocks(src: string): Map<string, TestBlock> {
  const map = new Map<string, TestBlock>();
  const fnRegex = /fn\s+(test_\w+)/g;
  let match: RegExpExecArray | null;

  while ((match = fnRegex.exec(src)) !== null) {
    const name = match[1];
    const module = classifyTest(name);
    if (!map.has(module)) {
      map.set(module, { module, lines: [`// Module: ${module}`] });
    }
    map.get(module)!.lines.push(`// ${name}`);
  }
  return map;
}

function renderPlan(blocks: Map<string, TestBlock>): void {
  console.log("[SC-002] Proposed submodule split:");
  for (const [mod, block] of blocks) {
    console.log(`  mod ${mod} — ${block.lines.length - 1} test(s)`);
  }
}

function main(): void {
  const src = process.argv[2] ?? "";
  if (!src) {
    console.error("Usage: ts-node sc002-split-tests.ts <tests.rs content string>");
    process.exit(1);
  }
  const blocks = splitTestBlocks(src);
  renderPlan(blocks);
}

main();
