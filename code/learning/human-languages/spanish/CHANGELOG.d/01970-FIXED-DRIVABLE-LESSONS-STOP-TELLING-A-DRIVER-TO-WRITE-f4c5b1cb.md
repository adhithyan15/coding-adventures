## Fixed — drivable lessons stop telling a driver to write

The modality manifest marks 5 lessons in this track `drivable: true`, but
each still asked for writing in bare prose ("Write…", "Draw…", "…, then write…").
Narration reads bare prose unhedged, so the audio edition told a driver to
write (issue #12070). Each writing task is now a `[YOU WRITE: …]` cue: the
narration defers it ("[once you have stopped driving — write: …]") and the
book prints it as "*Write it:* …". The cue does not create a writing block,
so every lesson stays drivable.

- **Lessons:** ES-C51-la-flecha, ES-C51-para, ES-C423-antipatico,
  ES-C426-mayusculas, ES-C437-sintesis-plan.
- ES-C423-antipatico and ES-C426-mayusculas: the warm-up writing tasks are
  cues; "read on to find out what you almost certainly got wrong" is its own
  paragraph.
- ES-C51-la-flecha and ES-C51-para: "Draw an arrow" was the *por*/*para* test
  done in the head, not on paper, so it becomes "Picture an arrow".
- ES-C437-sintesis-plan: "Now write the reply. Say aloud:" introduced four
  spoken `[YOU SAY: …]` cues, so it becomes "Now compose the reply. Say
  aloud:".
- The lessons leave `tests/drivable-writing-debt/` in human-language-data;
  this track has no debt left, so its ledger file is deleted.
- Regenerated: the affected book chapters, narration (`.json` and `.txt`),
  their generated book and narration hashes, and each lesson's
  `core/lesson-modality` owner (source hash only; all still `drivable: true`).

