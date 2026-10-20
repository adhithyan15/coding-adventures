---
category: Testing & coverage
---

# Run one full human-language-data suite at a time

Two full `human-language-data` Vitest runs were started while an earlier run
was still active. Both competed to parse and measure the same large corpus.
Tests that normally fit under their explicit budgets then crossed 30 seconds
and failed in unrelated level, chapter, and mock-coverage files.

Before starting the full suite, check that no earlier `vitest run` for this
package is still active. Run focused and integration suites in parallel only
when no whole-package run is underway; run the final full suite alone. If an
orchestration session stops yielding output, inspect the process instead of
assuming the command finished and launching a duplicate.
