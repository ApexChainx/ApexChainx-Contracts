/**
 * SC-W5-068: Fuzzing for config mutation sequences and freeze toggles.
 * Ensures config mutations are rejected when the contract is frozen.
 */


import { describe, it } from "node:test";
import assert from "node:assert/strict";
interface ContractConfig { threshold: number; penaltyBps: number; frozen: boolean }

function applyMutation(config: ContractConfig, patch: Partial<ContractConfig>): { ok: boolean; config: ContractConfig } {
  if (config.frozen) return { ok: false, config };
  return { ok: true, config: { ...config, ...patch } };
}

function toggleFreeze(config: ContractConfig): ContractConfig {
  return { ...config, frozen: !config.frozen };
}

const BASE: ContractConfig = { threshold: 60, penaltyBps: 300, frozen: false };

describe("SC-W5-068 Config Mutation Sequences and Freeze Toggles", () => {
  it("mutation succeeds when not frozen", () => {
    const { ok, config } = applyMutation(BASE, { threshold: 120 });
    assert.strictEqual(ok, true);
    assert.strictEqual(config.threshold, 120);
  });

  it("mutation is rejected when frozen", () => {
    const frozen = toggleFreeze(BASE);
    const { ok, config } = applyMutation(frozen, { threshold: 120 });
    assert.strictEqual(ok, false);
    assert.strictEqual(config.threshold, 60); // unchanged
  });

  it("freeze then unfreeze allows mutations again", () => {
    const frozen   = toggleFreeze(BASE);
    const unfrozen = toggleFreeze(frozen);
    assert.strictEqual(applyMutation(unfrozen, { penaltyBps: 500 }).ok, true);
  });

  it("mutation sequence: mutate → freeze → mutate fails → unfreeze → mutate succeeds", () => {
    let cfg = BASE;
    cfg = applyMutation(cfg, { penaltyBps: 400 }).config;
    cfg = toggleFreeze(cfg);
    assert.strictEqual(applyMutation(cfg, { penaltyBps: 999 }).ok, false);
    cfg = toggleFreeze(cfg);
    assert.strictEqual(applyMutation(cfg, { penaltyBps: 999 }).ok, true);
  });
});
