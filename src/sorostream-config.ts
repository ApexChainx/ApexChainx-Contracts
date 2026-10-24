/**
 * #762 — Project-level configuration loader for `SoroStreamClient`.
 *
 * A `sorostream.config.ts` (or `.js`) file placed at the project root can
 * supply default `SoroStreamClientOptions` that `SoroStreamClient` merges
 * with its runtime options during construction.  This module is the single
 * place that resolves and imports that file.
 *
 * Supported config-file shapes (default export):
 *   1. A plain object of `Partial<SoroStreamClientOptions>`.
 *   2. A function returning `Partial<SoroStreamClientOptions>` (useful when
 *      options depend on environment variables or runtime checks).
 *
 * A named `options` export is also accepted as a fallback.
 *
 * Lookup priority (first match wins):
 *   1. `sorostream.config.ts`  — preferred under tsx / dev tooling.
 *   2. `sorostream.config.js`  — production / compiled output.
 *
 * When no file is found the loader resolves to an empty object so callers
 * can always shallow-merge without a null-check.
 */

import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

/**
 * Runtime options consumed by `SoroStreamClient`.
 *
 * Every field is optional at the config-file and constructor level;
 * `SoroStreamClient` applies {@link DEFAULT_SOROSTREAM_OPTIONS} for any
 * field left unset.
 */
export interface SoroStreamClientOptions {
  /** Soroban RPC endpoint URL. */
  rpcUrl: string;
  /** Stellar network passphrase (testnet, mainnet, etc.). */
  networkPassphrase: string;
  /** Contract ID (C-address) whose events to stream. */
  contractId: string;
  /** Polling interval in milliseconds between ledger checks. */
  pollIntervalMs: number;
  /** Ledger sequence to start streaming from (0 = latest closed). */
  startLedger: number;
  /** Maximum reconnection attempts before the stream gives up. */
  maxReconnectAttempts: number;
  /** Per-request timeout in milliseconds for Soroban RPC calls. */
  timeoutMs: number;
}

/** Default option values applied when neither config file nor runtime supplies a field. */
export const DEFAULT_SOROSTREAM_OPTIONS: SoroStreamClientOptions = {
  rpcUrl: "",
  networkPassphrase: "",
  contractId: "",
  pollIntervalMs: 5_000,
  startLedger: 0,
  maxReconnectAttempts: 5,
  timeoutMs: 30_000,
};

/** Config-file basenames, searched in priority order. */
const CONFIG_FILE_NAMES = ["sorostream.config.ts", "sorostream.config.js"] as const;

/** Either a partial options object or a factory function returning one. */
type ConfigExport =
  | Partial<SoroStreamClientOptions>
  | (() => Partial<SoroStreamClientOptions>);

/**
 * Resolve the first existing config file in `dir`.
 * @returns Absolute path to the config file, or `undefined`.
 */
function resolveConfigFile(dir: string): string | undefined {
  for (const name of CONFIG_FILE_NAMES) {
    const candidate = resolve(dir, name);
    if (existsSync(candidate)) return candidate;
  }
  return undefined;
}

/**
 * Normalise a dynamically-imported module namespace into a partial options
 * object.
 *
 * Handles:
 *   - ESM `export default { … }`          → `ns.default` is the object
 *   - ESM `export default () => ({ … })`  → `ns.default` is the factory
 *   - CJS `module.exports = { … }`        → `ns.default` is `module.exports`
 *   - Named `export const options = { … }` → `ns.options` is the object
 */
function extractOptions(
  ns: Record<string, unknown>,
): Partial<SoroStreamClientOptions> {
  const def = ns.default;
  if (def !== undefined && def !== null) {
    if (typeof def === "function") {
      return (def as () => Partial<SoroStreamClientOptions>)() ?? {};
    }
    if (typeof def === "object") {
      return def as Partial<SoroStreamClientOptions>;
    }
  }

  const named = ns.options;
  if (named !== undefined && named !== null && typeof named === "object") {
    return named as Partial<SoroStreamClientOptions>;
  }

  return {};
}

/**
 * Load a project-level `sorostream.config.ts` (or `.js`) file from `cwd`
 * and return its exported options.
 *
 * Returns an empty object when no config file is present, so callers can
 * always shallow-merge without null-checks.
 *
 * @param cwd Directory to search for the config file. Defaults to `process.cwd()`.
 * @returns Partial options from the config file (may be empty).
 */
export async function loadSorostreamConfig(
  cwd: string = process.cwd(),
): Promise<Partial<SoroStreamClientOptions>> {
  const filePath = resolveConfigFile(cwd);
  if (filePath === undefined) return {};

  const fileUrl = pathToFileURL(filePath).href;
  const ns = (await import(fileUrl)) as Record<string, unknown>;
  return extractOptions(ns);
}
