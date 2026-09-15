### Added — HL11: 184 Words a Reader Can Now Say
- Paying down the closure debt. A lesson's `romanization` is what lets a reader
  use a word before they can read it — HL11 calls a headword shown beside one
  *exposure*, and it is the whole mechanism by which a book about an unfamiliar
  script is useful from page one. **489 headwords had none.** 184 now do, across
  all six Indic tracks; corpus closure violations fall 932 → 873.
- **These are recovered, not derived, and the difference is the point.** A
  mechanical ISO-15919 transliteration agreed with only **71%** of the 195
  romanizations these tracks' authors had already written by hand — and every
  disagreement was the machine being faithful to the spelling and wrong about
  the sound. Tamil was worst at 61%: it writes one letter for each of k/g/h,
  c/s, t/d and p/b, so transliteration says *cāppiṭu, paṭi, cukam, pēcu* where
  the words are said *sāppiḍu, paḍi, sugam, pēsu*. Publishing 344 derivations
  would have published 344 confident mispronunciations into a field the book,
  the app and the narration export all read aloud.
- So each one is recovered from what its lesson already tells the reader in
  prose — *"Say it va-ṇak-kam"* — a human's judgement, already reviewed and
  already shipped in the book, moved into the field that consumes it.
- A wrong grab is caught by a **skeleton** check: the word with every
  distinction the script does not record folded away, compared against the
  headword's own transliteration. Where nothing matches, the tool recovers
  nothing and says so. **160 headwords still need a human**, which is the
  correct output rather than a gap.
- `data/scripts/recover_romanization.py` carries the method, the measured
  per-script agreement rates, and the reason derivation was rejected.

