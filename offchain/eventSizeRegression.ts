/**
 * SC-017: Event-size regression tests for lifecycle and SLA calculation events.
 * Catches payload bloat before deployment by asserting max byte sizes per event type.
 */

const EVENT_SIZE_LIMITS: Record<string, number> = {
  sla_calculated: 200,
  contract_paused: 100,
  contract_unpaused: 80,
  admin_proposed: 120,
  operator_proposed: 120,
};

interface ContractEvent {
  type: string;
  payload: Record<string, unknown>;
}

function eventSize(event: ContractEvent): number {
  return Buffer.byteLength(JSON.stringify(event.payload), "utf8");
}

function assertEventSize(event: ContractEvent): void {
  const limit = EVENT_SIZE_LIMITS[event.type];
  if (limit === undefined) throw new Error(`Unknown event type: ${event.type}`);
  const size = eventSize(event);
  if (size > limit) {
    throw new Error(
      `[SC-017] Event "${event.type}" is ${size}B, limit is ${limit}B`
    );
  }
  console.log(`  ✓ ${event.type}: ${size}B / ${limit}B`);
}

const events: ContractEvent[] = [
  {
    type: "sla_calculated",
    payload: { severity: "critical", mttr: 45, sla_met: true, reward_tier: "good" },
  },
  {
    type: "contract_paused",
    payload: { reason: "maintenance", timestamp: 1714233600 },
  },
  {
    type: "contract_unpaused",
    payload: { timestamp: 1714237200 },
  },
  {
    type: "admin_proposed",
    payload: { proposed: "GABC123", proposer: "GXYZ456" },
  },
  {
    type: "operator_proposed",
    payload: { proposed: "GDEF789", proposer: "GXYZ456" },
  },
];

console.log("[SC-017] Event-size regression checks:");
events.forEach(assertEventSize);
console.log("All event-size checks passed.");
