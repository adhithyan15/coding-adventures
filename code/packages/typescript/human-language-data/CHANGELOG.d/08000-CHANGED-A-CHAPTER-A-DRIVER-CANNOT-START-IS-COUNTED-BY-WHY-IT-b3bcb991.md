### Changed — a chapter a driver cannot start is counted by why: its first lesson's type or its content

- **The question.** The gap report's `chaptersWithoutDrivablePrefix` counts
  chapters whose first lesson a driver cannot do, so the narration opens them
  with "The first lesson already needs your eyes or your hands, so save this
  one for when you have stopped." It read 306. The first thing
  checked was whether the metric was simply stricter than the narration's own
  announcement, which counts a lesson whose core is drivable even when a
  writing or letters section waits behind the stop guard. **It is not.** The
  report's prefix (`drivablePrefix` in `modality.ts`), the narration header
  (`narrateChapter`) and the book's "Hands-free start" line all stop at the
  first lesson whose **core** is not `voice`, so no metric alignment was
  needed. (The committed modality manifest's chapter rows are the one
  exception: `ModalityManifestChapter.drivablePrefix`, `drivableLessonIds`
  and `summary.unstartableChapters` still walk the full `modality`. That is
  the manifest's documented conservative filter, and changing it would break
  the compatibility promise its header makes, so it is unchanged here.)
- **What the 306 were.** 183 chapters open on a `type: writing` (161) or
  `type: reading` (22) lesson — a lesson whose whole point is the hand or the
  eye, with nothing separable to set aside. HL08 forbids thinning the book to
  make those drivable, so they are honest zeros and the number could never
  reach 0. The other 123 open on some other lesson whose core still needed
  eyes or hands: a table the lineariser refuses (20, some alongside a
  pointing phrase), a step said plainly (20, some alongside a pointing
  phrase), or a pointing phrase alone (83).
- **The split.** `ChapterModality` gains `unstartable`, which is
  `"lesson-type"`, `"lesson-content"` or null, from the new exported
  `unstartableCause(ordered)`. It reads the first lesson's core reasons:
  `writing-type` or `reading-type` means type, anything else means content.
  The gap report adds `chaptersWithoutDrivablePrefixByType` and
  `chaptersWithoutDrivablePrefixByContent` beside the total. The text view
  prints both counts and lists by-content chapters first, with "— by type" or
  "— by content" after each.
- **Order is rarely a lever.** In 293 of the 306, every later drivable lesson
  in the chapter depends, by `prerequisites` or by the knowledge it requires
  or practises, on a non-drivable lesson before it. In the other 13 (Arabic 3
  and 45, Gujarati 2, Hindi 21, Latin 48, Malayalam 4, Punjabi 18, Tamil 8,
  Telugu 3, 5 and 6, Urdu 7 and 8), the opener is mostly a letter or numeral
  lesson that teaches a glyph the following words print, so swapping them
  would break script closure. None was reordered.
- **Tests (`tests/drivable-prefix.test.ts`).** Every row of the truth table
  is tested on synthetic lessons: voice, empty, writing, reading, a pointing
  cue, a hand step, and authored order versus array order. Two remedies are
  shown to clear the content cause: deferring a step as `[YOU LOOK: …]`, and
  moving a writing step into its own `## Writing` section, which keeps
  `modality: pen` for the book and gives `coreModality: voice`. A test shows
  that no rewording clears the type cause. The report's split is checked as a
  partition. On the real corpus, every unstartable chapter has exactly one
  cause, and the by-content count is held under a **ceiling of 80 that may
  only fall** (it was 123 when the split was introduced). The by-type count
  has no ceiling, on purpose: HL08 says honest writing instruction may raise
  the pen count and "no gate here may push back on it". The non-vacuity
  checks use fixtures and never read debt off the corpus
  (`lessons.d/a-test-that-proves-a-measure-is-not-vacuous-by-reading-debt.md`).
