/**
 * moduleMap.ts
 * Issue #155 — documents the intended module split for apexchainx_calculator/src/lib.rs.
 * Use this as the authoritative reference when executing the refactor.
 */

export type ModuleName =
  | "governance"
  | "config"
  | "calculation"
  | "history"
  | "metadata";

export interface ModuleSpec {
  name: ModuleName;
  rustFile: string;
  publicFns: string[];
  internalOnly: boolean;
}

export const MODULE_MAP: ModuleSpec[] = [
  {
    name: "governance",
    rustFile: "src/governance.rs",
    publicFns: [
      "propose_admin", "accept_admin", "renounce_admin",
      "propose_operator", "accept_operator",
      "get_pending_admin", "get_pending_operator",
    ],
    internalOnly: false,
  },
  {
    name: "config",
    rustFile: "src/config.rs",
    publicFns: ["set_config", "get_config", "get_config_snapshot"],
    internalOnly: false,
  },
  {
    name: "calculation",
    rustFile: "src/calculation.rs",
    publicFns: ["calculate_sla", "calculate_sla_view", "get_stats"],
    internalOnly: false,
  },
  {
    name: "history",
    rustFile: "src/history.rs",
    publicFns: ["get_history", "prune_history"],
    internalOnly: false,
  },
  {
    name: "metadata",
    rustFile: "src/metadata.rs",
    publicFns: ["pause", "unpause", "get_pause_info", "is_paused"],
    internalOnly: false,
  },
];

/** Returns the module that owns a given public function, or undefined. */
export function findOwner(fn: string): ModuleSpec | undefined {
  return MODULE_MAP.find((m) => m.publicFns.includes(fn));
}
