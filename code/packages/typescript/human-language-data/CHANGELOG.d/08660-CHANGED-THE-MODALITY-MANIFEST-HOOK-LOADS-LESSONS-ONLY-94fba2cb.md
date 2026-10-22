### Changed — the modality-manifest hook loads lessons only

- `tests/modality-manifest.test.ts` built its shared real-corpus fixture in a
  `beforeAll` hook with `loadEverything()`. The manifest only needs lessons, which
  are exactly `loadLessons()`'s result. `loadEverything()` also reads the taxonomy,
  registry, spine, every language curriculum (about 39,000 membership owner files,
  each one `lstat`ed and parsed), the books, the scripts and the ledgers. On the
  loaded Windows runner that extra file-system work pushed the hook past its 30 s
  budget (`Hook timed out in 30000ms` at `modality-manifest.test.ts:201`).
- The hook now calls `loadLessons()` directly, with the same lessons and the same
  manifest. No timeout was raised. Measured without the lesson cache on Linux,
  `loadLessons` takes 9.9 s against 13.9 s for `loadEverything`, and inside the
  suite the lesson cache serves the lessons, so most of what is left was the
  owner-file work this change drops. All 43 cases pass.
