import { EVENT_VERSIONS } from "./generated/contractConstants";

export type EventName = keyof typeof EVENT_VERSIONS;
export type EventHandler<T> = (payload: unknown, context: unknown) => T;

/** Consumer dispatch is keyed by BOTH name and version. No global-version fallback. */
export class EventDispatcher<T> {
  private readonly handlers = new Map<string, EventHandler<T>>();

  register(name: EventName, version: string, handler: EventHandler<T>): void {
    // Explicit older decoders may coexist for replaying historical events.
    if (!Object.hasOwn(EVENT_VERSIONS, name)) throw new Error("Unknown event name");
    if (!/^v[1-9][0-9]*$/.test(version)) throw new Error("Invalid event version");
    const key = `${name}:${version}`;
    if (this.handlers.has(key)) throw new Error(`Duplicate decoder: ${key}`);
    this.handlers.set(key, handler);
  }

  dispatch(topics: readonly unknown[], payload: unknown): T {
    if (topics.length !== 3 || typeof topics[0] !== "string" || typeof topics[1] !== "string") {
      throw new Error("Invalid event topics");
    }
    const key = `${topics[0]}:${topics[1]}`;
    const handler = this.handlers.get(key);
    if (!handler) throw new Error(`Unsupported event schema: ${key}`);
    return handler(payload, topics[2]);
  }
}

/** v2 correlation encoding; use bigint to avoid losing the high ledger bits. */
export function correlationId(outageId: string, ledgerSequence: number): bigint {
  if (!Number.isInteger(ledgerSequence) || ledgerSequence < 0 || ledgerSequence > 0xffffffff) {
    throw new Error("Ledger sequence must be a u32");
  }
  if (!/^[A-Za-z0-9_]{0,32}$/.test(outageId)) throw new Error("Invalid Soroban symbol");
  let fingerprint = 0x811c9dc5;
  for (const byte of Buffer.from(outageId, "ascii")) {
    fingerprint = Math.imul(fingerprint ^ byte, 0x01000193) >>> 0;
  }
  return (BigInt(ledgerSequence) << 32n) | BigInt(fingerprint);
}
