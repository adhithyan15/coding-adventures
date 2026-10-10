### Fixed — a strip of digits says digits, not letters

The digit lessons that print a strip of several digits (Persian FA-W19, Urdu
UR-W31, Malayalam ML-W07) called the digits letters: the strip's heading read
"How it is written — 3 letters, one after another", each group "Letter 1 of
3", and the book's caption "How these letters are written … ۰, ۱". A digit is
not a letter.

- **`filmstrip-caption.ts`.** `listNoun` names every kind of item a list
  holds, in the order letters, digits, signs. A digit is one `\p{Nd}`
  grapheme (۰, ൧, ೨, 7); a number sign such as Malayalam ൰ (`\p{No}`) is not
  one. So ۰ ۱ is "digits", ക ൧ "letters and digits", ു ൧ "digits and signs".
  "letters", "signs" and "letters and signs" read exactly as before, so the
  book's caption and the app's (`filmstripCaption`) change only for a list
  with a digit in it.
- **`figure-filmstrip.ts`.** `SequenceUnit` gains `"Digit"`, which
  `sequenceUnit` chooses when every entry's glyph is a decimal digit (a sign
  anywhere still makes it `"Part"`; a mix of letters and digits stays
  `"Letter"`). The heading, the group labels, the citation lines, the
  aria-label and the `<desc>` say "digits". `scriptSequenceFilmstripFigureSource`
  now puts the unit into the source for `"Digit"` as well as `"Word"`: the
  digit strips' SVG changed while their entries did not, so their source hash
  moves with them. Every other strip's source and hash are unchanged.
- **Regenerated.** Eleven strips, their figure hashes and three chapters:
  Persian FA-W19 zero-one, two-three, four-five-six, seven-eight-nine; Urdu
  UR-W31 zero-one, two-three; Malayalam ML-W07 digits-1-3, digits-4-5,
  digits-6-8, numbers-1-5-guided-copy, numbers-1-5-delayed-copy. Lesson text
  and narration are unchanged. The single-digit strips (Kannada KA-S140 to
  KA-S148) print no unit word, but their `<desc>`, which a screen reader
  speaks, said the movement is drawn "over the finished letter"; it now says
  "the finished digit", and those nine strips and the Kannada figure hashes
  are regenerated too.
- **Tests.** `figure-filmstrip-sequence.test.ts` covers the digit unit, its
  printed words and its hashed source; the list-captions case covers digit,
  mixed and unchanged nouns, and the book and app captions for ۰ ۱ and ൧ ൨ ൩.
