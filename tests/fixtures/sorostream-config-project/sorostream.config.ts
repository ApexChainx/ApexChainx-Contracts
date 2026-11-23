/**
 * Fixture project config for the SoroStreamClient config-file integration
 * test (#762).  Every value below differs from DEFAULT_SOROSTREAM_OPTIONS so
 * the test can distinguish "loaded from config file" from "fell back to
 * defaults".
 */
export default {
  rpcUrl: "https://soroban-testnet.stellar.org",
  networkPassphrase: "Test SDF Network ; September 2015",
  contractId: "CDXFIXTURECONFIGFILE000000000000000000000000000000",
  pollIntervalMs: 15_000,
  startLedger: 1_000,
  maxReconnectAttempts: 10,
  timeoutMs: 45_000,
};
