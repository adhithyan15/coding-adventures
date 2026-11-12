# HL123 — Keep plan-CLI inventory refusals inside their test budget

## Status

Implementation child of #18852. The exact-source macOS arm64 full-corpus run
43101467065 passed 4,532 tests and timed out only in the 120-second
`plan-cli` probe-key refusal case. The failure was under whole-suite load, not
an assertion mismatch; the existing test still reads every lesson in a
pruned full-corpus copy just to verify an invalid inventory is refused.

## Contract

The refusal test must continue to delete every probe key in a real committed
French A1 inventory and assert the planner's nonzero exit and named diagnostic.
It may use the existing inventory-only fixture if that fixture preserves the
real registry, inventory, assessment-policy, and other inputs needed to reach
the same refusal. Do not mock the inventory loader, skip validation, select
fewer corpus-wide test files, or raise the test, hook, heap, or worker budgets.

The separate clean-corpus and pruned-vs-committed equivalence cases retain the
full lesson-coverage path. The mutation case should not pay for that path a
third time when it is asserting a schema-boundary failure that precedes
meaningful coverage measurement.

## Acceptance

- The existing probe-key refusal assertions pass on the inventory-only fixture.
- The full `plan-cli` test file and the package's unchanged complete coverage
  suite pass locally where available.
- A final exact-source macOS arm64 full-corpus run reaches a terminal green
  verdict before #18852 or #18947 is closed.
