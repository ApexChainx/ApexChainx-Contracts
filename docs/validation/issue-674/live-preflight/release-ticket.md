# Release preflight evidence

Candidate: bbbb137778d5a53f0a317cda5dd85e3571d4b4da
Evidence: Local validation-results/preflight directory

- [x] candidate-clean: PASS — candidate-clean.log
- [ ] nightly-fuzz: FAIL — nightly-fuzz.log
- [x] fixture-freshness: PASS — fixture-freshness.log
- [x] parity-coverage: PASS — parity-coverage.log
- [x] ts-typecheck: PASS — ts-typecheck.log
- [x] ts-parity: PASS — ts-parity.log
- [x] offchain-eventSizeRegression: PASS — offchain-eventSizeRegression.log
- [x] offchain-readCostRegression: PASS — offchain-readCostRegression.log
- [x] offchain-governanceConsistency: PASS — offchain-governanceConsistency.log
- [x] offchain-contractMetadata: PASS — offchain-contractMetadata.log
- [x] persistence-freshness: PASS — persistence-freshness.log

Attach the release-validation-report artifact and this checklist to the release ticket.
Review nightly-run.json, nightly-artifacts.json and waivers.json; a hash alone does not approve release.
