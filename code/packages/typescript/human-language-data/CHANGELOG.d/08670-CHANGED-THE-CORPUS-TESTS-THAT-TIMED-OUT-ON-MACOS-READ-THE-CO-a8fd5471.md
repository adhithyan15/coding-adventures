### Changed — the corpus tests that timed out on macOS read the corpus once

A follow-up to 08630 and 08660. On a 3-core macOS runner under full-suite load,
`build (macos-latest)` failed with seven cases. Six were timeouts and one was a
real macOS-only bug. No timeout was raised.

- **`lesson-cache.test.ts`, "refuses an entry swapped for another file".** This
  failed on every macOS run, whatever the load. `openEntryStore` stores the
  cache directory's REAL path (`realpathSync.native`) and opens entries under
  it. On macOS `tmpdir()` is under `/var`, a symlink to `/private/var`, so the
  test's open hook compared `/private/var/…/entry` against `/var/…/entry` and
  never swapped. The hook now compares against
  `join(realpathSync.native(cacheDir), …)`. It was reproduced on Linux by
  pointing `TMPDIR` at a symlink: the old test fails exactly this case, and the
  new one passes all 72.
- **`chapters.test.ts`, "corpus snapshot".** Three cases each ran the identical
  chapter-gate report, and a fourth read the lessons again, each through
  `loadEverything()`. That is four reads of about 42,000 lessons per file, plus
  the curricula and the dataset that nobody here reads. One lazily memoised
  `committedSnapshot()` (`loadLessons` + `loadBookCorpus` + one report) now
  serves all four cases.
- **`metalanguage.test.ts`.** Two corpus cases called `loadEverything()` for
  lessons alone. They now share one `loadLessons()`.
- **`letter-ledger.test.ts`.** The block-level fixture loads exactly
  `loadLessons`, `loadLetterLedgers` and `loadScripts`. The scripts case used
  to call `loadEverything()` a second time; it now reads the fixture.
- **`plan-cli.test.ts`.** The first case planned the whole corpus twice inside
  one 120 s budget: once over the pruned copy, which the lesson cache cannot
  serve, and once over the committed corpus for the NEVER_READ_* guard. The
  guard is now its own case, "plans the pruned shared copy exactly as it plans
  the committed corpus", with the same assertion. Each case runs one plan, and
  the shared run stays memoised.
- **Measured on Linux with a warm cache.**
  - The first four files took 141.9 s of test time (63.9 s wall); they now take
    43.7 s (24.1 s wall).
  - The slowest single case fell from 22.5 s to 11.2 s.
  - The two plan cases take 29 s and 21 s. The old single case did both runs,
    about 50 s.
  - Every case passes: 161 in the first four files, 6 in `plan-cli`.
