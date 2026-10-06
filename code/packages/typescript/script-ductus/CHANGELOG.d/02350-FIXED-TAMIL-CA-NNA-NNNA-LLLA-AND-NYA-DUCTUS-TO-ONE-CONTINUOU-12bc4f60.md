### Fixed Tamil ca, nna, nnna, llla and nya ductus to one continuous stroke

- `ச` (U+0B9A), `ண` (U+0BA3), `ன` (U+0BA9), `ழ` (U+0BB4) and `ஞ` (U+0B9E)
  in `src/strokes/tamil/` are now one stroke each, with no pen lift. They
  used to be two, two, two, three and four strokes. As in the first batch,
  the lifts came from reading the numbered arrows in Radhakrishnan's
  Appendix I ("Hand-movements") as separate pen-down runs.
- **Native evidence.** In HP Labs India's online Tamil handwriting data (the
  LipiTk 4.0 Tamil recognizer, trained on hpl-tamil-iso-char), the share of
  stored prototypes written as one pen-down stroke is: ச 88%, ண 93%,
  ன 94%, ழ 97% and ஞ 86%. Each `source.variation` cites this figure next to
  the manual.
- **The numbered movements keep their order.** Each glyph has the same
  number of movements as before: 4, 7, 6, 6 and 8. Where the next movement
  starts somewhere other than where the last one ended, the path retraces
  ink it has already drawn:
  - ச comes back along its middle bar to the crossing before the bowl.
  - ண and ன come back along the top bar to the right upright.
  - ஞ comes back along the top bar, and climbs back up the central upright
    to the outer bowl.
  - ழ runs round its lower hook to the tip and back, because the print face
    joins the hook to the stem only at the hook's far end.
- **Fitted to the print face.** ழ's low crossbar runs straight into the
  bowl's foot, so movement 3 carries on round the foot and up the right
  side, and movement 4 comes back over the top into the inner upright; the
  bowl is drawn once, in the manual's direction. ஞ now starts inside its
  small loop, as its record says, and spirals out into the top bar, instead
  of starting at the bar and ending at a dead end inside the loop. The old
  ழ path left the stem below the baseline untraced (9 of 999 ink points),
  and the old ஞ outer-bowl stroke was only 0.9776 on ink.
- **Refitted to the font.** ச, ழ and ஞ were fitted again along the medial
  line of the bundled Noto Sans Tamil outline. ண and ன keep their first
  six and five movements and gain one: back along the bar and down the
  right upright. `fractionOnInk` is 1.0000 for every glyph, with no
  override. No ink point is left untraced: 639, 1525, 1091, 999 and 1354
  points were sampled. Every join is exact. Every caption renders in two
  lines or fewer, which was checked by rendering the five filmstrips; ண's
  movement 2 is now "arch over to the first junction", because the old
  caption ran to three lines.
- **ங is unchanged.** It keeps its one lift: Frame 2 draws the upright as a
  separate, detached part first, and joining it to the body would reverse
  the manual's order or retrace the whole body.
- **Tests.** `tests/strokes/tamil/U-B9A`, `U-BA3`, `U-BA9`, `U-BB4` and
  `U-B9E` now pin:
  - zero lifts and the labels in order;
  - the directions the captions claim, and where each path retraces;
  - the new glyph-data hashes;
  - the citation and the phrases of each variation note, including the
    HP Labs figure. ண and ன keep their "variation|no single" check.

  The matching `tests/ductusview/tamil/` files now pin a filmstrip with no
  lift: the summary reads "one unbroken stroke · N movements", and in the
  last frame the whole pen path is the only path drawn. The
  filmstrip-geometry ledger was regenerated.
