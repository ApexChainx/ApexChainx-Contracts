/**
 * SC-W5-069: Property tests for idempotent duplicate submissions.
 * Verifies that submitting the same outage report twice yields the same result.
 */

interface OutageReport { id: string; mttr: number; severity: string }
type SubmitResult = "accepted" | "duplicate" | "invalid";

const submitted = new Set<string>();

function submitOutage(report: OutageReport): SubmitResult {
  if (!report.id || report.mttr < 0) return "invalid";
  if (submitted.has(report.id)) return "duplicate";
  submitted.add(report.id);
  return "accepted";
}

const SAMPLE_REPORTS: OutageReport[] = [
  { id: "outage-001", mttr: 30, severity: "critical" },
  { id: "outage-002", mttr: 120, severity: "high" },
  { id: "outage-003", mttr: 0, severity: "medium" },
];

describe("SC-W5-069 Idempotent Duplicate Submissions", () => {
  beforeEach(() => submitted.clear());

  it("first submission is accepted", () => {
    for (const r of SAMPLE_REPORTS) {
      expect(submitOutage(r)).toBe("accepted");
    }
  });

  it("duplicate submission returns duplicate — not accepted again", () => {
    submitOutage(SAMPLE_REPORTS[0]);
    expect(submitOutage(SAMPLE_REPORTS[0])).toBe("duplicate");
  });

  it("idempotent: repeated duplicate calls always return duplicate", () => {
    submitOutage(SAMPLE_REPORTS[1]);
    expect(submitOutage(SAMPLE_REPORTS[1])).toBe("duplicate");
    expect(submitOutage(SAMPLE_REPORTS[1])).toBe("duplicate");
  });

  it("negative mttr is rejected as invalid", () => {
    expect(submitOutage({ id: "bad-001", mttr: -1, severity: "high" })).toBe("invalid");
  });

  it("distinct ids are each accepted once", () => {
    for (const r of SAMPLE_REPORTS) {
      expect(submitOutage(r)).toBe("accepted");
    }
    expect(submitted.size).toBe(SAMPLE_REPORTS.length);
  });
});
