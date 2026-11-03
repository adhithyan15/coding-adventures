---
category: Testing & coverage
---

# usr bin time on vitest reports the main process, not the worker that ran the corpus test

**What went wrong.** To measure `human-language-data`'s memory fixes (HL112) I
ran `/usr/bin/time -l npx vitest run <file>` before and after. Afterwards, all
four files reported almost the same maximum RSS, about 1.51 GB. That is not a
coincidence. `/usr/bin/time` reports the largest process in the tree, and here
that was vitest's main process. Its `globalSetup` reads the whole corpus to warm
the lesson cache, so it costs about 1.4 GB on every run, whatever the test file.
The fork worker that actually ran `figure-targets` had fallen from 3.8 GB to
0.45 GB, and that drop was invisible under the main process's floor.
`--logHeapUsage` did not settle it either. It samples heap *after* each test,
so it misses import-time peaks and counts garbage V8 has not yet collected.

**The fix.** Sample every descendant's RSS from `ps` while the run is going (a
0.2 s poll is enough) and report the worker's peak separately. On macOS the
worker is the child whose command line begins `node --experimental-import-…`
(vitest's fork pool). The main process is `node …/vitest`.

**What to do differently.** Before you compare a memory number, find out which
process it belongs to. If the "after" figure is the same for unrelated files,
you are looking at a floor, not a measurement.
