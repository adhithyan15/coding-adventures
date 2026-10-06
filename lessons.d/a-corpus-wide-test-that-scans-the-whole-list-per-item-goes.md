---
category: Testing & coverage
---

# A corpus-wide test that scans the whole list per item goes quadratic and times out only in CI

**What went wrong.** `human-language-data`'s `tests/modality-manifest.test.ts`
"keeps every rollup internally consistent" checked each chapter's drivable
lesson ids with `manifest.lessons.find((entry) => entry.id === id)`. That is a
full scan of roughly 1,400 entries for every id, so the case grew with the
square of the corpus. Run alone locally it took about 11.5 s. Under full-suite
parallel load in CI it crossed the shared 30 s budget and failed a Bengali PR
whose own diff never touched modality.

**The fix.** Index once with `new Map(manifest.lessons.map((e) => [e.id, e]))`
and look up with `byId.get(id)`. The assertions stay the same and the case runs
in about 0.7 s. The 30 s budget was not raised: the vitest config says a test
that needs more is a signal about the test.

**What to do differently.** In tests that walk the whole curriculum, never
call `find`, `filter` or `includes` over the corpus inside a loop over the
corpus. Build a `Map` or `Set` first. When a corpus test fails with "Test timed
out in 30000ms", run that file alone with `--reporter=verbose`. If one case
takes more than a few seconds, look for a nested scan before reaching for a
timeout.
