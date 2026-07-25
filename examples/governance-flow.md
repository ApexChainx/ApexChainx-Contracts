# Governance Flow Example

This example walks through the two-step governance transitions for admin and operator roles.

## Admin handoff

1. Current admin proposes a new admin:
   ```bash
   soroban contract invoke \
     --id <contract-id> \
     --source-account <source-account> \
     --network <network-name> \
     -- propose_admin \
     --caller <current-admin> \
     --new_admin <new-admin>
   ```
2. The proposed admin accepts the role:
   ```bash
   soroban contract invoke \
     --id <contract-id> \
     --source-account <source-account> \
     --network <network-name> \
     -- accept_admin \
     --caller <new-admin>
   ```

## Operator handoff

1. Admin proposes a new operator.
2. The proposed operator accepts the role.
3. The active operator can be read from the contract state through the corresponding query functions.

This flow is intentionally two-step to reduce accidental privilege changes.
