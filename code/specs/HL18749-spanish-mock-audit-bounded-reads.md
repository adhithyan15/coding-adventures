# HL18749 — Bound Spanish mock-audit corpus reads

## Status and scope

Issue #18749 is a performance prerequisite for the full Windows
`human-language-data` suite in #12732, which in turn blocks the complete
Windows validation required by #17475. It changes no lesson, mock, answer,
credit policy, report schema, or timeout budget.

On exact main `0803b1d80d0`, TypeScript built and the direct-owner security
test passed 9/9. An unweakened single-worker package run then reported three
Spanish mock-audit `--check` cases taking about 68–71 seconds each against
their existing 60-second limits. That already-failing diagnostic was stopped;
it is not a full-suite result.

## Contract

The Spanish audit derives its taught forms only from Spanish lessons,
curriculum membership, and the common spine. Its headword conversion and
documented credits remain exactly the same at pre-A1, A1, and A2. The existing
bounded Spanish loader may supply those inputs, provided it retains the
real-directory and direct-owner link checks before reading them. An unrelated
track must not be read merely to audit Spanish; no fixture may be counted as
taught language or used as a stale cache of the real corpus.

The end-to-end `--check` cases still rebuild each level from its source bytes,
compare the canonical committed report, and fail on a changed source, malformed
key, or linked Spanish owner. Do not skip those cases or raise their budgets.

## Verification

Use the three existing failing `--check` tests as the red gate, add a focused
linked-Spanish-root regression for the audit entry point, then make the
smallest loader-path change. Verify the report bytes, TypeScript build, focused
Spanish audit and direct-owner security tests, and the full unweakened Windows
package suite on the final head. Ubuntu CI must pass independently. A focused
green result alone does not close #12732 or #17475.
