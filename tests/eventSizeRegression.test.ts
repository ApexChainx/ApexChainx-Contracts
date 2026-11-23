import assert from "node:assert/strict";
import { test } from "node:test";
import { estimateScValSize, runChecks } from "../offchain/eventSizeRegression";

test("event payload budgets match actual contract XDR fixtures", () => runChecks());
test("XDR estimator counts discriminants, padding and full-length symbols", () => {
  assert.equal(estimateScValSize([]), 12);
  for (const length of [0, 1, 4, 5, 9, 32]) {
    assert.equal(estimateScValSize([{ kind: "symbol", value: "A".repeat(length) }]),
      20 + Math.ceil(length / 4) * 4);
  }
  assert.throws(() => estimateScValSize([{ kind: "symbol", value: "A".repeat(33) }]));
  assert.equal(estimateScValSize([{ kind: "u64" }]), 24);
  assert.equal(estimateScValSize([{ kind: "i128" }]), 32);
  assert.equal(estimateScValSize([{ kind: "address" }]), 56);
});
