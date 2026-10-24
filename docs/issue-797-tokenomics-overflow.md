# Issue #797: Check Tokenomics for integer overflow/underflow risks

Status: blocked by repository scope mismatch.

The upstream `main` tree contains no Tokenomics contract or module. The
implemented Soroban contracts are the SLA calculator and staking contract, so
there is no Tokenomics arithmetic to audit or safely change in this checkout.
Adding speculative arithmetic checks or tests without the target implementation
would not satisfy the security acceptance criteria.

This PR records the blocker; it does not claim that Tokenomics was audited or
that the issue's implementation, test, documentation, or CI acceptance criteria
are complete. Please identify the repository or branch containing Tokenomics
before this issue can be resolved.

Refs #797