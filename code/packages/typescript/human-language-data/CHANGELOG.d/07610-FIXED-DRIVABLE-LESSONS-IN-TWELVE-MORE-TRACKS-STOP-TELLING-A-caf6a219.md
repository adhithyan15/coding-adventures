### Fixed — drivable lessons in twelve more tracks stop telling a driver to write

- Issue #12070, second pass. `tests/drivable-writing-cues.test.ts` recorded
  402 drivable lessons that still asked for writing in bare prose. This pays
  off the 63 in the twelve smallest ledgers: hindi 15, urdu 11, marathi 8,
  persian 5, russian 5, spanish 5, bengali 3, french 3, punjabi 3,
  portuguese 2, telugu 2, malayalam 1. Their ledger files in
  `tests/drivable-writing-debt/` are deleted, since the test says an absent
  file is how a track has no debt. Chinese, Japanese and Marwadi keep theirs.
- Each writing task is now a `[YOU WRITE: …]` cue, the same form #16893 used
  for Chinese. Spoken halves stay prose ("Say *desh*. [YOU WRITE: **देश** from
  memory]"), and prose that followed the old instruction is its own paragraph.
  In numbered steps, the remark is folded into the cue so the step stays one
  item.
- Every cue sits on one source line. The first draft wrapped 28 cues, which
  the old line-at-a-time `book.ts` printed as a literal `{[}YOU WRITE: …{]}`.
  Since the shared cue grammar (07600) a wrapped cue renders correctly and
  `check:books` rejects any raw cue, so regenerating on top of it changed no
  chapter.
- Six flagged sentences were not writing tasks, so they were reworded
  instead of cued: HI-C86-ve ("in writing, use the forms above"), FR-C37-ne-chute
  ("Written, both. Spoken, one."), ES-C51-la-flecha and ES-C51-para ("Picture
  an arrow", since the *por*/*para* test happens in the head), ES-C437-sintesis-plan
  ("Now compose the reply. Say aloud:", which leads into spoken cues), and
  TE-R152-jhari-recall's answer key ("the circle goes round **ఝ**").
- Five more writing tasks in the same lessons used an ", and write" clause the
  detector does not look for (FA-C19-practice, UR-C30-practice,
  UR-C31-practice, RU-R26-close, TE-R152-jhari-recall). They are cued as well.
- Regenerated: book chapters, narration, book and narration hashes, and the
  63 `core/lesson-modality` owners (source hash only; every lesson is still
  `drivable: true`).
