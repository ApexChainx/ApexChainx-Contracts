# ApexChainx Event Payload Compatibility & Ordering Policy

To maintain robust off-chain indexers and seamless frontend integration, all Soroban smart contracts within `ApexChainx-Contracts` must adhere to strict event schema and field ordering rules.

## 1. Field Ordering Rules (Append-Only)
* **Never reorder existing fields**: The positional sequence of payload fields within an event tuple or struct is immutable once deployed to production networks.
* **Append-only evolution**: Any additions to event payloads must be appended as new fields at the end of the structure. Removing fields is strictly prohibited; deprecate fields instead.

## 2. Backward-Compatibility Guarantees
* **Topic Stability**: Event topic symbols (defined in `policy.rs`) must remain constant. Changing a topic string constitutes a breaking protocol change requiring a major version increment.
* **Type Safety**: Field types within event schemas must remain static. Changing a field type (e.g., from `i128` to `u64`) breaks downstream indexers and is prohibited.
* **Event Version Topic**: The second topic of versioned events is reserved for the schema version symbol (e.g., `v1`). Non-breaking additive changes preserve the existing version symbol (`v1`), while breaking structural changes bump the symbol (e.g., `v2`).

## 3. Consumer Migration Guide

Off-chain indexers, backend services, and analytics adapters must process contract events in a version-aware manner to avoid service disruption when contract schemas evolve.

### 3.1 Version-Aware Parsing Strategy
1. **Extract Version Topic**: Check topic index 1 (`topics[1]`) for the version string (e.g., `"v1"`, `"v2"`).
2. **Supported Version Routing**: Route event payloads to version-specific parser handlers based on the extracted version symbol.
3. **Unknown Version Handling**: If an event contains an unrecognized or newer version symbol (e.g., `"v3"`), log a structured warning and fall back gracefully without crashing the indexing loop.
4. **Append-Only Tolerance**: Trailing fields added in minor updates must be safely ignored by older consumers that only extract the initial fields.

### 3.2 Code Example: Python Event Parser (Backend Adapter)

```python
from typing import Any, Dict, Optional

SUPPORTED_VERSIONS = {"v1"}

def parse_sla_event(event: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    topics = event.get("topics", [])
    if len(topics) < 2:
        return None

    event_name = topics[0]
    version = topics[1]

    if version not in SUPPORTED_VERSIONS:
        # Gracefully log unsupported versions without throwing an exception
        print(f"Warning: Ignored event '{event_name}' with unsupported version '{version}'")
        return None

    data = event.get("data", [])

    if version == "v1" and event_name == "sla_calc":
        # Parse canonical 7-field v1 tuple (outage_id, status, payment_type, rating, mttr, threshold, amount)
        if len(data) < 7:
            raise ValueError(f"Invalid v1 sla_calc payload length: {len(data)}")
        return {
            "version": version,
            "outage_id": data[0],
            "status": data[1],
            "payment_type": data[2],
            "rating": data[3],
            "mttr": data[4],
            "threshold": data[5],
            "amount": data[6],
        }

    return None
```

### 3.3 Code Example: Rust Event Decoder

```rust
use soroban_sdk::{symbol_short, Env, Symbol, TryIntoVal, Val};

pub const EVENT_VERSION_V1: Symbol = symbol_short!("v1");

pub fn process_event(env: &Env, topics: &[Val], data: Val) {
    if topics.len() < 2 {
        return;
    }

    let version: Result<Symbol, _> = topics[1].try_into_val(env);
    match version {
        Ok(v) if v == EVENT_VERSION_V1 => {
            // Parse v1 7-tuple
            if let Ok((outage_id, status, ptype, rating, mttr, threshold, amount)) =
                data.try_into_val::<(Symbol, Symbol, Symbol, Symbol, u32, u32, i128)>(env)
            {
                // Handle v1 event data
                let _ = (outage_id, status, ptype, rating, mttr, threshold, amount);
            }
        }
        Ok(other_v) => {
            // Log or ignore future/unknown versions without panicking
            std::eprintln!("Received unhandled event version: {:?}", other_v);
        }
        Err(_) => {}
    }
}
```