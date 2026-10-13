### Added — two Malayalam word lessons print filmstrips: ാ and ് written after their letter, സ്ക drawn apart, and a digit beside a word

- **Lessons.** ML-W01-namaskaram-read (നമസ്കാരം, drawn ന, മ, സ, ്, ക, ാ, ര, ം)
  and ML-W07-numbers-6-10-delayed-copy (ഏഴ് ൭, drawn ഏ, ഴ, ്, ൭) now print a
  strip. Every glyph in both was already cited; what was missing was where ാ
  and ് are written against their consonant. Strips: Malayalam 69 -> 71
  (855 of 1,367 writing lessons, up from 853).
- **Written-order rows.** `WRITTEN_SIGN_SIDES.malayalam` gains ാ "after" and
  ് "after", beside ം. Both cite Jayasree's composer (`sachn1/jayasree` at
  `e0c9d57`, `js/src/index.js`, `applyMarkStroke`): it classes both signs as
  suffix marks and animates a consonant's recorded strokes before the sign's,
  and its recorded ോ and ൊ end with ാ. That order is the composer's, not a
  recording of a consonant with the sign, so both records say confidence is
  medium. HarfBuzz shaping of Noto Sans Malayalam 2.104 shows every one of the
  38 consonants printed with ാ, and with a word-final ്, as the two unmoved
  glyphs, so no pair is fused.
- **Clusters (`APART_CLUSTER_SOURCES`).** A candrakkala between two consonants
  stays inside one grapheme in Malayalam (സ്കാ). A cluster is now drawn as its
  parts (consonant, ്, next consonant, then that consonant's signs) only when
  this new cited table lists it; it holds only സ്ക, which the font's 'mlm2'
  rules print as samlym, viramamlym and kamlym, each the glyph it is alone.
  The font fuses 174 of the 1,444 two-consonant clusters (ന്ത, മ്മ, ക്ക, ...);
  every unlisted cluster, and a listed one with a sign written before its
  consonant (സ്കോ), stays refused.
- **Digits.** `writtenPiecesOf` takes a decimal digit as one piece, like a
  base letter, so a word followed by its numeral (ഏഴ് ൭) is drawn word, then
  digit. The ledger still decides whether the digit is cited. No other lesson
  changes; Tamil ஏழு ௭ still waits for ு.
- **Inventory rows.** `malayalam.json`: the ാ record gains `compositionOrder`
  and a Jayasree `compositionSource`; the ് record keeps its order and its
  composition source moves from Unicode to Jayasree, with Unicode kept in the
  variation for what it says (encoded after its consonant) and does not
  (when it is written). No stroke order changes, so the filmstrip ledger is
  unchanged.
- **Still not drawn.** സന്തോഷം (ML-W02-santosham-guided-copy,
  ML-W03-santosham-delayed-copy): Noto prints ന്ത as one ligature glyph, which
  no cited ductus draws (Jayasree's ന്ത is Manjari's form). No row is added
  for െ, േ, ൈ or the left parts of ൊ and ോ: no word they would unlock prints
  its letters apart.
- **Tests and pins.** New case `malayalam-signs-after-their-letter` holds the
  rows to their records, the cluster table to its citation and to the bundled
  font's version string, and the two words' pieces. The anusvara case, the
  Tamil digit expectation, the Malayalam inventory evidence and
  `the-real-corpus` word list follow; `tests/filmstrip-target-counts/malayalam`
  is 71. HL06 gains an "As built" section.
- **Regenerated.** Two new SVG figures and the Malayalam figure hashes; the
  Malayalam chapters 1 and 7, which place them.
