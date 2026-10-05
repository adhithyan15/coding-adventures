### Fixed Tamil i, ai, o and oo ductus to one continuous stroke, and uu to two

- `இ` (U+0B87), `ஐ` (U+0B90), `ஒ` (U+0B92) and `ஓ` (U+0B93) in
  `src/strokes/tamil/` are now one stroke each, with no pen lift. They used
  to be two, five, two and two strokes. `ஊ` (U+0B8A) is now two strokes
  instead of four. The lifts came from reading numbered movements, or the
  frames of an animation, as separate pen-down runs.
- **Native evidence.** In HP Labs India's online Tamil handwriting data (the
  LipiTk 4.0 Tamil recognizer, trained on hpl-tamil-iso-char), the share of
  stored prototypes written as one pen-down stroke is: இ 100%, ஐ 99%,
  ஒ 99% and ஓ 97%; 93% of the prototypes of ஊ are two strokes. Each
  `source.variation` cites this figure next to the source.
- **The movements keep their order.** Each glyph has the same number of
  movements as before: 7, 5, 3, 3 and 9.
  - இ follows the print face's single line: down one lower diagonal into the
    lower-left loop, up the loop's outer side to the left crossing, along
    the middle bar into the lower-right loop, and up the other diagonal into
    the outer left side and the arch. Nothing is retraced. The old path
    climbed the wrong diagonal and left the other one untraced (18 of 1835
    ink points).
  - ஐ starts inside its spiral, where the print face's spiral has a free
    end, and curls outward; the old path curled inward and stopped at that
    end. It runs down the central upright to its free foot and draws it back
    up, and comes back down the short centre stem into the lower-right
    bowl.
  - ஒ and ஓ come back along the tail into the lower bowl, which the print
    face joins to the tail.
  - ஊ writes உ in one stroke, as `U-B89` does, lifts once, and writes ள in
    one stroke with the same six movements as `U-BB3`.
- **Refitted to the font.** All five were fitted again along the medial line
  of the bundled Noto Sans Tamil outline; ஒ and ஓ keep the geometry of
  their first two movements. `fractionOnInk` is 1.0000 for every stroke, and the 0.9
  overrides that ஐ and ஊ needed are gone. No ink point is left untraced:
  1835, 1295, 1137, 1226 and 1571 points were sampled. Every join is exact.
  Every caption renders in two lines or fewer, which was checked by
  rendering the five filmstrips; several old captions ran to three or four.
- **எ is unchanged.** It keeps its one lift: Frame 5 draws the right upright
  last and upward from its foot, and the print face does not join the lower
  foot to it, so one stroke would reverse the manual's order or retrace the
  bowl, the left side and the bar.
- **Tests.** `tests/strokes/tamil/U-B87`, `U-B90`, `U-B92`, `U-B93` and
  `U-B8A` now pin:
  - the lift count and the labels in order;
  - the directions the captions claim, and where each path retraces;
  - the new glyph-data hashes;
  - the citation and the phrases of each variation note, including the
    HP Labs figure. இ keeps its "variation|no single" check.

  The matching `tests/ductusview/tamil/` files now pin a filmstrip with no
  lift ("one unbroken stroke · N movements"; ஊ: "2 strokes · 1 pen lift · 9
  movements"). The filmstrip-geometry ledger was regenerated.
