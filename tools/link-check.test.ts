import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { checkLinksInFile, extractLocalLinks } from "./link-check.ts";

test("extractLocalLinks ignores external and anchor links", () => {
  const markdown = "See [Code of Conduct](CODE_OF_CONDUCT.md), [docs](./docs/PROJECT_CONTEXT.md), [section](#section), and [external](https://example.com).";
  assert.deepEqual(extractLocalLinks(markdown), ["CODE_OF_CONDUCT.md", "./docs/PROJECT_CONTEXT.md"]);
});

test("checkLinksInFile reports missing targets", () => {
  const tempDir = mkdtempSync(path.join(os.tmpdir(), "link-check-"));
  const filePath = path.join(tempDir, "CONTRIBUTING.md");
  writeFileSync(filePath, "See [ok](./ok.txt) and [missing](./missing.txt).\n");
  writeFileSync(path.join(tempDir, "ok.txt"), "ok\n");

  try {
    const result = checkLinksInFile(filePath);
    assert.equal(result.missing.length, 1);
    assert.equal(result.missing[0]?.target, "./missing.txt");
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test("checkLinksInFile resolves the repository contributing guide", () => {
  const result = checkLinksInFile(path.resolve("CONTRIBUTING.md"));
  assert.equal(result.missing.length, 0);
});
