### Changed — Japanese chapters 138-142 pins for the A2 spine

- The Japanese integration evidence moves from 809 lessons in 137 chapters to
  837 in 142. Chapters 138-142 realize the five A2 spine nodes the track had
  not realized (what I do, negation and questions, the past, the future,
  practical texts). Every new lesson carries one activity.
- `tests/curriculum-digests/japanese.json` is re-measured for the 28 new
  lessons (809 -> 837) and the five new path segments.
- `tests/levels.test.ts`: Japanese joins the list of tracks whose `reach`
  is A2, which is now 22 tracks (every track but Spanish, which reaches B1).
  `reach` only touches A2; the level gate's `attained` stays A1.
- Nothing else moves. The Japanese spine overlays in
  `curriculum.d/spine/` are unchanged: the validator recomputes `omits` from
  the canonical concept tags, and the new lessons carry track tags
  (`JA-VERB-*`, `JA-NEG139-*` and the like), so `VERB-NEGATE`,
  `QUESTION-POLAR`, `VERB-PAST`, `VERB-FUTURE` and the `VERB-*` list of
  SPINE-SAY-WHAT-I-DO stay omitted, as they do for Telugu and Malayalam after
  their A2 spine chapters. The level gate counts a node as realized by a path
  segment with lessons, so the A2 spine criterion goes from 5 unrealized to
  0. Japanese stays attained at A1 (vocabulary 694 of 1200 and verbs 73 of
  120 remain); `touches` moves from A1 to A2.
- Letter anchoring, script closure (zero violations, zero never-taught
  glyphs), the gentle-ramp snapshot and the R1, R2, R3 and R4 misses (1, 459,
  245 and 519) are unchanged: the five chapters open 46 older reinforcement
  slots, and their warm-ups serve all 46.
