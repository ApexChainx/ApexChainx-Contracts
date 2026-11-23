/**
 * Fixture project config using a **factory-function** default export
 * (#762).  Verifies that `loadSorostreamConfig` calls the function and uses
 * its return value rather than treating the function itself as options.
 */
export default () => ({
  rpcUrl: "https://factory-fn-fixture.stellar.org",
  pollIntervalMs: 7_500,
});
