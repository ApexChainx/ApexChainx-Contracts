/**
 * #762 — Integration test: a project-level `sorostream.config.ts` file in a
 * fixture project actually influences a constructed `SoroStreamClient`'s
 * options.
 *
 * Covers:
 *   - `loadSorostreamConfig` reads and returns the fixture config.
 *   - `SoroStreamClient.create()` merges config-file options into the client.
 *   - Runtime options override config-file options (highest precedence).
 *   - Factory-function default exports are called and their return value used.
 *   - When no config file exists, the client falls back to defaults.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

import {
  SoroStreamClient,
  loadSorostreamConfig,
  DEFAULT_SOROSTREAM_OPTIONS,
} from "../src/index";

const __dirname = dirname(fileURLToPath(import.meta.url));

const fixtureDir = resolve(__dirname, "fixtures", "sorostream-config-project");
const fixtureFnDir = resolve(__dirname, "fixtures", "sorostream-config-fn-project");
const emptyDir = resolve(__dirname, "fixtures", "sorostream-empty-project");

describe("#762 SoroStreamClient config-file integration", () => {
  it("loadSorostreamConfig reads the fixture sorostream.config.ts", async () => {
    const config = await loadSorostreamConfig(fixtureDir);

    assert.strictEqual(config.rpcUrl, "https://soroban-testnet.stellar.org");
    assert.strictEqual(config.networkPassphrase, "Test SDF Network ; September 2015");
    assert.strictEqual(config.contractId, "CDXFIXTURECONFIGFILE000000000000000000000000000000");
    assert.strictEqual(config.pollIntervalMs, 15_000);
    assert.strictEqual(config.startLedger, 1_000);
    assert.strictEqual(config.maxReconnectAttempts, 10);
    assert.strictEqual(config.timeoutMs, 45_000);
  });

  it("SoroStreamClient.create merges config-file options into the client", async () => {
    const client = await SoroStreamClient.create({}, fixtureDir);
    const opts = client.getOptions();

    // Every value comes from the fixture config file, not defaults.
    assert.strictEqual(opts.rpcUrl, "https://soroban-testnet.stellar.org");
    assert.strictEqual(opts.networkPassphrase, "Test SDF Network ; September 2015");
    assert.strictEqual(opts.contractId, "CDXFIXTURECONFIGFILE000000000000000000000000000000");
    assert.strictEqual(opts.pollIntervalMs, 15_000);
    assert.strictEqual(opts.startLedger, 1_000);
    assert.strictEqual(opts.maxReconnectAttempts, 10);
    assert.strictEqual(opts.timeoutMs, 45_000);
  });

  it("runtime options override config-file options", async () => {
    const client = await SoroStreamClient.create(
      { pollIntervalMs: 3_000, rpcUrl: "https://override.example.com" },
      fixtureDir,
    );
    const opts = client.getOptions();

    // Runtime overrides config file.
    assert.strictEqual(opts.pollIntervalMs, 3_000);
    assert.strictEqual(opts.rpcUrl, "https://override.example.com");

    // Config-file values still present for non-overridden fields.
    assert.strictEqual(opts.networkPassphrase, "Test SDF Network ; September 2015");
    assert.strictEqual(opts.maxReconnectAttempts, 10);
  });

  it("factory-function default export is called and its return value used", async () => {
    const client = await SoroStreamClient.create({}, fixtureFnDir);
    const opts = client.getOptions();

    assert.strictEqual(opts.rpcUrl, "https://factory-fn-fixture.stellar.org");
    assert.strictEqual(opts.pollIntervalMs, 7_500);
    // Fields not set by the config file fall back to defaults.
    assert.strictEqual(opts.timeoutMs, DEFAULT_SOROSTREAM_OPTIONS.timeoutMs);
  });

  it("falls back to defaults when no config file exists", async () => {
    const config = await loadSorostreamConfig(emptyDir);
    assert.deepStrictEqual(config, {});

    const client = await SoroStreamClient.create({}, emptyDir);
    const opts = client.getOptions();
    assert.deepStrictEqual(opts, DEFAULT_SOROSTREAM_OPTIONS);
  });
});
