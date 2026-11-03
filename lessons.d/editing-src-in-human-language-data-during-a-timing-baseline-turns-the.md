---
category: Testing & coverage
---

# Editing src in human-language-data during a timing baseline turns the lesson cache cold and spoils the numbers

**What went wrong.** I started a background baseline of four slow test files,
then edited `src/loader.ts` while it ran, and edited one of the test files too.
The lesson cache (`src/lesson-cache.ts`) keys every entry on a digest of all
of `src/`, so the runs that started after the edit found a cold cache and
re-parsed the corpus. `figure-targets` reported a 359 s import instead of
about 50 s. Separately, `modality.test.ts` collected "no tests" because the
edited file now imported functions that did not exist yet. A concurrent
`npm ci` then slowed the level-gate and book-cli baselines by about 2×. None
of those numbers described `main`.

**The fix.** Take "before" numbers in a separate worktree checked out at
`origin/main`, with its own `npm ci` and therefore its own cache under
`node_modules/.cache`. Measure one file at a time, with nothing else heavy
running. Keep editing in your own worktree.

**What to do differently.** In this package, treat a `src/` edit as a cache
flush, and treat a test-file edit during a run as a possible collection
failure. Never edit the tree that a timing run is reading.
