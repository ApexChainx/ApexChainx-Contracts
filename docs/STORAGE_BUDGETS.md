# Storage Budgets & TTL Management

## Ledger Rent

Soroban contracts pay ledger rent on instance storage. Without proactive TTL bumps, contracts get archived after the rent window expires.

### TTL Thresholds

- **Threshold**: 100,000 ledgers
- **Extend To**: 1,000,000 ledgers (~5 days on mainnet)

When TTL falls below threshold, a bump extends it to full amount.

### Bump Timing

TTL is bumped after every state-mutating operation:
- `set_config` — configuration changes
- `migrate` — schema upgrades
- `pause` — pause state
- `unpause` — resume state
- `calculate_sla` — history writes
- `prune_history` — history cleanup

### Rent Math

On Soroban (Stellar network):
- Base: ~0.0001 XLM per ledger per byte
- Instance storage: ~1000 bytes typical
- Monthly cost: ~3 XLM

Regular bumping prevents archival and data loss.
