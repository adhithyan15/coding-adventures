### Added — Latin v, m and R from the Grundschrift-App, and eight marked letters drawn by analogy

- **Three letters from the school model**, `latin:v`, `latin:m` and
  `latin:R`, cited per level to the Grundschrift-App at commit f6dbd80 (no
  licence, facts only): v one stroke, down to the point and up; m one stroke,
  the stem, back up, two arches; R two strokes, the stem, then over the top,
  clockwise round the bowl, back along its foot (Andika starts the leg there)
  and down the leg. UJIpenchars2 was out of reach from the build container, so
  their records cite no native-writer count and say so.
- **Eight letters BY ANALOGY, not separately sourced:** è ê ë ï ä ö ē ç. No
  reachable source records the grave, circumflex, macron, cedilla, or the
  diaeresis on any letter but ü. Each is its cited base letter's first stroke,
  unchanged (ï takes i's stem, as í does), a lift, then the mark last, as the
  cited ü (left dot, then right dot), acute and tilde are drawn. A mark no
  cited record gives a direction runs left to right and top to bottom: the
  grave down to the right, the circumflex up to its peak and down in one
  stroke, the macron to the right, the cedilla down from the c's foot and
  round along the printed hook. Each record's citation opens "By analogy with
  the cited …, not separately sourced", and its url is the base letter's
  Grundschrift level, the only sourced part.
- **Fit.** All eleven are fitted to LatinPrint-Subset.ttf at the default
  tolerances: every stroke 100% on ink, nothing untraced, no override. The
  font already carried every glyph, so it is not re-subset.
- **Evidence.** `tests/strokes/latin.test.ts` takes the 42 glyphs; pins v's
  point, m's three movements and clockwise arches, R's stem, bowl and leg; and
  checks that each analogy letter starts with its base letter's stroke, puts
  every mark last (above, or below for ç), draws the dots left first and each
  mark in its stated direction, and that exactly these eight records, and no
  other, speak of analogy. `tests/ductusview/latin.test.ts` adds their frames.
- **Pins and ledger.** `tests/stroke-ownership/latin.json` 31 -> 42;
  `filmstrip-geometry.d/latin.json` gains the eleven entries.
- **README** and the owner's header state the analogy and what stays out (æ,
  œ, ÿ, capitals with marks).
