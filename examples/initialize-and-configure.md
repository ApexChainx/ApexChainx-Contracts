# Initialize and Configure the Contract

This example shows the basic bootstrap flow for the ApexChainx SLA calculator contract.

## 1. Deploy and initialize

1. Build the WASM artifact:
   ```bash
   cd apexchainx_calculator
   cargo build --target wasm32-unknown-unknown --release
   ```
2. Deploy the contract with Soroban and capture the returned contract ID.
3. Initialize the contract with an admin and operator:
   ```bash
   soroban contract invoke \
     --id <contract-id> \
     --source-account <source-account> \
     --network <network-name> \
     -- initialize \
     --admin <admin-address> \
     --operator <operator-address>
   ```

## 2. Configure SLA thresholds

Set a severity policy for the contract:

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- set_config \
  --caller <admin-address> \
  --severity critical \
  --threshold 60 \
  --penalty 100 \
  --reward 50
```

## 3. Read back the configuration

```bash
soroban contract invoke \
  --id <contract-id> \
  --source-account <source-account> \
  --network <network-name> \
  -- get_config \
  --severity critical
```

This gives you a quick starting point for local testing and contract setup.
