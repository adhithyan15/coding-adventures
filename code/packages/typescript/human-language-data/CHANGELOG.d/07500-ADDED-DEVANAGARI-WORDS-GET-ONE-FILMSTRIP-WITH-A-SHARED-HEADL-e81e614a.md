### Added — Devanagari words get one filmstrip with a shared headline

- `src/figure-targets.ts`: `headlineWordOf` makes a Devanagari writing
  lesson whose headword is ONE word of two or more bare letters a candidate
  with `composition: "shared-headline"`; script-ductus composes it as the
  letters' bodies followed by one headline over the word (HL06). Refused from
  the text alone: every vowel sign, nasal, visarga, nukta (also precomposed
  क़) and virama, since none has a cited written order against its consonant
  or the shared headline; ई and ऐ, which Noto Sans Devanagari 2.006 splits
  while shaping (GSUB `abvs` lookup 179, `HEADLINE_WORD_SPLIT_LETTER_SOURCES`);
  phrases, lists, digits, punctuation and other scripts. Whether a candidate
  is drawn is still the ledger's answer.
- `src/figure.ts`, `src/figure-cli.ts`: `ScriptFilmstripTarget.composition`,
  validated (only `"shared-headline"`, never with `letters`).
- `src/figure-filmstrip.ts`: a ledger entry for a whole word says in its
  `<desc>` that the frames draw each letter's body, then one headline, over
  the finished word; a letter's strip is byte-identical.
- The book alt text reads "How मम is written, letter by letter, then one
  headline, stroke by stroke".
- Tests: a new figure-targets case pins what is accepted and every refusal;
  the real-corpus case pins the three Sanskrit मम lessons; the Sanskrit
  filmstrip target count moves 48 -> 51.
