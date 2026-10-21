Contract API Archetypes
Every public entrypoint in apexchainx_calculator belongs to one of three archetypes. Knowing which category a method falls into tells you immediately whether a call can be made freely, requires operator credentials, or is gated behind admin authority.

Read-only (no auth required, no state written)
These functions can be called by anyone at any time — including while the contract is paused — and never modify on-chain state or emit events.

Group	Methods
Healthcheck & version	healthcheck, get_version_info, get_migration_state
Pause / freeze status	is_paused, get_pause_info, is_config_frozen
Config views	get_config, get_config_snapshot, get_config_version_hash, list_configs, get_last_config_update, get_config_bundle
Custom severity views	get_custom_severity, get_custom_config_snapshot
Stats & telemetry	get_stats, get_economic_exposure, get_severity_telemetry
History views	get_history, get_history_page, get_history_page_with_meta, get_history_by_outage, get_latest_by_outage
get_history_page follows the documented History Pagination Policy (issue #263): offset-based, oldest-first, empty-page end-of-history signalling, and saturating offset + limit arithmetic so extreme u32 values can never overflow into a wrong slice. get_history_page_with_meta returns the same page plus total and has_more metadata so consumers can page without a separate get_history call. | Role queries | get_admin, get_operator, get_pending_admin, get_pending_operator | | Introspection | get_result_schema, get_failure_schema, get_contract_metadata, get_full_audit_state | | Retention helpers | get_retention_limit, get_config_count, get_storage_version | | View-mode calculation | calculate_sla_view, replay_calculate_sla |

Mutating — operator role
These functions write state and emit events. Only the current operator address may call them.

Method	What it writes
calculate_sla	Appends a result to history, updates cumulative stats and per-severity telemetry, emits sla_calc and set_int events. Idempotent on exact replay; rejects conflicting duplicates with a dup_input event carrying the stored result.
Privileged — admin role
These functions can only be called by the current admin address. They control the contract's lifecycle, configuration, and role management. Several of them are irreversible (e.g., renounce_admin) or have broad blast radius (e.g., prune_history), so treat them accordingly.

Group	Methods
Lifecycle	initialize, migrate
Config management	set_config, set_custom_severity, remove_custom_severity, freeze_config, unfreeze_config
Operational controls	pause, unpause, set_retention_limit
Admin transfer	propose_admin, accept_admin, cancel_admin_proposal, renounce_admin
Operator transfer	set_operator (legacy direct), propose_operator, accept_operator, cancel_operator_proposal
History pruning	prune_history, prune_history_by_age
Quick rule of thumb for contributors: if the method name starts with get_, is_, list_, healthcheck, calculate_sla_view, or replay_calculate_sla it is read-only and safe to call freely. Everything else either requires the operator role (calculate_sla) or the admin role (everything remaining).
