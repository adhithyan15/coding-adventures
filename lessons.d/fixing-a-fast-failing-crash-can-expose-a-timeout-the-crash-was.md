---
category: TypeScript / JavaScript
---

# Fixing a fast-failing crash can expose a timeout the crash was hiding

**What happened:** on PR #17185, `language-ladder`'s vitest run on
`build (windows-2025)` failed with `EMFILE` in three test files. Those files
load the whole ~29,600-file lesson corpus. Bounding the raw lesson reads
removed that error. The next Windows run then failed in a fourth file:
`tests/curriculum.test.ts` had `Hook timed out in 60000ms`, which skipped all 7
of its tests.

The crash had been doing work-avoidance. While the three corpus files failed
fast, the curriculum hook finished in 18 s. Once they loaded the corpus, they
shared the one Vite server with the hook while the whole-repo build ran
alongside. The suite went from 240 s to 352 s, and the hook went past 60 s.
On an idle Linux box the hook takes ~9 s with or without the read limiter, so
the limiter did not make it slower.

**Fix:** raise `hookTimeout` in `vitest.config.ts` from 60 s to 180 s, and add
the Windows measurements to the table that justifies the budget.

**Do differently:** when a fix turns fast failures into real work, the time
budgets of everything sharing that worker change too. Compare the suite's total
duration before and after on the slowest lane, not just the files you fixed. A
timeout that appears right after a crash disappears is the same run finally
doing all its work, not a new bug.
