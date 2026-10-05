### Added Bengali: a ductus owner for nine glyphs cited to native writers' pen traces

- New owner `src/strokes/bengali.ts` (keys `bengali:<glyph>`, appended after
  every existing owner so no key moves): এ, ও, খ, থ, ঞ, ব, র and the signs ঃ
  and ঁ. The order, start, turning and lift count of each come from the
  native writers' tablet pen traces stored in HP Labs India's LipiTk 4.0
  Bangla recognizer (MIT licence; user manual Table 4, classes 7, 9, 12, 27,
  20, 33, 37, 47 and 48). The source of every path is looked up from the
  letter's own row in `data/scripts/bengali.json`, so the two cannot drift.
- Modal counts, all from the stored prototypes: ব 433 of 477 one stroke, র
  198 of 312 two, খ 189 of 212 one, থ 192 of 220 one, এ 194 of 226 one, ও
  231 of 239 one, ঞ 172 of 191 two, ঃ 195 of 195 two, ঁ 190 of 194 two. Each
  `variation` gives the start, direction and end shares behind its path.
- The headline is not assumed. In these traces it is drawn first, last,
  partly or not at all depending on the letter, so a letter is authored only
  where one placement wins a majority and covers the printed bar: ব and র
  draw it first, left to right; খ and থ end with a short move right into
  their flag; the others print none. ন, ক, ম, ল, য, ত and the rest are left
  out with the reason recorded in the inventory notes.
- The traces are scaled to a square, so they fix order and direction, not
  proportions; every path is fitted to Noto Sans Bengali at the default
  tolerances with no overrides (on-ink fraction 0.993 to 1.000, nothing
  untraced). No trace coordinates are copied.
- `src/scriptdata.ts` adds the Bengali inventory as the last script;
  `tests/support/font-fixtures.ts` gains `bengaliOutline`; new tests
  `tests/strokes/bengali.test.ts` (honesty at default tolerances, lifts,
  order, start and turning against the evidence) and
  `tests/ductusview/bengali.test.ts` (frames, lifts, summaries).
- `tests/stroke-ownership.test.ts`: keys 439 -> 448, a new `bengali: 9`
  count, the ordered key hash and the non-Tamil data hash move (commented);
  `bengali` joins the owner-module list. The filmstrip-geometry ledger gains
  the owner `bengali` (nine entries); no other owner changes.
