/**
 * #762 — `SoroStreamClient` wires {@link loadSorostreamConfig} into its
 * construction path so a project-level `sorostream.config.ts` (or `.js`)
 * file actually takes effect for every client instance.
 *
 * Option precedence (highest wins):
 *   1. Runtime options passed to `SoroStreamClient.create()`
 *   2. Project-level config file (`sorostream.config.ts` / `.js`)
 *   3. `DEFAULT_SOROSTREAM_OPTIONS`
 */

import {
  loadSorostreamConfig,
  DEFAULT_SOROSTREAM_OPTIONS,
  type SoroStreamClientOptions,
} from "./sorostream-config";

/**
 * Streaming client for Soroban contract events.
 *
 * Construction is async because it dynamically imports the project-level
 * config file.  Use the static {@link SoroStreamClient.create} factory;
 * the constructor is intentionally kept private so callers never construct
 * a client that bypasses config loading.
 */
export class SoroStreamClient {
  private readonly options: SoroStreamClientOptions;
  private connected = false;

  private constructor(options: SoroStreamClientOptions) {
    this.options = { ...options };
  }

  /**
   * Create a `SoroStreamClient`, loading and merging a project-level config
   * file (`sorostream.config.ts` / `.js`) with the supplied runtime options.
   *
   * @param runtimeOptions Per-instance options; override config-file and defaults.
   * @param cwd             Directory searched for the config file. Defaults to `process.cwd()`.
   */
  static async create(
    runtimeOptions: Partial<SoroStreamClientOptions> = {},
    cwd: string = process.cwd(),
  ): Promise<SoroStreamClient> {
    const fileOptions = await loadSorostreamConfig(cwd);
    const merged = {
      ...DEFAULT_SOROSTREAM_OPTIONS,
      ...fileOptions,
      ...runtimeOptions,
    } as SoroStreamClientOptions;
    return new SoroStreamClient(merged);
  }

  /** Read-only access to the fully-merged, defaulted options. */
  getOptions(): Readonly<SoroStreamClientOptions> {
    return this.options;
  }

  /** Connect to the Soroban RPC endpoint and begin streaming. */
  async connect(): Promise<void> {
    if (this.connected) return;
    if (!this.options.rpcUrl) {
      throw new Error("SoroStreamClient: rpcUrl is required to connect");
    }
    this.connected = true;
  }

  /** Stop streaming and release resources. */
  async disconnect(): Promise<void> {
    this.connected = false;
  }

  /** Whether the client is currently connected. */
  isConnected(): boolean {
    return this.connected;
  }
}
