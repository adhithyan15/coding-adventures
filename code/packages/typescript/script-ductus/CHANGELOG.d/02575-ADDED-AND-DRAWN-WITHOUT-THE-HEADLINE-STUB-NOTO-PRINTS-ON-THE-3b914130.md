### Added — ी, ो and ः drawn without the headline stub Noto prints on them

- **Three new Devanagari entries** at the end of `src/strokes/devanagari.ts`:
  ी (U+0940) as one run, from the hook's lower tip up, over the top and
  straight down the stem; ो (U+094B) as the stem straight down, a lift, then
  the flag from its upper-left tip, right and down to the top of the stem
  (the way े is drawn); and the visarga ः (U+0903) as two loops, upper dot
  first, each from its top and anticlockwise (the way ं is drawn), with one
  lift between them. Each is fitted to the bundled Noto Sans Devanagari
  outline of the sign by itself; every stroke is 100% on ink.
- **The source.** The mark records in `devanagari.json` cite the same HP Labs
  India LipiTk 4.0 Devanagari recognizer as the other signs (classes 49, 55
  and 60), re-measured from its stored prototypes (counts and shares only; no
  trace copied). The re-measurement reproduces every count already recorded
  for these classes (ी stem-first 39 of 91; ो two strokes 42 of 83; ै two
  strokes 76 of 83; ौ three strokes 35 of 83). ी: 49 of 91 draw the arch
  before the stem, 44 of them in one run, the commonest form (48%). ो: 58 of
  83 draw the stem down and lift before the flag, and 41 of those start the
  flag at its upper-left tip (49% overall). ः: 77 of 81 are two strokes, 76
  of them upper dot first; 66 turn both loops anticlockwise.
- **The headline-stub exception.** Noto prints each of the three with a
  short piece of headline (x 0 to 273, or 217 for ः, at the headline's
  height) that the writers, who wrote each sign without a headline, mostly
  do not draw (62 of 91 for ी, 43 of 83 for ो, none of the two-dot
  visargas). In a word that piece is the word's one headline, which native
  writers draw last across the whole word. The paths leave it undrawn and
  the strip shows it in grey. `tests/support/stroke-honesty.ts` gains
  `ExcusedInk`, a rectangle of printed ink one glyph's coverage check does
  not count, plus `untracedShare` and `insideExcused`: narrower than raising
  a glyph's untraced ceiling, because ink outside the rectangle must still
  be traced at the default 2%. `tests/strokes/devanagari.test.ts` declares
  `HEADLINE_STUBS` for exactly ी, ो and ः, and pins that they are the only
  three, that each rectangle's corners are on-curve points of the font's own
  outline at the headline height, that the default check fails without the
  exception (4.6%, 4.2% and 37% untraced) and passes with nothing else left
  over, and that no path runs along the stub. ā keeps drawing its stub,
  which a composed word's headline is built from.
- **Left out.** ि: its 75 prototypes split three ways (31 stem down, lift,
  arch; 22 one run from the arch's right tip down the stem; 20 one run up
  the stem and over), so no form wins a majority. ै and ौ stay out, as
  before (flag directions split; no majority stroke count).
- **Evidence.** `tests/strokes/devanagari-marks.test.ts` (hashes, classes,
  lifts, labels, start and turning geometry for the three, and ि ै ौ still
  absent) and `tests/ductusview/devanagari-marks.test.ts` (frames, lifts,
  summaries). `tests/headline-word.test.ts`: the non-ā signs with no
  headline stroke to take off go from 8 to 11.
- **Pins and ledger.** `tests/stroke-ownership/devanagari.json`: count
  53 -> 56, key and data hashes move; no other script changes.
  `filmstrip-geometry.d/devanagari.json` gains the three signs; no existing
  entry changes.
