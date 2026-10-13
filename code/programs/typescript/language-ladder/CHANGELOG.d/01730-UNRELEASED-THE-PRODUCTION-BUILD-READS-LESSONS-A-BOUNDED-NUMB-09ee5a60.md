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
