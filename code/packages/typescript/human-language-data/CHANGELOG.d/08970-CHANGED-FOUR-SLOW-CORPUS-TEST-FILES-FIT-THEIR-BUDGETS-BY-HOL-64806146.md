### Changed — four slow corpus test files fit their budgets by holding less

These four files failed unrelated PRs whenever this package was rebuilt: the
`figure-targets` worker ran out of heap on macOS (#17737), and `level-gate`,
`modality` and `book-cli` crossed the 30 s / 60 s budgets. Spec HL112. No
timeout, heap ceiling or assertion moved. The fix is in what each file loads
and how often it repeats whole-corpus work.

- **Added `forEachTrackLessons(root, visit)`** (`src/loader.ts`). It is the
  `loadLessons` walk, with the same order, guards and cache, but it hands over
  one track at a time and keeps nothing. `loadLessons` is now that walk with an
  appending visitor. `tests/loader-guards.test.ts` pins the equivalence.
- **Added `summarizeLessonModalities(entries, options)`** (`src/modality.ts`).
  `summarizeModality` is now this function applied to `lessonModalities`, so
  entries derived one track at a time summarize identically. A two-track
  fixture test pins this at both table widths.
- **figure-targets.** The suite held five parsed copies of the corpus: the
  fixture loaded one, and three cases loaded their own inside `describe`. Now
  the fixture makes the suite's only pass, one track at a time. It keeps the
  target-owning lessons, every lesson id, and which bodies match the two
  strip-prose patterns, which moved to `tests/figure-targets/strip-prose.ts`.
  Worker peak RSS fell from 3,817 MB to 448 MB.
- **level-gate.** `levels`, `ramp` and `continuity` are now read off the report
  built once at import. They used to be re-measured in about a dozen test
  bodies, and the report text was rendered four times. The "absent without a
  policy" case now builds its two reports over the smallest attaining track. It
  shows that without the policy there is no gate, and that adding only the
  policy brings back the same rung. Test bodies fell from 118.6 s to 2.3 s, and
  the slowest case from 14.9 s to 1.2 s.
- **modality.** The corpus-regression block streams the corpus once. It derives
  each lesson's modality at both widths and the partition counts per track,
  where it used to derive every lesson's modality four times in test bodies.
  Worker peak fell from 1,838 MB to 764 MB, and test bodies from 6.8 s to
  0.7 s.
- **book-cli.**
  - The banner check reads 17 bytes per committed chapter, not the whole file.
  - The title/label case reuses its `describe`'s parsed config.
  - The ledger case indexes declarations once instead of calling `find` per
    chapter.

  Test bodies fell from 7.9 s to 6.3 s. What remains is reading the 12,227
  committed chapters and the chapter ledgers, which those assertions inspect.
- **Checked.** All numbers are for one file alone, warm cache, macOS arm64,
  against a clean `origin/main` worktree. An uncommitted probe found every
  streamed value equal to the whole-corpus value it replaces. Reverted
  mutations failed two figure-target cases, three modality cases and four
  level-gate cases. In the full suite on the same machine all four files
  passed. That run was still not green: 14 timeouts in other whole-corpus files
  on this branch, against 16 in the same files on `main`. Those files are out
  of scope here.
