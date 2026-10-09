### Changed — Tamil அ, ஆ, எ, ங and ஷ drawn in one stroke, on LipiTk counts

- **Why.** Learners found the Tamil lessons lifted the pen far more than
  native writers do. Every Tamil owner's stroke count was compared with the
  majority count of the matching class in HP Labs India's LipiTk 4.0 Tamil
  recognizer (lipi-reco-indic-char 4.0.0; counts and shares only, no trace
  copied). Five letters drew more strokes than a clear majority: அ 2 vs 1
  (104/114), ஆ 2 vs 1 (28/29), எ 2 vs 1 (81/88), ங 2 vs 1 (92/108) and ஷ 4 vs 1
  (175/188). Every other owner already agreed (ஊ is 2, as 68/73 are); the
  signs ா ி ீ ெ ே ை were already LipiTk-cited, and ் has no LipiTk class.
- **அ, ஆ (`U-B85`, `U-B86`).** Frame 4's order is kept; the lift before the
  right upright is gone. After the horizontal the pen climbs the upright to
  its top (95/104 and 26/28 do) and draws it down over that ink; ஆ runs on
  into its long-vowel loop. 6 and 7 movements.
- **எ (`U-B8E`).** Native writers' order replaces Frame 5's (outer left side
  first, upright drawn up after a lift): from the inner end of the curl,
  clockwise round the bowl (81/81), up the left side, along the top bar to
  its right end and back, and the upright down last (79/81). 4 movements.
- **ங (`U-B99`).** Native writers' order replaces Frame 2's (detached upright
  first): down the left upright and back up (69/92), out along the top bar
  and back, down the inner stem to its free foot and back up, round the bowl,
  out along the low bar to its left end and back, and up the right upright
  last (81/92). The old body crossed the white gap under the inner stem
  (97.1% on ink); the new path is 100% on ink. 6 movements.
- **ஷ (`U-BB7`).** Narale's four numbered parts, in his order, without
  lifting: the inner loop clockwise (174/175), on round the bottom and over
  the outer body to the bar, the bar left to right, up its right end and back
  over into the right loop (clockwise, 126/175), and down the tail (ends in
  the bottom fifth, 174/175). Its 0.9 on-ink override is gone. 6 movements.
- **Sources.** Each citation keeps the manual or chart and gains the LipiTk
  class ("drawn in one stroke after", or for எ and ங "reordered, and drawn in
  one stroke, after"); the URL is unchanged. Each variation note gives the
  counts and says plainly what it overrides and why: a numbered movement or
  part of a teaching chart is not evidence of a pen lift.
- **Fit.** All five fitted along the medial line of the bundled Noto Sans
  Tamil outline at the default tolerances, no override: on ink 0.9972 (அ),
  0.9978 (ஆ, the old curl's two edge samples) and 1.0000 (எ, ங, ஷ); untraced
  0% except ஆ's unchanged 0.94% (the upright's foot below the loop).
- **Pins.** The five `tests/strokes/tamil` glyph-data hashes, labels, the
  claimed start/direction/retraces and source phrases; the five
  `tests/ductusview/tamil` files now pin "one unbroken stroke · N movements".
  Filmstrip-geometry ledger regenerated; stroke-ownership pins unchanged.
  README notes the five.
