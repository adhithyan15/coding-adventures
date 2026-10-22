---
category: Cross-platform & Windows BUILD_windows
---

# A loader that starts one promise per corpus file runs out of file handles on Windows once the corpus is large

`language-ladder`'s `loadBundledLessons()` ran `Promise.all` over one lazy
`?raw` import per lesson. While the corpus was a few thousand lessons that
was fine everywhere. At about 39,000 lessons the Windows CI runner failed
three test files at import with `EMFILE: too many open files`. Linux and
macOS have much higher descriptor limits, so the bug only showed on the one
PR that runs the Windows matrix (adhithyan15/coding-adventures#17522).

The fix was a small `mapWithConcurrency(items, limit, fn)`, a worker-pool
throttle that keeps input order, with the limit set to 64. It is reproducible
on Linux: `ulimit -n 1024` before `npx vitest run tests/lessons.test.ts`
makes the old code fail the same way.

What to do differently: any loop that starts one async operation per corpus
file (lessons, owners, ledgers) needs a bound from the start. Reproduce
Windows-only descriptor failures locally by lowering `ulimit -n` instead of
waiting for the Windows runner.
