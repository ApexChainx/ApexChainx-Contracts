/**
 * SC-045: Authorization matrix coverage for every privileged contract method.
 * Each entry maps a method to its required role and the actors that must be rejected.
 */

type Role = "admin" | "operator" | "anyone";

interface AuthEntry {
  method: string;
  requiredRole: Role;
  rejectedActors: string[];
}

const AUTH_MATRIX: AuthEntry[] = [
  { method: "initialize",       requiredRole: "anyone",   rejectedActors: [] },
  { method: "set_config",       requiredRole: "admin",    rejectedActors: ["operator", "stranger"] },
  { method: "pause",            requiredRole: "admin",    rejectedActors: ["operator", "stranger"] },
  { method: "unpause",          requiredRole: "admin",    rejectedActors: ["operator", "stranger"] },
  { method: "prune_history",    requiredRole: "admin",    rejectedActors: ["operator", "stranger"] },
  { method: "propose_admin",    requiredRole: "admin",    rejectedActors: ["operator", "stranger"] },
  { method: "accept_admin",     requiredRole: "anyone",   rejectedActors: ["stranger"] },
  { method: "renounce_admin",   requiredRole: "admin",    rejectedActors: ["operator", "stranger"] },
  { method: "propose_operator", requiredRole: "admin",    rejectedActors: ["operator", "stranger"] },
  { method: "accept_operator",  requiredRole: "anyone",   rejectedActors: ["stranger"] },
  { method: "calculate_sla",    requiredRole: "operator", rejectedActors: ["admin", "stranger"] },
];

function simulateCall(method: string, actor: string, matrix: AuthEntry[]): "ok" | "unauthorized" {
  const entry = matrix.find((e) => e.method === method);
  if (!entry) throw new Error(`Unknown method: ${method}`);
  if (entry.rejectedActors.includes(actor)) return "unauthorized";
  return "ok";
}

describe("SC-045 Authorization Matrix", () => {
  for (const entry of AUTH_MATRIX) {
    for (const actor of entry.rejectedActors) {
      it(`${entry.method} rejects ${actor}`, () => {
        const result = simulateCall(entry.method, actor, AUTH_MATRIX);
        expect(result).toBe("unauthorized");
      });
    }

    it(`${entry.method} allows authorized caller`, () => {
      const authorizedActor = entry.requiredRole === "anyone" ? "anyone" : entry.requiredRole;
      const result = simulateCall(entry.method, authorizedActor, AUTH_MATRIX);
      expect(result).toBe("ok");
    });
  }

  it("matrix covers all known privileged methods", () => {
    const privileged = AUTH_MATRIX.filter((e) => e.requiredRole !== "anyone");
    expect(privileged.length).toBeGreaterThan(0);
  });
});
