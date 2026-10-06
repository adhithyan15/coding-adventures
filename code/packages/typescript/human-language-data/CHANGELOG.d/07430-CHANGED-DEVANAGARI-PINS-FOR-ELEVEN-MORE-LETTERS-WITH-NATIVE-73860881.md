### Changed — Devanagari pins for eleven more letters with native pen lifts

- **Evidence.** `tests/script-inventories/devanagari.evidence.ts` now also
  pins अ, आ, ओ, झ, स, ब, च, थ, भ, म and व: the lift count; the note's
  movement count, native stroke count, share and sample count; the Commons
  citation (JackPotte is now an accepted author); the four variation
  phrases; one "lift" step per lift; and a "without lifting" step.
- **Regenerated curriculum outputs.** The Devanagari filmstrip-geometry
  ledger; 118 filmstrip SVGs and their figure hashes (Hindi 32, Marathi 31,
  Marwadi 25, Sanskrit 30), since every Devanagari headline caption changed;
  the book and narration hashes for 8 Hindi and 9 Sanskrit chapters; and 18
  lessons' modality records, written by the package's own
  `generatedModalityOutputs` only where the bytes changed.
- **Unchanged.** Script-owner evidence, the curriculum digest, the lesson
  count, the reinforcement counts and the filmstrip target counts do not
  move.
