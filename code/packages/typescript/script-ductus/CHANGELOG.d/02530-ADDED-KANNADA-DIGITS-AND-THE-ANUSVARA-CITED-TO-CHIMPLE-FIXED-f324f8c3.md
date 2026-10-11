### Added — Kannada digits ೧-೯ and the anusvara ಂ, cited to Chimple; ಃ fixed; Malayalam ജ and ൈ

- **Kannada digits ೧-೯** enter `src/strokes/kannada.ts`, appended after ಠ so
  no existing key moves. Each is one pen-down run. Order, start and direction
  come from the hidden tracing path of Chimple's digit lesson `LIDO_kn2_0318`
  (Sutara Learning Foundation, Bangalore; `chimple/chimple-zips` at commit
  `5b137ab1`). That repository has no licence, so only facts are cited and
  nothing is copied. `LIDO_kn2_0319` repeats the same paths byte for byte, so
  the two lessons count as one source. Chimple keeps one path per pen-down run
  elsewhere: on every consonant, the anusvara ring is its own last path. So
  one path means one stroke. This is one designer-authored source, so every
  record says confidence is medium. **೦ has no ductus.** Chimple's ೧೦ re-uses
  the ೧ picture and draws no zero.
- **ಂ** is one anticlockwise ring, begun at its left side (9 o'clock). In all
  34 of Chimple's consonant + anusvara pictures the ring is drawn last and
  anticlockwise; it starts at the left in 20 of them. Chimple's recorded ಅಂ
  (`chimple/bahama` at `62608a6`, MPL-2.0) agrees. The two repositories come
  from one organisation, so the record counts them as one source.
- **ಃ fixed.** Its first path called its first loop "the upper dot" but drew
  it round the lower one: font units point up, and the path used the smaller
  y. The cited animation and all 35 of Chimple's consonant + visarga pictures
  draw the upper dot first. Both loops now run anticlockwise from near
  8 o'clock, as 31 of the 35 do. Its captions become "circle the upper dot
  anticlockwise" and "lift, then circle the lower dot".
- `kannadaLetterSource` now also reads the `digits` rows.
  `verifiedLetterFont` (and the test helper `fontForDuctus`) resolve a cited
  digit's font as they do a letter's.
- **Malayalam ജ** is appended after േ. It follows Moag's six numbered
  movements (p. xxvi), in one run. Arrow 2 stops above the short stem and
  arrow 3 starts at its foot. With no lift between them (*hand*: one stroke;
  *grahyam*: 45 of 45 unique samples in one run), the pen can only reach the
  foot down the stem. So movement 2 ends with that descent, and movement 3
  retraces it. The record says this join is read, not drawn, and gives it
  medium confidence.
- **Malayalam ൈ** follows Moag's four movements (p. xxii): two coils of െ,
  each the cited െ path. Noto composes the standalone sign from two copies of
  െ, 715 units apart. The gap is paper, so the pen lifts once. No recording
  of ൈ itself was found, so the record says the lift is reasoned and gives it
  medium confidence. Like every Malayalam vowel sign, it is drawn only alone.
- **Fitted, not traced.** Every new path follows the skeleton of the bundled
  Noto outline between the points the sources mark. Default tolerances, no
  overrides: `fractionOnInk` is 1.0000 on every stroke, join gaps are 0, and
  no ink is left untraced. Every new strip was rendered and checked: every
  caption fits its panel in at most two lines.
- **Not drawn, with reasons:**
  - Malayalam ഠ: Moag's ring runs clockwise, but Thooval and grahyam run it
    anticlockwise.
  - Malayalam ൊ and ോ: Noto's standalone glyphs put a placeholder dot
    between the two parts. Tracing the sign alone leaves about 5% of the ink
    untraced, which is above the default limit.
  - Malayalam ്: Moag gives its placement only, with no movements.
- Tests:
  - `tests/strokes/kannada.test.ts`: every digit's movements, source,
    citation and font. Every caption that says "clockwise" or
    "anticlockwise" is held to the turning of its points. ೦ stays undrawn.
    ಂ's ring and start are pinned, and ಃ's upper loop must come first.
  - `tests/strokes/malayalam.test.ts`: ജ joins the Moag table, its stem
    caveat is pinned, and ൈ is pinned as two shifted െ runs.
  - `tests/ductusview/`: strip counts and summaries.
  - `tests/strokes.test.ts`: counts cited digit rows among the verified
    claims.
  - `tests/stroke-ownership.test.ts`: keys go 636 -> 648, Kannada 44 -> 54,
    Malayalam 53 -> 55, with a new ordered key hash and non-Tamil data hash,
    measured after the captions were settled.
- The filmstrip ledger is regenerated: Kannada gains ಂ and ೧-೯, ಃ is
  redrawn, and Malayalam gains ജ and ൈ.
