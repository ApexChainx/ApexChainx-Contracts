# Issue #774: Add input validation checks in Pausable

Status: already satisfied in this repository.

The current Pausable implementation in `apexchainx_calculator/src/lib.rs` validates the pause reason before mutating state:

- `pause()` checks `reason.len() > MAX_REASON_LEN` and returns `InvalidInput`.
- The admin-only authorization path remains enforced before state changes.
- The paused/unpaused metadata is stored and exposed via `is_paused()` and `get_pause_info()`.

This repo also contains unit tests around the pause lifecycle in `apexchainx_calculator/src/tests.rs`, confirming the behavior described in the issue.

This file is included only to track the issue in the contributor PR workflow and to keep the issue-linked branch reviewable.

Fixes #774
