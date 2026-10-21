## Unreleased — the corpus loader opens at most 64 lessons at a time

- **Fix:** `loadBundledLessons()` used to start all of its lazy `?raw` lesson
  loaders at once with a bare `Promise.all`. The corpus now has about 39,000
  lessons, and on the Windows CI runner that asked for more open files than
  the runner allows (`EMFILE: too many open files`). The Lessons, Concepts and
  lesson-body test files then failed while importing their shared fixture,
  `tests/real-lessons.ts`. Loads now go through the new
  `mapWithConcurrency(items, limit, fn)`, which keeps at most
  `LESSON_LOAD_CONCURRENCY` (64) loaders in flight and returns results in input
  order, just as `Promise.all` does. The browser build behaves the same; only
  the number of requests in flight at once changes.
- **Reproduced on Linux** by lowering the open-file limit (`ulimit -n 1024`).
  The old loader fails `tests/lessons.test.ts` with the same EMFILE error, and
  with the throttle the three affected test files pass.
- **Tests:** a new `tests/load-concurrency.test.ts` checks:
  - results come back in input order;
  - no more than `limit` loaders are ever in flight;
  - a slot frees as soon as any call settles (a throttle, not batches);
  - the first rejection is propagated;
  - empty input and an oversized limit both work;
  - limits that make no sense are refused.
