### Changed — Devanagari pins for eleven letters with native pen lifts

- **Evidence.** `tests/script-inventories/devanagari.evidence.ts` now pins
  the eleven re-segmented records (क, य, र, प, ध, ल, द, ठ, घ, ष, औ): the
  lift count; the note's movement count, native stroke count, share and
  sample count; the Commons citation; four variation phrases naming
  hpl-dvng-iso-char and the MIT-licensed LipiTk 4.0 Devanagari recognizer
  and saying the counts give no break points; one "lift" step per lift; and
  at least one "without lifting" step.
- **Regenerated curriculum outputs.** The Devanagari filmstrip-geometry
  ledger; 40 filmstrip SVGs and their figure hashes (Hindi 12, Marathi 11,
  Marwadi 8 and Sanskrit 9); the book and
  narration hashes for the Hindi and Sanskrit chapters that hold the
  changed lessons; and 20 lessons' modality records. The modality records
  were written by the package's own `generatedModalityOutputs`, only where
  the bytes changed, because the disk is too full for the CLI's staging
  copy.
- **Unchanged.** Script-owner evidence, the curriculum digest, the lesson
  count, the reinforcement counts and the filmstrip target counts do not
  move.
