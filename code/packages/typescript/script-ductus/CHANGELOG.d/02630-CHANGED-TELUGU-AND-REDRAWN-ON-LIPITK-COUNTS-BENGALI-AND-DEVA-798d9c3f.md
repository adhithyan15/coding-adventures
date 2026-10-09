### Changed — Telugu ఆ, ఇ and ఏ redrawn on LipiTk counts; Bengali and Devanagari audited and unchanged

- **Why.** Learners found the writing lessons lifted the pen far more than
  native writers do. Every Telugu, Bengali and Devanagari owner's pen-down
  run count was compared with the matching class of HP Labs India's LipiTk
  4.0 recognizers (lipi-reco-indic-char 4.0.0; counts and shares only, no
  trace copied). A glyph changes only when it draws more runs than a count
  held by at least 75% of its class's prototypes.
- **Telugu.** Three vowels did: ఆ 2 vs 1 (104/105), ఇ 3 vs 1 (104/106) and ఏ 3
  vs 2 (91/102). The earlier native-lift batches had left them because
  joining their parts meant reversing a source movement; the prototypes now
  settle the order and direction too.
  - **ఆ.** One run: the left lobe counterclockwise and the bowl, as before
    (104/104), then up through Noto's crossing into the right lobe, turned
    clockwise (91/104 cross its top moving right), back down through the
    crossing and left along the bar (104/104 end moving left). 3 movements.
  - **ఇ.** One run in the writers' order: from the upper-left lobe's free
    tip (95/104 start upper left) over both upper parts left to right
    (104/104), down the right side, left under the bowl and back along its
    top (96/104), and down the tail (102/104 end there). 4 movements.
  - **ఏ.** Noto prints ఏ's body with ఎ's outline point for point, so the
    body is ఎ's run, the arch drawn up from the junction (90/91 end the
    body at the arch's upper end), and the hook stays its own stroke, last
    (91/91). 2 strokes, 4 movements.
- **Left alone.** ష (3 vs 2 at 69%) and ఝ (5 vs 3 at 40%) have no 75%
  majority. Bengali: with the headline left out on both sides (the bar the
  convention draws last on the repo side; a flat, wide run in the top
  quarter on the LipiTk side), every letter and sign already matches its
  class's body majority. Devanagari: likewise, with the shirorekha left out;
  ख (body 3 vs 2 at 53%), त (2 vs 1 at 59%) and ह (2 vs 1 at 51%) draw more
  body runs than the most common count, but none of those counts reaches 75%.
- **Fit.** ఆ and ఇ were fitted along the medial line of the bundled Noto
  Sans Telugu outline at the default tolerances: on ink 1.0000, no ink left
  untraced. ఏ reuses ఎ's fitted body and its own unchanged hook.
- **Pins.** `tests/strokes/telugu.test.ts` pins the new runs, labels, the
  lobes' turning directions, ఇ's start, end and bowl order, and ఏ's body as
  ఎ's; `tests/ductusview/telugu.test.ts` pins ఆ and ఇ as "one unbroken
  stroke · N movements". Filmstrip-geometry ledger and the Telugu
  stroke-ownership data hash regenerated.
