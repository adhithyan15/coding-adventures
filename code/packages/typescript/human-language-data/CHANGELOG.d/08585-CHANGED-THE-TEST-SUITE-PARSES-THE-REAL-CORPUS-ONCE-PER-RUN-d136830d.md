### Changed — the test suite parses the real corpus once per run

- New `src/lesson-cache.ts`: a content-addressed, test-only cache of each
  track's parsed lessons. An entry is named by a SHA-256 of the track's lesson
  file names and bytes, its resolved script, the package's own source files,
  the V8 version and a format version, so any lesson or parser edit is a miss
  and can never be answered from an old parse. Entries are `v8.serialize`d
  behind a magic header and a SHA-256 of the payload, written to an
  `O_EXCL | O_NOFOLLOW` 0600 temp file and `rename`d into place (atomic, so
  racing workers are safe), and read only from a private 0700 directory under
  `node_modules/.cache/human-language-data/lessons/`, only through
  `O_NOFOLLOW`, only when owned by the current user with no group/other bits,
  and only when the digest matches. Every refusal is a miss, never an error.
  Writing an entry retires the track's superseded entries.
- `loadTrackLessons` consults the cache only when one has been configured for
  the exact curriculum root being read. Nothing but the Vitest setup files
  configures it, so every CLI process (`generate:*`, `check:*`, `report`)
  parses from disk exactly as before.
- Windows uses the cache too, because the same corpus-wide tests were timing
  out on `windows-2025`. Windows has no `process.getuid` or mode bits, so only
  the owner/mode checks are skipped there. In exchange the cache is enabled
  only for the fixed `defaultLessonCacheDirectory()`, and never for a
  configured or temp path. The directory and every entry must not be a link
  or junction (`lstat`), and must resolve (`realpathSync.native`) exactly to
  where they claim to be. The opened entry must be the same file (device and
  inode) that was checked, which stands in for the missing `O_NOFOLLOW`. Entry
  names, magic and SHA-256 are checked as on POSIX. Only `ENTRY_NAME` and
  `TEMP_NAME` files are ever deleted. A rename that fails with EPERM, EACCES
  or EBUSY because another writer's identical entry is open counts as "already
  written": the temp file is removed and nothing is swept. The platform rules
  are a `CachePlatform` parameter, so tests simulate win32 on Linux: the
  directory restriction, skipped mode checks, linked directory and entry,
  digest refusal, rename collision, other rename failure, and that unrelated
  files survive a sweep. The same-file check and the entry real-path check now
  also run on POSIX.
- `vitest.config.ts` gains `globalSetup: tests/setup/warm-lesson-cache.ts`,
  which parses the real corpus once before the workers start, and
  `setupFiles: tests/setup/lesson-cache.ts`, which turns the cache on inside
  each worker. No timeout was raised.
- Measured on a 4-core machine, before → after. Real-corpus parses per
  `npx vitest run`: about 120 → 2 (one in global setup, one deliberate fresh
  parse in the cache's fidelity test); 121 corpus-equivalents are now cache
  hits. Lesson-parsing CPU: about 1,250 s → about 18 s. In a worker,
  `loadLessons()` 8.6 s → 2.2–2.8 s and `loadEverything()` 10.7 s → 4.8–5.0 s
  (the rest is curriculum/book IO, which the cache does not touch). Full-suite
  wall time: 836 s → 528 s with a cold cache (CI after `npm ci`) and 518 s warm.
  All 4,266 existing tests still pass, and the one platform skip is unchanged.
- New `tests/lesson-cache.test.ts`: entry round-trip fidelity (including the
  `NaN` chapter of an unset `chapter:`), key sensitivity, hit, miss after a
  same-size edit / add / remove / rename / `track.json` change, truncated /
  empty / bit-flipped / foreign entries, uncached parse errors, the private
  directory and file checks, symlinked directories and entries, three
  concurrent writer processes against a live reader, and a per-track
  comparison of the real corpus read through the cache against a fresh parse.
- The cache key also includes the ICU build and the default collation locale,
  because concept tags and `sourceHash` key order follow `localeCompare`. A
  parse made under one locale is never served to another.
- `required_capabilities.json` declares the test-only cache directory
  (`node_modules/.cache/human-language-data/**`: create, write, read, list,
  delete), and its summary says the CLI never enables the cache.
