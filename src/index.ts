/**
 * Public entry point for the ApexChainx SoroStream client package.
 *
 * Re-exports the `SoroStreamClient` and the project-level config loader
 * (`loadSorostreamConfig`) so applications can either let the client load
 * config automatically or call the loader directly.
 *
 * @example
 * // Let the client load config automatically
 * import { SoroStreamClient } from "apexchainx-contracts";
 * const client = await SoroStreamClient.create({ rpcUrl: "…" });
 *
 * @example
 * // Call the loader directly (e.g. to inspect or validate config before constructing)
 * import { loadSorostreamConfig } from "apexchainx-contracts";
 * const config = await loadSorostreamConfig();
 */

export { SoroStreamClient } from "./sorostream-client";
export {
  loadSorostreamConfig,
  DEFAULT_SOROSTREAM_OPTIONS,
} from "./sorostream-config";
export type { SoroStreamClientOptions } from "./sorostream-config";
