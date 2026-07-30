# Large-history pruning benchmark note

## Summary

History compaction is supported, but it is not a free maintenance step. The pruning implementation in [apexchainx_calculator/src/history.rs](../apexchainx_calculator/src/history.rs) rebuilds the retained history by scanning the existing vector, so the cost grows with the amount of history currently stored. The performance-focused coverage in [apexchainx_calculator/src/pruning_perf.rs](../apexchainx_calculator/src/pruning_perf.rs) and the budget checks in [apexchainx_calculator/src/tests.rs](../apexchainx_calculator/src/tests.rs) show that large prune operations are expected to work, but they should be treated as operationally significant actions.

## Expected behavior

- `prune_history(keep_latest=N)` keeps the most recent `N` records and removes the older prefix.
- `prune_history_by_age(min_age_seconds=T)` removes entries older than the current ledger timestamp minus `T` and retains the newer records.
- Both paths are admin-only and should be no-ops when there is nothing to remove. They are also expected to remain stable for large histories, but they still perform linear work over the full stored history.

## Operational caveats

- Treat pruning as a maintenance operation that can consume a meaningful amount of execution budget.
- Schedule large prune runs during low-traffic windows or planned maintenance periods.
- Expect larger history sizes to increase runtime and resource consumption roughly in proportion to the number of entries being scanned.
- Avoid very aggressive retention targets or repeated full-history compactions back-to-back without monitoring the effect.
- Prefer conservative retention thresholds and age-based pruning when the main goal is to reclaim storage headroom without overloading the network.

## Recommended procedure

1. Estimate the current history size before pruning. Do not assume a large prune is cheap simply because the contract accepts it.
2. Run a single prune during a low-traffic window, starting with a conservative target such as keeping the latest 100-200 entries or using an age threshold that removes only clearly old entries.
3. Observe the resulting state, emitted prune events, and any operational metrics before repeating the action.
4. If history is very large, prefer a smaller number of measured compactions over one very large, unbounded prune.
5. Keep post-prune monitoring in place for storage growth, event volume, and any instruction-budget anomalies.

## References

- [apexchainx_calculator/src/history.rs](../apexchainx_calculator/src/history.rs)
- [apexchainx_calculator/src/pruning_perf.rs](../apexchainx_calculator/src/pruning_perf.rs)
- [apexchainx_calculator/src/tests.rs](../apexchainx_calculator/src/tests.rs)
