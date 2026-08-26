## HL-C11U — every assessment contract promises files that do not exist

`<track>/assessment.json` names, per level, the task-shape inventory each of the
four skills is measured against and the two timed mocks, their rubric and their
answer key that constitute HL16's pass evidence. Every one of those is a path.
Nothing checked that the paths led anywhere.

Measured 2026-08-26 across all 23 registered tracks:

| | |
|---|---|
| tracks with an assessment contract | **13** |
| tracks whose contract dangles | **13** — all of them |
| dangling reference occurrences | **674 of 742** |
| distinct artifacts promised and absent | **351** |
| — mock papers, rubrics, answer keys | 276 |
| — task-shape inventories | 75 |
| `mocks/` directories in the repository | **0** |

Per track the shape is uniform: 21 unbuilt mock artifacts each (seven rubrics,
fourteen answer keys), plus five to seven missing task-shape inventories.
Persian owes 31, having declared an external capstone as well. Spanish's contract
names `mocks/a1/rubric.md`, `mocks/a1/mock-1-answer-key.md` and 24 more, and
Spanish is the track closest to printing a level line.

This is HL20 §1's flattering failure one layer down. A level that names two timed
mocks with a rubric and an answer key reads as *stronger* evidence than a level
that names none, so the dangling contract is worse than an empty one.

**Now gated, as a ceiling.** `core/assessment-artifact-ceiling/<track>.json`
pins the 351, one shard per track so thirteen independent authors do not collide.
A dangling reference outside the pin fails CI in any track; a pinned path that
gets built also fails, so the pin must be lowered rather than left to rot. The
pin is a set, not a count — a count is satisfied by paying one debt and taking
another.

**The debt is unchanged by having been written down.** The 351 artifacts are the
work. The natural order is the one HL15's completion plan already implies:
task-shape inventories first (finite research, one file per track-level), then
mocks at the levels a track is actually near. Spanish A1 is being authored now.

**Related, and deliberately not fixed here:** HL09 §3.1's five criteria all
measure the corpus and none measures a learner, so `mock-performance` (HL09 §3.2)
remains a sketch blocked on the first scored A1 sitting. The report's wording was
corrected in the same change — "levels ATTAINED" now reads "levels with
STRUCTURAL COVERAGE COMPLETE … performance UNVERIFIED" — but a label is not a
measurement, and the third row of that table is still empty.
