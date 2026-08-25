### Fixed — Tamil chapters 2-5 had no declared order at all

24 Tamil lessons carried no `sequence`, so four whole chapters fell back to
**alphabetical** order. Chapter 2 therefore taught the assembled phrase *en peyar*
("my name") before *peyar* existed, and asked *"what is your name?"* last,
after the practice lesson. Nothing caught it, because a chapter with no declared order
has no order to contradict. All 116 Tamil lessons now carry a globally unique sequence,
which also clears seven duplicate sequence values main was already carrying undetected
(the duplicate gate only checks schema-v2 lessons, and each pair had a v1 lesson on one
side, so the schema-v2 duplicate gate could not see them — though the continuity walk
was already reporting all seven as `duplicate-sequence` order defects, unsummarised and
unpinned). Two genuine forward reviews are fixed with it: `TA-C02-magizhcci` reviewed
`TA-C02-ungal-peyar-enna` and `TA-C04-mindum-sandippom` reviewed `TA-C04-naalai`, both
before those lessons existed. Tamil forward prerequisites and forward reviews are now
both **zero**.

Measured: `lessonsWithoutSequence` 507 → 483, `tracksWithUnorderedLessons` 19 → 18
(Tamil joins chinese, japanese and latin — the fourth of 22), `forwardPrerequisites` 240 → 230,
`forwardReviews` 285 → 273, `forwardReferences` 469 → 468, and `chapterViolations`
25 → 24 — that last being the gate that measures how much a chapter throws at a reader
at once, which Tamil chapter 1 had been failing at 24 atoms against a budget of 12.

Three numbers moved the wrong way, all deliberate:

- `missedByWindow.R1` 834 → 843. Consecutive script lessons now sit 4-5 apart and R1 is
  a 1-3 window, so no script lesson can reinforce the previous one inside it. 24 atoms
  miss R1 while their real revisit counts run 0 to 7, median 2 — reinforced, just never
  within three lessons. An interleaved strand cannot satisfy R1 as defined.
- `fullyDrivableChapters` 327 → 322, which is −6 +1 rather than a flat loss. Six
  chapters that were entirely ear-only now hold a writing lesson (6, 10, 13, 16, 18, 19;
  chapter 6 holds two), offset by chapter 1 becoming fully drivable for the first time.
- Script-ramp `lessonViolations` 60 → 61, a net of three moves: `TA-W03-pulli-vanakkam`
  (9 glyphs, the steepest in the corpus) stopped counting once it moved to chapter 7,
  while `TA-C01-practice` (5) and `TA-C01-nandri` (4) joined — both **speaking** lessons
  showing glyphs the writing strand has not reached. The gate counts a glyph the first
  time it appears, which is the right rule for a track that teaches script alongside
  speech and the wrong one for a track that shows before it teaches.

