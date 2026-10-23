### Changed — test workers get a stated 4 GB heap, so macOS stops running out

A follow-up to 08670. `build (macos-latest)` still failed on #17522 and #17972,
and the cause was memory, not time. One Vitest worker died with `FATAL ERROR:
Reached heap limit Allocation failed - JavaScript heap out of memory` at about
2,026 MB. Vitest reported `Worker exited unexpectedly`, and the cases queued
behind it (`track-progress.test.ts`, `corpus/persian.test.ts`) then timed out.

- **Why only macOS.** Node sizes its default old-space limit from the
  machine's RAM: about 4 GB on a 16 GB Linux runner, about 2 GB on the 7 GB
  `macos-latest` runner. A corpus-wide worker over ~42,000 lessons now needs
  more than 2 GB, so Linux had headroom and macOS did not.
- **Fix.** `vitest.config.ts` passes `execArgv: ["--max-old-space-size=4096"]`
  to every test worker, so each platform runs the suite under the same stated
  ceiling instead of whatever its RAM implies. No timeout was raised.
- **Checked.** A probe test read `v8.getHeapStatistics().heap_size_limit` inside
  a worker. With the option set to 3000 it reported 3,048 MB, so the flag does
  reach the workers. `lesson-cache.test.ts` (72 cases) passes with the new
  config.
