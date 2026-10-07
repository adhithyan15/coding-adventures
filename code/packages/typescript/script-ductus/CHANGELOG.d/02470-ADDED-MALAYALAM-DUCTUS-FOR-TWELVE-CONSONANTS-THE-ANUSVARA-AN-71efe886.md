### Added Malayalam ductus for twelve consonants, ഏ, the anusvara and eight vowel signs, cited to Moag

- Twenty-two Malayalam glyphs enter `src/strokes/malayalam.ts`, appended after
  ബ so no existing key moves: the consonants ക യ ഖ ങ ച ഛ ഞ ഥ ധ ഭ ഫ ള, the
  independent vowel ഏ, the anusvara ം and the vowel signs ാ ി ീ ു ൂ ൃ െ േ, each
  drawn by itself. The consonant and vowel rows read their source through the
  existing helpers; the signs read theirs through a new `malayalamMarkSource`,
  from their mark records, as the Gujarati and Devanagari signs do.
- **Order, start, direction and end** come from Rodney F. Moag's *Malayalam: A
  University Course and Reference Grammar* (UT Austin South Asia Institute /
  COERLL, updated April 2018), Tables II-IV, "How to Write ... Symbols":
  numbered, arrowed movements in the hand of a native writer, Thomas Joseph.
  The book is CC BY-NC-SA 4.0, so only those facts are cited; no drawing is
  copied. Each record links the page's scan in Mathew Jacob's digital edition
  at commit `7141acd`.
- **Pen lifts: none.** Moag's numbers mark movements, not lifts, so each
  numbered movement is one segment of one run. The lift count rests on
  recordings: santhoshtr/hand (MIT) stores ക യ ാ ി ീ ു െ േ as one stroke,
  and grahyam (counts only) shows one run in every unique sample of every
  letter and sign but ക (105 of 107) and ച (44 of 45). ഏ has only six grahyam
  samples, so its record says confidence is medium.
- **Disagreements are recorded, not hidden:** യ's start (upper left in Moag and
  hand, upper centre in Thooval, middle left in grahyam); ു's end (the
  recordings end at the lower left, where Moag closes the loop at its top);
  and ം's direction, read from a small arrowhead as clockwise with
  medium-low confidence, where an unlicensed tracing app and the matching
  Gujarati and Devanagari signs run anticlockwise.
- **Fitted, not traced.** Every path follows the skeleton of the bundled Noto
  Sans Malayalam outline between the points Moag's arrows mark, retracing ink
  where the next movement starts on ink already drawn (ക's stem, ങ ഞ ധ's
  stems, ച ഛ's base, the middle tip of ങ and ള). Default tolerances, no
  overrides: every stroke wholly on ink, join gaps 0, no untraced ink. Every
  caption fits its printed panel in at most two lines, checked on the
  rendered SVGs.
- **Left out:** ജ (Moag's arrows do not show where the short stem between its
  two humps is drawn), ഠ (Moag runs the ring clockwise, Thooval and grahyam
  anticlockwise, and direction is all a ring teaches), ൈ (two separate pieces
  of ink need a lift no source records), ോ and ൊ (Moag numbers the sign's two
  parts but not the consonant between them), and ് (Moag's Table V shows it
  without movements).
- Tests: `tests/strokes/malayalam.test.ts` pins each glyph's labels, single
  run, joins, Moag URL, citation and font, and the anusvara's stated
  confidence; `tests/ductusview/malayalam.test.ts` pins each strip's movement
  count and "one unbroken stroke" summary. `tests/stroke-ownership.test.ts`:
  keys 582 -> 604, Malayalam 31 -> 53, new ordered key hash and non-Tamil data
  hash, measured after the captions were settled. The filmstrip ledger's
  Malayalam owner gains the twenty-two glyphs.
