# ApexChainx Event Payload Compatibility & Ordering Policy

To maintain robust off-chain indexers and seamless frontend integration, all Soroban smart contracts within `ApexChainx-Contracts` must adhere to strict event schema and field ordering rules.

## 1. Field Ordering Rules (Append-Only)
* **Never reorder existing fields**: The positional sequence of payload fields within an event tuple or struct is immutable once deployed to production networks.
* **Append-only evolution**: Any additions to event payloads must be appended as new fields at the end of the structure. Removing fields is strictly prohibited; deprecate fields instead.

## 2. Backward-Compatibility Guarantees
* **Topic Stability**: Event topic symbols (defined in `policy.rs`) must remain constant. Changing a topic string constitutes a breaking protocol change requiring a major version increment.
* **Type Safety**: Field types within event schemas must remain static. Changing a field type (e.g., from `i128` to `u64`) breaks downstream indexers and is prohibited.

# ApexChainx Event Payload Compatibility & Ordering Policy

To maintain robust off-chain indexers and seamless frontend integration, all Soroban smart contracts within `ApexChainx-Contracts` must adhere to strict event schema and field ordering rules.

## 1. Field Ordering Rules (Append-Only)
* **Never reorder existing fields**: The positional sequence of payload fields within an event tuple or struct is immutable once deployed to production networks.
* **Append-only evolution**: Any additions to event payloads must be appended as new fields at the end of the structure. Removing fields is strictly prohibited; deprecate fields instead.

## 2. Backward-Compatibility Guarantees
* **Topic Stability**: Event topic symbols must remain constant. Changing a topic string constitutes a breaking protocol change requiring a major version increment.
* **Type Safety**: Field types within event schemas must remain static. Changing a field type (e.g., from `i128` to `u64`) breaks downstream indexers and is prohibited.