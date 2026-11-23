# Prune-by-Age and Retention Limit Interaction

> **Issue:** [#648](https://github.com/ApexChainx/ApexChainx-Contracts/issues/648)

## Two pruning policies

The contract supports two independent pruning policies:

| Policy | Trigger | Implementation |
|---|---|---|
| **Age-based** | Admin calls `prune_history_by_age(min_age_seconds)` | Removes all entries with `recorded_at < now - min_age_seconds`; emits `pruned_a` |
| **Count-based (retention limit)** | Every `calculate_sla` append + admin calls `prune_history` | Trims oldest entries to keep at most `retention_limit`; emits `pruned` |

## Interaction: age prune followed by append

When `prune_history_by_age` runs, it may lower the history length well below
the retention limit. On the **next `calculate_sla` append**, the count-based
trim also fires if the new length exceeds the retention limit.

```
Before age-prune:  [e1, e2, e3, e4, e5]  len=5, retention=5
After age-prune:   [e4, e5]              len=2  (e1-e3 too old)
After next append: [e4, e5, e6]          len=3  (under retention, no trim)
```

If the retention limit is low (e.g. 2) and a burst of new events follows
the age prune, count-based trim fires — removing entries that survived the
age filter.

## Distinguishing trim causes from the event stream

- `pruned_a` event → age-based removal (explicit admin call)
- `pruned` event → count-based removal (explicit `prune_history` call)
- Automatic retention trim inside `calculate_sla` does **not** currently emit
  a separate event — the count change is visible only via `get_full_audit_state`

## Backend guidance

Backends reconstructing history should:
1. Watch for both `pruned` and `pruned_a` events to invalidate local caches.
2. Treat a decrease in `HISTORY_LEN_KEY` (from `get_full_audit_state`) between
   polls as a signal that trimming occurred, even if no explicit prune event
   is visible (automatic retention trim).