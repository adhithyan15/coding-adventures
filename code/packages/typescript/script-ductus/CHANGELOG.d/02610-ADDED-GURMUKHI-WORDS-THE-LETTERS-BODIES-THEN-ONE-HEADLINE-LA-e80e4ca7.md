### Added — Gurmukhi words: the letters' bodies, then one headline last by convention, and the split-bar letters joined whole

- **Gurmukhi is a headline-word script.** `HEADLINE_WORD_SCRIPTS` gains
  `gurmukhi`, so `composeHeadlineWord` and `composeHeadlinePhrase` compose a
  Gurmukhi word of bare cited letters: each letter's body in reading order,
  then ONE headline, last, left to right ("lift, then the word's shirorekha").
- **The order is a convention, and says so.** GNPS's tracing lesson draws
  every letter's headline FIRST, and a letter's own strip is unchanged. The
  word-level order is the order fluent writers are described as using and the
  one Devanagari words already use; it is not separately sourced from a
  Gurmukhi recording. `GURMUKHI_HEADLINE_LAST_SOURCE` is printed in the
  footer ("by this book's convention for words, not from a Gurmukhi recording
  … GNPS's own letters draw the headline first"), and the word's variation
  repeats it. Devanagari words keep their LipiTk citation and note, byte for
  byte (`HEADLINE_LAST` picks the note by script).
- **Headline found at either end.** New `LETTER_HEADLINE` (per script: `at:
  "last"` with Devanagari's label, `at: "first"` with
  `GURMUKHI_LETTER_HEADLINE_LABEL`, "draw the headline from left to right").
  `splitHeadline(letter, rule?)` takes the headline off that end; it defaults
  to the letter's script and reads an unknown script like Devanagari, so
  existing callers are unchanged.
- **Split-bar letters.** `SPLIT_HEADLINE_LETTERS.gurmukhi` = ਅ ਖ ਘ ਪ ਮ: Noto
  Sans Gurmukhi prints a gap in their bar and GNPS draws no separate bar, so
  they join a word whole, and the shared headline runs only from the first to
  the last letter whose own headline was taken off. A split letter between
  two others (ਨਪਨ) is refused by the ink check (86.1% on ink); a word of split
  letters only is refused ("no letter of the word has a headline stroke of
  its own to share").
- **First movement.** A body that opens the word drops its now-false
  `LIFT_PREFIX` ("lift, then "), keeping the rest of the cited label (ਰਨ
  opens "come down the stem"). No Devanagari word changes.
- **Font.** Read with fontTools, Noto Sans Gurmukhi 2.004 has no GSUB or GPOS
  lookup acting on a run of bare base letters, so the composed outline is the
  printed word. No Gurmukhi sign is taken: none has a cited ductus.
- **Tests.** New `tests/headline-word-gurmukhi.test.ts` (20 cases): every
  cited letter has a front headline or is one of the five split letters, and
  the font shows exactly those gaps; single letters keep headline-first; ਪਰ,
  ਅਮਨ and ਮਨਨ compose with the exact strokes, headline span and footer; every
  refusal. `filmstrip-ledger.test.ts` pins the corpus outcomes: ਪਰ, ਅਮਨ,
  ਮਨਨ composed; ਉਹ and ਉਮਰ refused only because ਉ has no cited ductus.
- **Ledger.** `filmstrip-geometry.d/gurmukhi.json` gains `gurmukhi:ਪਰ`,
  `gurmukhi:ਅਮਨ` and `gurmukhi:ਮਨਨ`; no other entry or script changes.
- **Docs.** `headline-word.ts` and `strokes/gurmukhi.ts` headers, `index.ts`
  and the README describe the Gurmukhi case.
