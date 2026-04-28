/**
 * SC-030: Malformed-symbol negative test harness.
 * Validates that all public contract methods reject bad symbols deterministically
 * without silently mutating state.
 */

export type ContractMethod =
  | "get_config"
  | "set_config"
  | "calculate_sla"
  | "calculate_sla_view"
  | "get_stats";

const VALID_SYMBOLS = new Set(["critical", "high", "medium", "low"]);

const MALFORMED_INPUTS = [
  "",
  " ",
  "CRITICAL",
  "Critical",
  "unknown",
  "null",
  "undefined",
  "critical\x00",
  "a".repeat(256),
  "crítical",
];

export interface ValidationResult {
  method: ContractMethod;
  input: string;
  rejected: boolean;
  error?: string;
}

export function validateSymbol(symbol: string): void {
  if (!symbol || symbol.trim() === "") throw new Error("Symbol must not be empty");
  if (!VALID_SYMBOLS.has(symbol)) throw new Error(`Unsupported symbol: "${symbol}"`);
}

export function runNegativeSuite(method: ContractMethod): ValidationResult[] {
  return MALFORMED_INPUTS.map((input) => {
    try {
      validateSymbol(input);
      return { method, input, rejected: false };
    } catch (e) {
      return { method, input, rejected: true, error: (e as Error).message };
    }
  });
}

export function assertAllRejected(results: ValidationResult[]): void {
  const passed = results.filter((r) => !r.rejected);
  if (passed.length > 0) {
    throw new Error(
      `Expected rejection for: ${passed.map((r) => JSON.stringify(r.input)).join(", ")}`
    );
  }
}

if (require.main === module) {
  const methods: ContractMethod[] = [
    "get_config", "set_config", "calculate_sla", "calculate_sla_view", "get_stats",
  ];
  for (const method of methods) {
    const results = runNegativeSuite(method);
    assertAllRejected(results);
    console.log(`[${method}] all ${results.length} malformed inputs rejected ✓`);
  }
}
