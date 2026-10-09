## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 2 lessons in this track `drivable: true`, but
each still asked for writing in bare prose ("Write…", "Draw…", "…, then write…").
Narration reads bare prose unhedged, so the audio edition told a driver to
write (issue #12070). Each writing task is now a `[YOU WRITE: …]` cue: the
narration defers it ("[once you have stopped driving — write: …]") and the
book prints it as "*Write it:* …". The cue does not create a writing block,
so every lesson stays drivable.

- **Lessons:** TE-R151-chhaya-recall, TE-R152-jhari-recall.
- TE-R152-jhari-recall: "Circle **ఝ**." and the wrap-up's "then circle its
  first base letter" are write cues ("a circle round **ఝ**"). The answer key
  "(**ఝరి**; circle **ఝ**.)" becomes "(**ఝరి**; the circle goes round **ఝ**.)",
  so the answer is not heard as another instruction. The Guided Practice
  line "…use the picture as your cue and write the whole word from memory"
  is also a cue now.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).
