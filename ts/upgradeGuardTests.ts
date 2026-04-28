// SC-023: Init and upgrade guard tests for storage version mismatches.
// Covers uninitialized, current, older, and unsupported version scenarios.

export const CURRENT_VERSION = 2;

export type VersionStatus = "uninitialized" | "current" | "older" | "unsupported";

export function classifyVersion(stored: number | null): VersionStatus {
  if (stored === null) return "uninitialized";
  if (stored === CURRENT_VERSION) return "current";
  if (stored > 0 && stored < CURRENT_VERSION) return "older";
  return "unsupported";
}

export function assertMigrationGuard(stored: number | null): void {
  const status = classifyVersion(stored);
  if (status === "uninitialized") throw new Error("Contract not initialized");
  if (status === "unsupported") throw new Error(`Unsupported storage version: ${stored}`);
}

// --- guard tests ---

function test(label: string, fn: () => void): void {
  try {
    fn();
    console.log(`PASS  ${label}`);
  } catch (e) {
    console.error(`FAIL  ${label}: ${(e as Error).message}`);
  }
}

function expect(actual: unknown, expected: unknown): void {
  if (actual !== expected) throw new Error(`expected ${expected}, got ${actual}`);
}

function expectThrows(fn: () => void, msg: string): void {
  try { fn(); throw new Error("no error thrown"); }
  catch (e) { if (!(e as Error).message.includes(msg)) throw e; }
}

test("uninitialized version returns uninitialized", () => expect(classifyVersion(null), "uninitialized"));
test("version 2 returns current", () => expect(classifyVersion(2), "current"));
test("version 1 returns older", () => expect(classifyVersion(1), "older"));
test("version 99 returns unsupported", () => expect(classifyVersion(99), "unsupported"));
test("guard throws on uninitialized", () => expectThrows(() => assertMigrationGuard(null), "not initialized"));
test("guard throws on unsupported", () => expectThrows(() => assertMigrationGuard(99), "Unsupported"));
test("guard passes on current version", () => assertMigrationGuard(2));
