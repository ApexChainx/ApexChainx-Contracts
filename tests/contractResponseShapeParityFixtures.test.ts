/**
 * SC-W5-057: Contract response shape parity fixtures for backend decoders.
 * Pins the exact shape of contract responses so backend decoders stay aligned.
 */

interface SlaResponse {
  outage_id: string;
  verdict: "met" | "violated" | "invalid";
  penalty_bps: number;
  mttr_minutes: number;
}

function buildResponse(outage_id: string, mttr: number, threshold: number, penaltyBps: number): SlaResponse {
  if (mttr < 0 || threshold <= 0) return { outage_id, verdict: "invalid", penalty_bps: 0, mttr_minutes: mttr };
  const verdict = mttr <= threshold ? "met" : "violated";
  return { outage_id, verdict, penalty_bps: verdict === "violated" ? penaltyBps : 0, mttr_minutes: mttr };
}

const FIXTURES: SlaResponse[] = [
  { outage_id: "o-001", verdict: "met",      penalty_bps: 0,   mttr_minutes: 30  },
  { outage_id: "o-002", verdict: "violated",  penalty_bps: 500, mttr_minutes: 90  },
  { outage_id: "o-003", verdict: "invalid",   penalty_bps: 0,   mttr_minutes: -1  },
];

describe("SC-W5-057 Contract Response Shape Parity", () => {
  it("met response has zero penalty and correct fields", () => {
    const res = buildResponse("o-001", 30, 60, 500);
    expect(res).toEqual(FIXTURES[0]);
  });

  it("violated response carries penalty_bps", () => {
    const res = buildResponse("o-002", 90, 60, 500);
    expect(res).toEqual(FIXTURES[1]);
  });

  it("invalid response has zero penalty regardless of bps", () => {
    const res = buildResponse("o-003", -1, 60, 500);
    expect(res).toEqual(FIXTURES[2]);
  });

  it("response always contains all required fields", () => {
    const res = buildResponse("o-004", 60, 60, 300);
    expect(Object.keys(res).sort()).toEqual(["mttr_minutes", "outage_id", "penalty_bps", "verdict"]);
  });
});
