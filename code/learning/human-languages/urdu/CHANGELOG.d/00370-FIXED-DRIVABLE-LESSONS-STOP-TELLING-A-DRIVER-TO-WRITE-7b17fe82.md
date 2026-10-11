## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 11 lessons in this track `drivable: true`, but
each still asked for writing in bare prose ("Write…", "Draw…", "…, then write…").
Narration reads bare prose unhedged, so the audio edition told a driver to
write (issue #12070). Each writing task is now a `[YOU WRITE: …]` cue: the
narration defers it ("[once you have stopped driving — write: …]") and the
book prints it as "*Write it:* …". The cue does not create a writing block,
so every lesson stays drivable.

- **Lessons:** UR-C17-kahan, UR-C17-maan, UR-C18-aam, UR-C18-purana,
  UR-C18-voh, UR-C28-practice, UR-C29-practice, UR-C30-practice,
  UR-C31-practice, UR-R32-close, UR-C141-lautna.
- Warm-ups such as "Write یہ and say it" become "[YOU WRITE: یہ — and say it]",
  the form the Chinese track already uses ("[YOU WRITE: 日 — and say both of its
  meanings]"). Questions and remarks that followed are their own paragraph.
- Dictation steps in the number chapters ("4. Write all five from
  dictation. Every letter…") fold their remark into the cue, so a numbered
  item stays one item.
- UR-C30-practice and UR-C31-practice: "Hear … amounts named and write each
  in … figures" also asked a driver to write, through an "and write" clause
  the detector does not look for. Both now hear in prose and write in a cue.
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).
