## Fixed — six more drivable lessons stop telling a driver to write

The drivable-writing detector in human-language-data now also reads writing
verbs chained onto an earlier step ("Read **नमस्कार**, write **हो** once …")
and writing verbs after a fronted phrase ("From spoken names only, write …").
Six drivable lessons in this track used those shapes, so the audio edition
told a driver to write (issue #12070). Each is now a `[YOU WRITE: …]` cue,
as in the earlier MR-R09 fix; every lesson stays `drivable: true`.

- **Lessons:** MR-C01-practice, MR-R18-script-a-r4, MR-R18-script-b-r4,
  MR-R18-script-c-r4, MR-R18-script-d-r4, MR-R18-script-warmup.
- MR-C01-practice: "Read **नमस्कार**, write **हो** once without a model, then
  name two ways …" keeps the reading and the naming as prose; only the
  writing is the cue, and "Then name two ways …" becomes its own paragraph so
  the book does not run the cue into the next sentence.
- The four MR-R18 wrap-ups: "From spoken names only, write …" moves the
  fronted phrase inside the cue ("…, from spoken names only"), and "Compare
  once: …" becomes its own paragraph.
- MR-R18-script-warmup: "then cover the model and write it once" keeps
  "cover the model" as prose; the cue names the word (**धन्यवाद**), and the
  closing sentence about why becomes its own paragraph.
- Regenerated: book chapters 4 and 22, their narration (`.json` and
  `.txt`), generated book and narration hashes, and the six
  `core/lesson-modality` owners (source hash only).
