### Added — Devanagari words: letter bodies, then one shared headline

- `src/headline-word.ts` (new): `composeHeadlineWord(word, script, font)`
  builds the stroke order of a Devanagari WORD from its cited letters. A
  printed word hangs from one headline, while every cited letter ends with
  its own "lift, then draw the shirorekha rightward" stroke, so the letters'
  strips side by side would draw one headline per letter. The composer takes
  each letter's headline stroke off (`splitHeadline`: the last stroke, one
  segment with exactly that label, horizontal, left to right; all 44 cited
  letters have one, no sign does), places the bodies at the font's advance
  widths, and ends with ONE headline over the whole word, captioned "lift,
  then the word's shirorekha". No stroke data is authored.
- The headline comes last because most native writers draw it last: HP Labs
  India's LipiTk 4.0 Devanagari recognizer stores the 33 consonants' native
  prototypes with the headline last in 82% of 2,706 and first in about 5%
  (`HEADLINE_LAST_SOURCE`). The word's source cites each letter by position
  and those counts; its `variation` says some writers draw the headline
  first and that the traces are single letters, not words.
- The composed path must fit the printed word at the default tolerances
  (over 97% of each stroke on ink, under 2% of the ink untraced), or the word
  is refused with the reason. That refuses a word whose printed headline is
  broken: थ ध भ श and अ आ ओ औ, whose own headline does not reach their left
  edge, anywhere but first (मथ: 88.2% on ink).
- `src/truetype.ts`: `Font.advanceFor(character)` reads `hhea`/`hmtx`
  (clamped; `undefined` without the tables).
- `src/ink.ts` (new): the ink measurements the stroke-honesty tests used
  (`makeInInk`, `fractionOnInk`, `distanceToPath`, `inkPoints`), moved
  unchanged from `tests/support/stroke-honesty.ts`, which re-exports them.
- `tests/filmstrip-ledger.test.ts`: a `composition: "shared-headline"`
  candidate is composed into one ledger entry keyed by the word; the outcome
  of every corpus candidate is pinned (Sanskrit मम, three lessons). The
  regenerated Devanagari ledger owner gains `devanagari:मम`.
