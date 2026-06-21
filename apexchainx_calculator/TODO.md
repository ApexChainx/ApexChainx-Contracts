# apexchainx_calculator - Event schema canonicalization

- [ ] Update `src/lib.rs` to remove local event constants and re-export canonical ones from `src/event_schema.rs`.
- [ ] Ensure all contract event publishes compile against the re-exported constants.
- [ ] Add regression test in `src/tests.rs` to assert symbol equality between `lib.rs` and `event_schema.rs` for:
  - EVENT_VERSION
  - all 15 event name constants.
- [ ] Run `cargo test`.
- [ ] Confirm the regression test fails if constants drift (sanity check by reasoning; no manual drift commit).

