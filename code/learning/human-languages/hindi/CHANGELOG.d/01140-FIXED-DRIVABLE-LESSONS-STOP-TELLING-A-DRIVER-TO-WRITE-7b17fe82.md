## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 15 lessons in this track `drivable: true`, but
each still asked for writing in bare prose ("Write…", "Draw…", "…, then write…").
Narration reads bare prose unhedged, so the audio edition told a driver to
write (issue #12070). Each writing task is now a `[YOU WRITE: …]` cue: the
narration defers it ("[once you have stopped driving — write: …]") and the
book prints it as "*Write it:* …". The cue does not create a writing block,
so every lesson stays drivable.

- **Lessons:** HI-C68-india, HI-C68-language, HI-C69-railway, HI-C69-ticket,
  HI-C70-play, HI-C71-also, HI-C72-expensive, HI-C72-rupee, HI-C73-breakfast,
  HI-C73-waiter, HI-C74-closed, HI-C74-exit, HI-C81-skool, HI-C86-ve,
  HI-C93-ke-saath.
- The chapter 68–74 warm-ups ("Say *desh*, then write **देश** from memory")
  keep the spoken half as prose and move the writing half, with its stroke
  order, into the cue. Explanations that followed ("Three letters and one
  matra…") are their own paragraph.
- HI-C81-skool: "Write it: स + ् + क…" becomes a cue to write **स्कूल**, and the
  spelling walk-through (with its "…no: the ू belongs to क" self-correction)
  is kept as the next paragraph.
- HI-C86-ve: "Recognise it; write the forms above." was register advice, not
  a task, so it becomes "Recognise it; in writing, use the forms above."
- HI-C93-ke-saath: "Write the shape down once, because…" becomes a cue for the
  shape, followed by "The rest of this chapter is the same shape six more
  times:" and the table.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).
