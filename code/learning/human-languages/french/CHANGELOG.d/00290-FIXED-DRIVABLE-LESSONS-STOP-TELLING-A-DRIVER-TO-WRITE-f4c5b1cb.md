## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 3 lessons in this track `drivable: true`, but
each still asked for writing in bare prose ("Write…", "Draw…", "…, then write…").
Narration reads bare prose unhedged, so the audio edition told a driver to
write (issue #12070). Each writing task is now a `[YOU WRITE: …]` cue: the
narration defers it ("[once you have stopped driving — write: …]") and the
book prints it as "*Write it:* …". The cue does not create a writing block,
so every lesson stays drivable.

- **Lessons:** FR-C14-ai-as-a, FR-C37-ne-chute, FR-C119-boeuf.
- FR-C14-ai-as-a: the recall's answer "(**ai**, **as**, **a**.)" and the
  questions after it are their own paragraph.
- FR-C37-ne-chute: "Write both. Say one." was the rule of the lesson — *ne* on
  paper, dropped in speech — not a task, so it becomes "Written, both.
  Spoken, one." (the same change #16893 made to "Write it and it falls").
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).

