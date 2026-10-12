### Added — every word lesson's etymology is measured, and the gap may only fall

New module `src/etymology-coverage.ts` (exported from `src/index.ts`) measures
HL24 §9: every lesson that introduces a word says where that word comes from,
from a cited source, with "unknown" or "disputed" as allowed answers and an
invented etymology never one.

- **Word-introducing lesson.** Introduces a `<XX>-LEX-` atom (frontmatter
  `introduces.knowledge` or any block's `hl-knowledge`), or has `type: word` /
  `phrase`. The type test keeps the 632 legacy word lessons whose atoms are
  not named `-LEX-`; the anchored atom test keeps RETROFLEX, REFLEXIVE and
  CIRCUMFLEX atoms out. A review lesson that introduces nothing is not counted.
- **Two counts, because the hook is not printed.** `etymology_hook` is read
  only by the app; `book.ts` and `narration.ts` never render it. So the
  measure reports word lessons without a hook (empty or whitespace) and word
  lessons whose body shows no origin statement: no "taken apart" block, no
  origin heading, and no `←`, "from/via/through <language>", `Proto-`/`PIE`,
  "cognate with"/"same word as <language>", or verbless donor ancestor
  ("Latin *october*"). The visible rule is conservative. Its false positives
  and false negatives, measured on the corpus, are written in the module and
  pinned by unit tests. Every pattern uses bounded repetition, and HTML
  comments are removed with an indexOf walk, so no input can cause
  catastrophic backtracking.
- **Report.** The gap report gains `etymologyCoverage` (per track and summed,
  without the per-lesson gap list), the summary fields `wordLessons`,
  `wordLessonsWithoutEtymologyHook` and `wordLessonsWithoutVisibleEtymology`,
  an `etymology:` summary line, and a per-track `Etymology coverage (HL24 §9)`
  section.
- **Ratchet.** `ETYMOLOGY_GAP_PINS` in `tests/root-ledger.test.ts` pins both
  counts for all 23 tracks exactly, in the test file that already loads the
  corpus. A rise fails and names the 40 newest offending lessons in reading
  order. A fall fails and asks for the pin to be lowered in the same change.
- **First measurement.** 25,682 word-introducing lessons: 22,744 without a
  hook, 22,378 without visible etymology, 21,697 with neither. Spanish
  486 / 209; Chinese 0 / 62; every other track over 500 on both.

`HL24` gains §9 (rule, measure, ratchet) and a row in its exact-pins table.
`HL00`'s etymology section points to it. No lesson changed.
