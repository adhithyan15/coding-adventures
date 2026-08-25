### Added - Spanish proves both A1 writing stages, and its atom budget is clean

- Three new Spanish chapters (337-339) carry the two writing stages A1 requires
  on top of the pre-A1 four: `controlled-composition` twice, then
  `timed-assessment-production` against the timing, word count and criteria
  already sourced in `spanish/task-shapes/a1.json`. All three hang on
  `SPINE-TIME-OF-DAY`, an A1 spine node, so the evidence lands on the rung the
  blocker names. Spanish is the first track in the corpus to evidence any A1
  writing stage.
- The six Spanish lessons that introduced four atoms against the
  `maxNewAtomsPerLesson: 3` budget are split into twelve, each along a seam
  already present in the file — every one had two atom-introducing body blocks.
  No atom was removed to make a number fall; the atom totals are identical
  before and after. Each split lesson stays in its sibling's chapter, so no
  chapter was renumbered and no chapter's introduced-atom set moved.
- `plan-cli --ceiling C2` no longer lists `writing-stage` or `atom-budget` for
  spanish. Its `reinforcement` blocker fell from 88 to 81 as a side effect —
  the new lessons retrieve their siblings' atoms — and pre-A1 attainment is
  unchanged.

