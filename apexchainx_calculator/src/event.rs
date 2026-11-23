//! Event publication module.
//!
//! This module is reserved for event publication helpers. All event schemas
//! and topic layouts are defined in `event_schema.rs`, which is the canonical
//! source of truth for event structure.
//!
//! # Topic Layout Convention
//!
//! All events follow the 3-topic layout defined in `event_schema.rs`:
//! - topic[0] = event name (Symbol constant)
//! - topic[1] = event version ("v1")
//! - topic[2] = event-specific context (severity, caller address, etc.)
//!
//! Event publication should use direct `env.events().publish()` calls with
//! the topic tuple `(EVENT_NAME, EVENT_VERSION, context)` to ensure consistency.
