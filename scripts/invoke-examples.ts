/**
 * SC-036: Executable invocation examples for the current contract surface.
 * Covers governance flows and parity-oriented read flows.
 */

import { execSync } from "child_process";

const CONTRACT_ID = process.env.CONTRACT_ID ?? "<contract-id>";
const SOURCE = process.env.SOURCE_ACCOUNT ?? "<source-account>";
const NETWORK = process.env.NETWORK ?? "testnet";

function invoke(fn: string, args: Record<string, string> = {}): string {
  const flags = Object.entries(args)
    .map(([k, v]) => `--${k} ${v}`)
    .join(" ");
  const cmd = `soroban contract invoke --id ${CONTRACT_ID} --source-account ${SOURCE} --network ${NETWORK} -- ${fn} ${flags}`;
  console.log(`\n$ ${cmd}`);
  try {
    return execSync(cmd, { encoding: "utf8" });
  } catch (e: any) {
    return e.stderr ?? String(e);
  }
}

// --- Read flows (parity-oriented) ---
function readExamples() {
  console.log("=== Config snapshot ===");
  console.log(invoke("get_config_snapshot"));

  console.log("=== Stats ===");
  console.log(invoke("get_stats"));

  console.log("=== Result schema ===");
  console.log(invoke("get_result_schema"));

  console.log("=== SLA view (critical, 30 min MTTR) ===");
  console.log(invoke("calculate_sla_view", { severity: "critical", mttr_minutes: "30" }));
}

// --- Governance flows ---
function governanceExamples(newAdmin: string, newOperator: string) {
  console.log("=== Propose admin ===");
  console.log(invoke("propose_admin", { caller: SOURCE, new_admin: newAdmin }));

  console.log("=== Pending admin ===");
  console.log(invoke("get_pending_admin"));

  console.log("=== Propose operator ===");
  console.log(invoke("propose_operator", { caller: SOURCE, new_operator: newOperator }));

  console.log("=== Pending operator ===");
  console.log(invoke("get_pending_operator"));
}

const [, , mode, arg1 = "", arg2 = ""] = process.argv;
if (mode === "read") readExamples();
else if (mode === "governance") governanceExamples(arg1, arg2);
else console.log("Usage: ts-node invoke-examples.ts <read|governance> [newAdmin] [newOperator]");
