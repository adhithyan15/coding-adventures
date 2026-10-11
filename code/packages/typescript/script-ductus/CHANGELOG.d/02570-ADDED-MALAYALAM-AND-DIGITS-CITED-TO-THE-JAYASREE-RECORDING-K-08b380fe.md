### Added — Malayalam ്, ഠ, ൊ, ോ and digits ൧-൯ cited to the Jayasree recording; Kannada ಞ; Chinese 尔

- **Thirteen Malayalam glyphs** enter `src/strokes/malayalam.ts`, appended
  after ൈ so no existing key moves: the candrakkala ്, the consonant ഠ, the
  digits ൧-൯ and the two-part vowel signs ൊ and ോ. Their order, start, direction and lifts cite Jayasree
  (`sachn1/jayasree` at commit `e0c9d57`, Sachin Nandakumar), a Malayalam
  handwriting animator whose ~300 centre lines one recorder traced over the
  Manjari typeface, one gesture per pen-down stroke. A recording shows lifts
  directly; every glyph here is one recorded stroke (`penLifts` 0), except ൊ
  and ോ, which are two.
- **Licence and credit.** Jayasree's stroke data is CC BY 4.0 (its
  `LICENSE-DATA`, verified in the cloned repository), so each record credits
  "Jayasree" by Sachin Nandakumar, links `js/src/stroke-data.raw.json` at the
  pinned commit and the licence, and says the path is an adaptation. That
  citation is printed under each strip. Only the facts are taken; no recorded
  coordinate is copied.
- **Fitted, not traced.** Each path follows the skeleton of the bundled Noto
  Sans Malayalam outline in the recorded order. Manjari is rounder and wider
  than Noto, and its ൪ ends in a straight rise where Noto curls, so the fit
  follows Noto. Default tolerances, no overrides: on ink 1.0000 on every
  stroke, joins 0, nothing untraced. Stems the stroke goes down and back up
  (൩ ൬ ൮ ൯) are retraced, as recorded.
- **ഠ** runs anticlockwise from the top. Jayasree breaks the earlier tie
  (Thooval and grahyam anticlockwise, Moag clockwise); the record names it.
- **്** is one stroke from the left tip round the cup to the right tip. Its
  Unicode composition source keeps the placement claim; its wording no longer
  says the mark makes no standalone ductus claim.
- `malayalamDigitSource` reads the inventory's `digits` rows, as
  `kannadaLetterSource` does for Kannada.
- **ൊ and ോ** are Jayasree's two recorded strokes: the left sign (െ or േ),
  a lift, then ാ clockwise. Noto prints each standalone sign as the cited
  left-sign outline, a placeholder dot, and the ാ outline shifted 923 (ൊ) or
  788 (ോ) units right, so each run is the cited path of its part, the second
  shifted with its outline. The new caption "lift, then draw ാ clockwise"
  fits its panel in two lines.
- **The placeholder exception.** The dot (Noto's `period.mlym` component)
  marks where a consonant would sit; nobody writes it, and skipping it leaves
  4.9% (ൊ) and 5.2% (ോ) of the ink untraced, over the 2% limit. The coverage
  check in `tests/support/stroke-honesty.ts` now leaves out exactly that one
  contour for exactly these two glyphs: `NOTO_PLACEHOLDER_CONTOURS`, keyed by
  ductus key, names each by contour index AND pinned bounds, and
  `tracedContours` throws if the contour at that index has other bounds. The
  2% limit is unchanged, and the on-ink and join checks still see the whole
  glyph. Tests pin the table to exactly ൊ and ോ; show that each glyph is
  exactly the cited left sign's contour, the dot and ാ's contour shifted, with
  the dot standing apart between them, so nothing else is skipped; show the
  dot is the same contour in both signs (moved 134 units); keep every other
  glyph's contours untouched; and, as a control, show the dot alone breaks
  2% without the exception. ൦ is recorded but no lesson draws it.
- **Kannada ಞ** cites Chimple's consonant lesson `LIDO_kn2_0304` (two hidden
  paths: the body with its loop clockwise, then the hook at the top right; one
  lift). Chimple's recorded `bahama` trace agrees. Facts only, medium
  confidence.
- **Chinese 尔** cites Hanzi Writer Data's `尔.json` at the commit the other
  characters use: the five strokes, two hooks and four lifts that already close
  你, fitted to the standalone Noto Sans SC 尔. A test holds its captions and
  stroke headings equal to 你's last five runs.
- **Tests.** The Malayalam owner test pins each Jayasree glyph's captions,
  single run, credit and licence wording, and checks every "clockwise" or
  "anticlockwise" caption against the turning of its points; ഠ's start, closure
  and direction; ്'s left-to-right cup; ൊ and ോ as two shifted cited runs;
  and ൦ undrawn. Kannada pins ಞ's
  two runs, clockwise loop and hook placement; Chinese counts 61 rows. The
  ductusview tests pin each new strip's frames and summary.
- **Pins.** `tests/stroke-ownership/`: malayalam 55 -> 68, kannada 54 -> 55,
  chinese 60 -> 61, with new key and data hashes, measured after the captions
  were settled. The filmstrip ledger is regenerated.
