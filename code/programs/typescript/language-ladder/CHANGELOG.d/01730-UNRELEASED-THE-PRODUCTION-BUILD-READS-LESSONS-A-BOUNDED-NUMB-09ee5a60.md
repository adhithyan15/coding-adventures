## Unreleased — the production build reads lessons a bounded number at a time

- **Fixed:** `vite build` failed on Windows with `EMFILE: too many open files`
  (15,738 errors) on `build (windows-2025)`. `src/lesson-sources.ts` globs every
  lesson Markdown file with `?raw`, and Vite's `vite:asset` loader answered all
  ~29,600 of those loads with unbounded parallel `readFile` calls. That
  exhausted the Windows handle table, and it reproduces on Linux under
  `ulimit -n 1024` (21,899 errors).
- **Added:** `bounded-raw-markdown-plugin.ts`, a `pre` plugin that serves
  exactly the `?raw` Markdown imports inside the curriculum root. It emits the
  same `export default <text>` module Vite would, with an empty source map, and
  registers the same watch. Every read waits for one of 64 slots. Every other id
  falls through to Vite unchanged.
- **Verified:** the build now passes under `ulimit -n 1024`. Its `dist/` is
  byte-identical (`diff -rq`) to an unbounded build, and `check:bundle` reports
  the same 2,729 lesson batches. `tests/bounded-raw-markdown-plugin.test.ts`
  covers the limiter's cap and slot release, the id filter, and the emitted
  module. The full suite passes (485 tests).
- **Fixed:** `npx vitest run` then hit the same `EMFILE` on Windows. Three test
  files errored at `src/lessons.ts:211`, where `loadBundledLessons()` awaits
  every `?raw` lesson import at once. `vitest.config.ts` does not inherit
  `vite.config.ts`'s plugins, so the test pipeline still used Vite's unbounded
  raw loader. It now registers `boundedRawMarkdownPlugin` too. The failure
  reproduces under `ulimit -n 1024` without the plugin, and all 41 test files
  (485 tests) pass with it.
- **Changed:** `vitest.config.ts` raises `hookTimeout` from 60 s to 180 s.
  Once the corpus files loaded on Windows instead of crashing, the suite went
  from 240 s to 352 s. The `beforeAll` in `tests/curriculum.test.ts`
  (`loadCurriculumPlans()`) then went past 60 s and skipped all 7 of its tests.
  On an idle Linux box that hook takes ~9 s with or without the read limiter.
  The measurements are in the comment next to the budget. `testTimeout` stays
  at 60 s.
