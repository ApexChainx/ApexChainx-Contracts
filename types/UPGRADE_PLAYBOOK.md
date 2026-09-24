UPGRADE_PLAYBOOK must reconcile multiple version sources at release time
Repo Avatar
ApexChainx/ApexChainx-Contracts
Context
Version posture is spread across STORAGE_VERSION, RESULT_SCHEMA_VERSION, EVENT_VERSION, protocol version, and the co-bump tables in event_schema.rs. docs/UPGRADE_PLAYBOOK.md documents the rules.

Problem
Multiple sources with independent definitions invites a release where the doc and the code disagree (a version bumped in code but not noted in the playbook, or vice-versa). The playbook is prose; nothing machines the page against the actual constants.

Proposed approach
Generate the version table for the playbook from the same constants the tests assert (a small doc-gen step), and add a CI check that the checked-in playbook table matches the generated one.

Acceptance criteria
Version table is generated from constants.
Staleness fails CI.
Release checklist references the unified table.