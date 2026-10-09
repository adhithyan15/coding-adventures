### Added — three Punjabi word lessons print filmstrips: ਪਰ, ਅਮਨ and ਮਨਨ, one headline drawn last by convention

- **Lessons.** PA-C39-par-write (ਪਰ), PA-W02-aman (ਅਮਨ) and PA-W02-manan (ਮਨਨ)
  now print a strip. Every letter was already cited (GNPS); a Gurmukhi word
  was refused only because it was never composed. Strips: Punjabi 29 -> 32
  (858 of 1,367 writing lessons, up from 855).
- **Rule, not list.** `HEADLINE_WORD_SCRIPTS` gains
  `gurmukhi: /^\p{Script=Gurmukhi}$/u`: the Devanagari rule with nothing
  added (one word of two or more base letters, one code point each that NFD
  leaves alone; a phrase of such words word by word). No Gurmukhi row in
  `HEADLINE_WORD_SIGNS` (no sign has a source) or in
  `HEADLINE_WORD_SPLIT_LETTER_SOURCES` (the font changes no run of bare
  letters). In the corpus it reaches five candidates: ਪਰ, ਅਮਨ and ਮਨਨ compose;
  ਉਹ (PA-C42-oh-write) and ਉਮਰ (PA-W07-age-label) wait only for a cited ਉ.
- **The headline order is a convention.** The composed strip draws each
  letter's body without its own headline, then one headline last; GNPS's
  letters draw it first and a single-letter strip keeps that. The footer and
  the `<desc>` say the word-level order is this book's convention (as fluent
  writers do, and as Devanagari words are drawn), not a Gurmukhi recording.
- **Comments.** `SEPARATE_LETTER_SCRIPTS`, the shared-headline section and
  `DERIVED_FILMSTRIP_SCRIPTS.punjabi` in `figure-targets.ts` no longer say a
  Gurmukhi word is never composed, and explain the convention.
- **Tests and pins.** New case `gurmukhi-words-share-one-headline`: what the
  rule takes and refuses (every sign, nukta letters, labels, digits), the
  phrase, the candidate and the caption. The Devanagari case now expects
  `["devanagari", "gurmukhi"]`; `the-real-corpus` lists the three Punjabi
  words and pins ਉਹ, ਉਮਰ, ਨਹੀਂ and ਅਤੇ as still undrawn;
  `tests/filmstrip-target-counts/punjabi` is 32.
- **Regenerated.** Three new SVG figures and the Punjabi figure hashes;
  Punjabi chapters 15 and 39, which place them. Narration and modality do not
  change. HL06 gains an "As built" section.
