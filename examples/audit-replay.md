# Audit Replay Example

This example shows how to replay SLA calculations and inspect the resulting history for auditing.

## 1. Run a calculation

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- calculate_sla \
  --caller <operator-address> \
  --outage_id outage-001 \
  --severity critical \
  --mttr 45
```

## 2. Inspect the calculation history

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- get_history
```

## 3. Compare with a read-only replay

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- calculate_sla_view \
  --outage_id outage-001 \
  --severity critical \
  --mttr 45
```

These examples help operators and auditors validate that contract behavior remains deterministic over time.
