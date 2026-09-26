### Changed — a digit lesson is anchored in a quantity, not a word (HL-C443)

`letter-anchoring.ts` gains a fourth kind of letter lesson, `numeral`. A
lesson counts as a numeral when every glyph that is not anchored is a decimal
digit (Unicode `Nd`). Such a lesson is no longer reported as cold.

No word is spelled with ౧ or ೨. ఒకటి "one" uses letters. So "an earlier word
holds this shape" could never be true of a digit, however a track was ordered,
and those lessons would have stayed cold forever. That does not mean a digit has
nothing to hang on. The reader arrives knowing every amount from 0 to 9, and a
digit for each. The track teaches ౧ as "one, written this way". That anchor is
the quantity, not a word.

- A digit that an earlier word does hold (Kannada ೧ನೇ, "first") is still just
  anchored.
- A set that mixes a digit with a real letter is judged by that letter. A cold
  letter cannot hide behind a numeral.
- Tracks and the summary gain a `numeral` count.

Measured effect, cold letter lessons:

| Track    | Before | After | Numerals |
|----------|-------:|------:|---------:|
| telugu   | 10     | 0     | 10       |
| kannada  | 10     | 0     | 10       |
| punjabi  | 7      | 3     | 4        |

Punjabi's three that remain are real letter sets. The ceilings also take in
drift from earlier HL-C443 passes: builds-toward for arabic went 5 → 4 and for
bengali 12 → 11.
