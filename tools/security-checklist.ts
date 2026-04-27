// SC-049: Contract-level security review checklist for governance and state changes
// Prints a structured checklist for PRs touching privileged or stateful contract code.

interface CheckItem {
  area: string;
  checks: string[];
}

const CHECKLIST: CheckItem[] = [
  {
    area: "Auth",
    checks: [
      "All privileged functions assert caller == admin or operator before any state write",
      "Two-step flows (propose/accept) are atomic — no partial state on failure",
      "Renounce paths clear all pending proposals in the same transaction",
    ],
  },
  {
    area: "Storage",
    checks: [
      "New storage keys are namespaced and documented",
      "No unbounded growth without a prune or retention policy",
      "Reads before writes are guarded against missing-key panics",
    ],
  },
  {
    area: "Events",
    checks: [
      "Every state-mutating function emits a corresponding event",
      "Event payloads contain enough context for off-chain replay",
      "No sensitive data (keys, secrets) in event fields",
    ],
  },
  {
    area: "Config mutation",
    checks: [
      "Config changes validate ranges before writing (no zero thresholds, no negative weights)",
      "Snapshot view reflects the post-write state deterministically",
    ],
  },
  {
    area: "Migration",
    checks: [
      "Schema changes are backward-compatible or include an explicit migration path",
      "Old storage keys are cleaned up if superseded",
    ],
  },
];

function printChecklist() {
  console.log("=== Contract Security Review Checklist ===\n");
  for (const section of CHECKLIST) {
    console.log(`[${section.area}]`);
    for (const item of section.checks) {
      console.log(`  [ ] ${item}`);
    }
    console.log();
  }
  const total = CHECKLIST.reduce((n, s) => n + s.checks.length, 0);
  console.log(`${total} items across ${CHECKLIST.length} areas.`);
}

printChecklist();
