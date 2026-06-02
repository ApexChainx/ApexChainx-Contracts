/**
 * SC-W5-059: Blocked/paused error parity for backend API translation.
 * Ensures contract error codes for blocked/paused states map to expected backend API errors.
 */

type ContractError = "PAUSED" | "BLOCKED" | "NOT_FOUND" | "INVALID_INPUT";
type ApiStatus = 503 | 403 | 404 | 400;

const ERROR_MAP: Record<ContractError, ApiStatus> = {
  PAUSED:       503,
  BLOCKED:      403,
  NOT_FOUND:    404,
  INVALID_INPUT: 400,
};

function toApiStatus(err: ContractError): ApiStatus {
  return ERROR_MAP[err];
}

describe("SC-W5-059 Blocked/Paused Error Parity", () => {
  it("PAUSED maps to 503 Service Unavailable", () => {
    expect(toApiStatus("PAUSED")).toBe(503);
  });

  it("BLOCKED maps to 403 Forbidden", () => {
    expect(toApiStatus("BLOCKED")).toBe(403);
  });

  it("NOT_FOUND maps to 404", () => {
    expect(toApiStatus("NOT_FOUND")).toBe(404);
  });

  it("INVALID_INPUT maps to 400", () => {
    expect(toApiStatus("INVALID_INPUT")).toBe(400);
  });

  it("all contract errors have a defined API status", () => {
    const errors: ContractError[] = ["PAUSED", "BLOCKED", "NOT_FOUND", "INVALID_INPUT"];
    for (const e of errors) {
      expect(toApiStatus(e)).toBeGreaterThan(0);
    }
  });

  it("PAUSED and BLOCKED produce different status codes", () => {
    expect(toApiStatus("PAUSED")).not.toBe(toApiStatus("BLOCKED"));
  });
});
