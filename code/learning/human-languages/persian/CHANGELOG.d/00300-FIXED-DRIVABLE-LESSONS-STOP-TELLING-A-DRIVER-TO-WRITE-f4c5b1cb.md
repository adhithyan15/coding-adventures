## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 5 lessons in this track `drivable: true`, but
each still asked for writing in bare prose ("Write…", "Draw…", "…, then write…").
Narration reads bare prose unhedged, so the audio edition told a driver to
write (issue #12070). Each writing task is now a `[YOU WRITE: …]` cue: the
narration defers it ("[once you have stopped driving — write: …]") and the
book prints it as "*Write it:* …". The cue does not create a writing block,
so every lesson stays drivable.

- **Lessons:** FA-C16-practice, FA-C17-practice, FA-C19-practice,
  FA-C20-practice, FA-C131-amukhtan.
- Numbered Guided Practice steps ("4. Write **سه**, the one of the five…") are
  cue items; a remark that followed ("Both are made entirely of letters the
  script chapter taught.") is folded into the cue so the item stays one item.
- FA-C19-practice: step 3, "Hear ten amounts named and write each in Persian
  figures", asked for writing through an "and write" clause the detector does
  not look for. It now hears in prose and writes in a cue.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).

