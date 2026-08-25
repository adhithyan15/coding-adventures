### Added — a track must EARN a level, not touch one (HL09 §3.1)

- Add `src/level-gate.ts`. The gap report now publishes two numbers per track where
  it published one:

      levels: 650 pre-A1, 297 A1, 186 A2; 148 unmapped (88% placed)
      levels ATTAINED (HL09 §3.1): none; 22 tracks touch a level they have not attained

- **This is the gate that would have caught "Spanish reaches A2".** Nothing lied:
  `TrackLevelCoverage.reach` is documented as *the highest level this track has any
  lesson at*, and that was true. The mistake was letting a number that means
  **touches** be read as **means**. One lesson pointing at one A2 node moves `reach`;
  it is nowhere near enough to sit the exam.
- `touches` keeps the old meaning. `attained` is the highest level where all four
  §3.1 criteria hold at that level **and every level below**: every spine node
  realized, cumulative vocabulary met, no lesson over the atom budget, every atom
  revisited twice. **Zero of 22 tracks have attained even pre-A1.**
- Spanish is *in progress at pre-A1*: **44 distinct headwords at or below pre-A1
  against a 300 target** (shortfall 256), plus 92 atoms revisited fewer than twice.
- **Every criterion is scoped "at or below the level", and getting that wrong was the
  first version of this module committing the exact error it exists to catch.** The
  initial implementation measured whole-track vocabulary (Spanish 138) against a
  per-level cumulative target, and applied the atom-budget and reinforcement criteria
  track-wide — so Hindi's single over-budget lesson, which sits *above* pre-A1, blocked
  pre-A1, making criterion 3 unfalsifiable at the bottom of the ladder. Security review
  caught it; the honest pre-A1 vocabulary is **44**, not 138.
- Criterion 4 counts atoms revisited **fewer than twice**, per §3.1 — not "never
  revisited". The looser reading hid 51 of Spanish's 141 failures.
- Vocabulary counts only `CONTENT_TYPES` lessons. Counting every lesson type credited
  drill titles and grammar labels as vocabulary — `(practice)`, `qu-`, `fact or wish?` —
  25 of Spanish's 138.
- A level with **no authored spine nodes fails** criterion 1 rather than passing it
  vacuously. `spine.json` has zero B1-C2 nodes, and "no node is unrealized" is not
  "every node is realized" — the same touches-vs-means error, one level up.
- **Failures name the criterion and the shortfall**, not a bare `false` — a boolean
  would move the argument rather than settle it. `vocabulary: teaches 138 distinct
  headwords against 300 for pre-A1, shortfall 162`.
- The gate stops at the **first** failing level, because the criteria are cumulative:
  a level above a failing one is unreachable by definition.
- Vocabulary targets live in `LEVEL_VOCABULARY` and are **editorial** per §10 —
  conventional working figures for CEFR receptive vocabulary, not a claim about any
  awarding body's syllabus. They are named so a failure can cite what it was measured
  against.
- Absent, not empty, when the caller supplies no policy: *not measured* and *attained
  nothing* are opposite facts, and a test pins that distinction.

