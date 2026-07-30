# Event Drift Review Checklist

> **Quick reference for maintainers.** Use this during code review whenever a PR
> touches `EVENT_*` constants, event emission sites, or `event_schema.rs`.

---

## 🔍 What Is Event Drift?

Event drift is a **silent compatibility break** — no compilation error, no test
failure, just corrupted downstream data. A renamed topic, reordered payload
field, or changed field type breaks every backend indexer that deserialises
contract events by position.

---

## ✅ Review Checklist

### Topic Stability (names + versions)

- [ ] **No event name changed** without bumping `EVENT_VERSION` (`"v1"` → `"v2"`)
- [ ] **No event removed** from emission without a deprecation period
- [ ] **New events use unique names** — never reuse old event name constants
- [ ] **3-topic layout preserved** — all events emit `(name, version, context)`
- [ ] **`test_event_names_are_distinct` updated** — new events added to the array

### Payload Stability (fields + types)

- [ ] **No field removed** from an existing payload tuple
- [ ] **No field reordered** — new fields always appended at the end
- [ ] **No field type changed** (e.g. `u32` → `u64`) without a version bump
- [ ] **All emission sites consistent** — search for the event name and verify
      every `publish()` call emits the same tuple shape

### Documentation

- [ ] **`event_schema.rs` doc comment updated** — payload layout documented with
      field names and types
- [ ] **`docs/EVENT_TOPIC_COMPATIBILITY.md` event catalog updated** — new or
      changed events reflected in the catalog tables
- [ ] **`docs/AUDIT_TRAIL.md` updated** — new emission sites and backend
      recovery implications documented

### Testing

- [ ] **Topic stability tests pass:** `cargo test topic_stability_tests`
- [ ] **Event names distinctness test updated**
- [ ] **Payload size assertions added** for new/modified event payloads

---

## 📋 Version Bump Decision Table

| Change | Bump `EVENT_VERSION`? |
|--------|:---:|
| New event constant added | ❌ No (additive) |
| Existing event renamed | ✅ Yes |
| New field appended to payload end | ❌ No (additive) |
| Field removed from payload | ✅ Yes |
| Field reordered in payload | ✅ Yes |
| Field type changed | ✅ Yes |
| Event emission removed | ✅ Yes (major) |

---

## 🔗 Related Policies

- [Contract Maintenance Policy: SC-505](./CONTRACT_MAINTENANCE_POLICY.md#sc-505-event-drift-review-note) — full policy text
- [Event Topic Compatibility Policy](./EVENT_TOPIC_COMPATIBILITY.md) — formal backend contract
- [Event Payload Compatibility Policy](./EVENT_COMPATIBILITY_POLICY.md) — field ordering rules
- [Contributor Safety Checklist (SC-099)](../CONTRIBUTING.md#sc-099-event-topic--payload-schema-contributor-safety-checklist) — detailed PR review checklist
