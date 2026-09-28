/** Payload-only SCVal XDR budgets, cross-checked against actual Rust events. */
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
export const EVENT_SIZE_LIMITS = {
  sla_calc: 152, set_int: 164, cfg_upd: 60,
  paused: 20, unpause: 20, adm_prop: 56, op_prop: 56,
} as const;
type Kind = "symbol" | "u32" | "i128" | "u64" | "bool" | "address";
export interface Field { kind: Kind; value?: string }
export function estimateScValSize(fields: readonly Field[]): number {
  // SCVal discriminant, optional vector discriminant, vector length.
  return 12 + fields.reduce((size, field) => {
    if (field.kind === "symbol") {
      const length = Buffer.byteLength(field.value ?? "", "utf8");
      if (length > 32) throw new Error("Symbol exceeds 32 bytes");
      return size + 8 + Math.ceil(length / 4) * 4;
    }
    // Account addresses include SCAddress and PublicKey discriminants.
    return size + { u32: 8, bool: 8, i128: 20, u64: 12, address: 44 }[field.kind];
  }, 0);
}
const sla: Field[] = [
  { kind: "symbol", value: "ABCDEFGHIJKLMNOPQRSTUVWXYZ012345" },
  { kind: "symbol", value: "met" }, { kind: "u32" }, { kind: "u32" },
  { kind: "i128" }, { kind: "symbol", value: "rew" },
  { kind: "symbol", value: "excel" }, { kind: "u64" }, { kind: "u64" },
];
const payloads: Record<keyof typeof EVENT_SIZE_LIMITS, Field[]> = {
  sla_calc: sla, set_int: [...sla, { kind: "u64" }],
  cfg_upd: [{ kind: "u32" }, { kind: "i128" }, { kind: "i128" }],
  paused: [{ kind: "bool" }], unpause: [{ kind: "bool" }],
  adm_prop: [{ kind: "address" }], op_prop: [{ kind: "address" }],
};
export function runChecks(): void {
  const measured = JSON.parse(readFileSync(new URL("../ts/fixtures/event-size-semantics.json", import.meta.url), "utf8")) as
    { name: keyof typeof EVENT_SIZE_LIMITS; fields: number; xdrBytes: number }[];
  if (new Set(measured.map(row => row.name)).size !== Object.keys(payloads).length || measured.length !== Object.keys(payloads).length)
    throw new Error("Event-size fixture coverage mismatch");
  for (const row of measured) {
    const fields = payloads[row.name];
    if (!fields || fields.length !== row.fields || estimateScValSize(fields) !== row.xdrBytes)
      throw new Error(`${row.name}: TypeScript payload estimate disagrees with Rust XDR`);
    const limit = EVENT_SIZE_LIMITS[row.name];
    if (row.xdrBytes > limit) throw new Error(`${row.name}: ${row.xdrBytes}B exceeds ${limit}B`);
    console.log(`${row.name}: ${row.xdrBytes}B / ${limit}B (Rust XDR verified)`);
  }
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) runChecks();
