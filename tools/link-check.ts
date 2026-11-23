import { existsSync, readFileSync } from "fs";
import path from "path";

export interface LinkCheckResult {
  checkedFile: string;
  links: string[];
  missing: Array<{ target: string; resolvedPath: string }>;
}

function isExternalReference(target: string): boolean {
  if (!target || target.startsWith("#")) {
    return true;
  }

  if (target.startsWith("//")) {
    return true;
  }

  return /^(?:[a-z]+:|\/\/)/i.test(target);
}

export function extractLocalLinks(markdown: string): string[] {
  const links: string[] = [];
  const pattern = /(?<!!)\[([^\]]+)\]\(([^)]+)\)/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(markdown)) !== null) {
    const rawTarget = match[2].trim();
    const target = rawTarget.split(/\s+/)[0].replace(/^<|>$/g, "");

    if (!isExternalReference(target)) {
      links.push(target);
    }
  }

  return links;
}

export function checkLinksInFile(markdownFilePath: string): LinkCheckResult {
  const absolutePath = path.resolve(markdownFilePath);
  const markdown = readFileSync(absolutePath, "utf8");
  const links = extractLocalLinks(markdown);
  const missing = links
    .map((target) => {
      const [targetPath] = target.split("#");
      const cleanedTarget = targetPath.split("?")[0];

      if (!cleanedTarget) {
        return null;
      }

      const resolvedPath = path.resolve(path.dirname(absolutePath), cleanedTarget);
      return existsSync(resolvedPath)
        ? null
        : { target, resolvedPath };
    })
    .filter((entry): entry is { target: string; resolvedPath: string } => entry !== null);

  return { checkedFile: absolutePath, links, missing };
}

export function main(args: string[] = process.argv.slice(2)): void {
  const targetArg = args[0] ? path.resolve(process.cwd(), args[0]) : path.resolve(process.cwd(), "CONTRIBUTING.md");
  const result = checkLinksInFile(targetArg);

  if (result.missing.length > 0) {
    console.error(`Broken links found in ${path.relative(process.cwd(), result.checkedFile)}:`);
    for (const entry of result.missing) {
      console.error(`- ${entry.target} -> ${entry.resolvedPath}`);
    }
    process.exit(1);
  }

  console.log(`Checked ${result.links.length} local reference(s) in ${path.relative(process.cwd(), result.checkedFile)}; all resolved successfully.`);
}

if (process.argv[1] && process.argv[1].endsWith("link-check.ts")) {
  main();
}
