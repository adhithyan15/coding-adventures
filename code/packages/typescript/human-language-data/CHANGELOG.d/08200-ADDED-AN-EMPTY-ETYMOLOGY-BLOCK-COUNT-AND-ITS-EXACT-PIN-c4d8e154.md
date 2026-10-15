### Added — an empty-etymology-block count and its exact pin

HL24 §9.4. `hasVisibleEtymology` counted any `… taken apart` block, or any
heading naming an origin, as visible etymology whatever it said. A block that
takes apart a sentence, or lists a word's parts and stops, passed, so the
visible count could reach zero over a track that still owed those words their
origin. A review of the Spanish track found 48 such blocks.

- **`blockStatesOrigin(block, language)`** (src/etymology-coverage.ts): true
  when the block passes `statesOrigin`, or, inside a block the parser types
  `etymology` and nowhere else, one of 26 loose origin signals in
  `TAKEN_APART_ORIGIN_SIGNALS` fires. `takenApartOriginSignals(text,
  language)` names the ones that fire, for tests and re-measurement.
- **`hasEmptyEtymologyBlock(lesson)`** and the per-track and summary count
  **`wordLessonsWithEmptyEtymologyBlock`**: word lessons visible on structure
  alone, whose preamble fails `statesOrigin` and none of whose blocks passes
  `blockStatesOrigin`. Disjoint from `wordLessonsWithoutVisibleEtymology`.
  Each gap entry now carries `emptyEtymologyBlock`. The count is in
  `renderEtymologyCoverage`, the gap report's `etymology:` summary line, and
  the report JSON `summary` (src/report.ts).
- **The signals, measured.** Of 4,782 word lessons visible on structure alone
  with no `statesOrigin` statement anywhere, the signals read 4,307 as origins
  and left **475**. The requested list was kept whole; `goes-back-to` was
  tightened after it read "the tenth goes back to behaving", `a-loan` refuses a
  quoted gloss, and `learned-word` needs its article. Six signals were added
  after measurement: `source-named` ("DEDR 3376 lists ..."), `native-family`
  ("native Dravidian"), `derived-from` ("formed from *dessiner*"),
  `donor-word` ("Latin said *hodie*"), `cousin-of` and `is-english`. A bare
  donor-language name was measured and rejected (about one hit in seven
  misfired). Hit counts, sole rescues and the known false positives and
  negatives are in the module header.
- **`EMPTY_ETYMOLOGY_BLOCK_PINS`** in tests/root-ledger.test.ts pins every
  track's count exactly, beside `ETYMOLOGY_GAP_PINS`: arabic 19, bengali 3,
  chinese 43, french 66, german 43, gujarati 24, hindi 7, italian 7,
  japanese 2, kannada 8, latin 31, malayalam 6, marathi 5, marwadi 0,
  persian 2, portuguese 5, punjabi 31, russian 52, sanskrit 12, spanish 88,
  tamil 12, telugu 8, urdu 1. A rise names the newest lessons; a fall asks
  for the pin to be lowered. The pin may only fall.
- **Tests.** Every signal has firing fixtures and a near-miss control, checked
  against the signal table so a new signal cannot land untested; a syntax-only
  block is counted empty and a "built from" block is not; the documented
  misses are pinned.
- **Spec.** HL24 §9.4 defines the count and its signals; §4 lists the new pin.
