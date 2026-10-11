### Added — twelve more writing lessons print strips: Malayalam ്, ഠ, ൊ, ോ and digits from Jayasree, Kannada ಞ, Chinese 尔

- **Lessons.** Twelve writing lessons that printed no filmstrip because a glyph
  had no cited stroke order now print one: ML-S02-sign-virama (്),
  ML-S119-vowel-sign-oo (ോ), ML-S143-vowel-sign-o (ൊ), ML-S147-letter-ttha (ഠ),
  ML-W01-sa-chandrakkala-ka (സ ് ക), ML-W07-digits-1-3, ML-W07-digits-4-5,
  ML-W07-digits-6-8, ML-W07-numbers-1-5-guided-copy and
  ML-W07-numbers-1-5-delayed-copy (൧-൯), KA-S133-letter-nya (ಞ) and ZH-W01-er
  (尔). Strips: Malayalam 59 -> 69, Kannada 56 -> 57, Chinese 72 -> 73 (807 of
  1,367 writing lessons, up from 795).
- **Source and licence.** The Malayalam glyphs cite Jayasree
  (`sachn1/jayasree` at `e0c9d57`), a recorded Malayalam handwriting data set
  whose stroke data is CC BY 4.0. Each inventory row in
  `data/scripts/malayalam.json` (the ് mark record, the new ൊ and ോ mark
  records, the ഠ letter row and the digit rows ൧-൯) credits "Jayasree" by
  Sachin Nandakumar with the licence and link, and the printed citation under
  each strip carries that credit. ಞ cites Chimple's `LIDO_kn2_0304` (facts
  only) with the recorded `bahama` trace as corroboration; 尔 cites Hanzi
  Writer Data at the pinned snapshot.
- **ൊ and ോ.** Noto prints each standalone sign with a placeholder dot between
  its parts, where the consonant would sit. It is not written, so the strips
  draw the two parts only, and script-ductus's coverage check skips exactly
  that contour for exactly these two glyphs (the 2% limit is unchanged). Both
  records say so and claim no written order against a consonant; the two
  lessons tell the learner the dot only marks where the consonant goes.
- **Inventory rows.** The ് record keeps its Unicode composition source; that
  variation now says the sign's own ductus is cited separately in
  `strokeOrderSource`. `malayalam.json` gains the ൊ and ോ mark records.
  `chinese.json` gains the 尔 row (61 rows; the vendored subset font already
  held the glyph).
- **Lesson prose.** KA-S133, ML-S02, ML-S119, ML-S143 and ML-S147 replace "copy
  what you see / this book does not yet tell you where to start" with the
  numbered-strip wording earlier strip lessons use. ZH-W01-er's cue said to
  write "the middle with its hook last"; the cited order draws the two dots
  last, so it now reads "the middle and its hook, then the two dots", and the
  description names the vertical before the side strokes.
- **Not drawn.** The ൯-൰ and ൬-൰ lists: no source has ൰. Words with ് or ോ
  (നമസ്കാരം, സന്തോഷം, ഏഴ്) still need a written-order row.
- **Pins and comments.** `tests/filmstrip-target-counts/` (malayalam 69,
  kannada 57, chinese 73); the Malayalam inventory evidence pins every
  Jayasree row's source, credit and licence wording, the two-part signs and
  the marks order, and the Kannada evidence pins ಞ. `validate.ts` comments
  name the Malayalam digits beside the Kannada ones (no behaviour change).
- **Regenerated.** The filmstrip ledger; 12 new SVG figures; figure, book and
  narration hashes; the affected Malayalam, Kannada and Chinese chapters with
  their narration; the edited lessons' modality manifests.
