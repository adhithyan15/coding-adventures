## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 8 lessons in this track `drivable: true`, but
each still asked for writing in bare prose ("Write…", "Draw…", "…, then write…").
Narration reads bare prose unhedged, so the audio edition told a driver to
write (issue #12070). Each writing task is now a `[YOU WRITE: …]` cue: the
narration defers it ("[once you have stopped driving — write: …]") and the
book prints it as "*Write it:* …". The cue does not create a writing block,
so every lesson stays drivable.

- **Lessons:** MR-R09-script-a-r3, MR-R09-script-b-r3, MR-R09-script-c-r3,
  MR-R09-script-d-r3, MR-R25-recognition-script, MR-R33-ask-and-answer,
  MR-R39-there, MR-R69-dem-recall.
- The four MR-R09 wrap-ups keep "Cover the answer." and their "Compare: …
  Stop after one retrieval and one repair." as prose; only the retrieval
  itself is the cue.
- MR-R33-ask-and-answer: "Say one, then write it — and write **नमस्कार!**
  underneath" keeps "Say one." as prose and writes both lines in one cue.
- MR-R69-dem-recall: "Draw the grid from memory" becomes a write cue for the
  grid.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).
