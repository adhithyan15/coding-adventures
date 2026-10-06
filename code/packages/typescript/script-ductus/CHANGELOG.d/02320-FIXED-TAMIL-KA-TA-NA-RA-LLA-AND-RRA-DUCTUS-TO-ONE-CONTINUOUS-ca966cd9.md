### Fixed Tamil ka, ta, na, ra, lla and rra ductus to one continuous stroke

- `க` (U+0B95), `த` (U+0BA4), `ந` (U+0BA8), `ர` (U+0BB0), `ள` (U+0BB3) and
  `ற` (U+0BB1) in `src/strokes/tamil/` are now one stroke each, with no
  pen lift. They used to be three, four, three, three, three and three
  strokes. The lifts came from reading the numbered arrows in
  Radhakrishnan's Appendix I ("Hand-movements") as separate pen-down runs.
  The arrows number movements. The manual does not say the pen lifts
  between them.
- **Native evidence.** In HP Labs India's online Tamil handwriting data (the
  LipiTk 4.0 Tamil recognizer, trained on hpl-tamil-iso-char), the share of
  stored prototypes written as one pen-down stroke is: க 88%, த 88%, ந 85%,
  ர 90%, ள 90% and ற 99%. Each `source.variation` cites this figure next to
  the manual.
- **The numbered movements keep their order.** Each glyph has the same
  number of movements as before: 6, 7, 6, 4, 6 and 5. Where the next
  movement starts somewhere other than where the last one ended, the path
  retraces ink it has already drawn, as the Japanese け, え and ん do.
  - த retraces its right bowl twice, because the manual numbers the bowl
    before the left loop.
  - ர climbs back up its left upright.
  - ந climbs back up its middle upright.
  - ற and ள go down and back up one shared stem.
  - க's movement 3 now ends at the inner crossing, and movement 6 crosses
    the middle bar. Before, these segments did not join end to end.
  - ள now starts inside its loop, because the print face joins the loop to
    the middle upright only at the top.
- **Refitted to the font.** Each path was fitted again along the medial line
  of the bundled Noto Sans Tamil outline. `fractionOnInk` is 1.0000 for
  every glyph. No ink point is left untraced: 766, 930, 854, 551, 922 and 877
  points were sampled. Every join is exact. Every caption is short enough to
  render in two lines or fewer, which was checked by rendering the six
  filmstrips. Some old captions ran to three or four lines.
- **Tests.** `tests/strokes/tamil/U-B95`, `U-BA4`, `U-BA8`, `U-BB0`, `U-BB1`
  and `U-BB3` now pin:
  - zero lifts and the labels in order;
  - the directions the captions claim, and where each path retraces;
  - the new glyph-data hashes;
  - the citation and the phrases of each variation note, including the
    HP Labs figure.

  The matching `tests/ductusview/tamil/` files now pin a filmstrip with no
  lift: the summary reads "one unbroken stroke · N movements", and in the
  last frame the whole pen path is the only path drawn. ள and ர gain these
  view tests for the first time. The filmstrip-geometry ledger was
  regenerated. The stroke-ownership pins are unchanged, because Tamil's own
  data is pinned per glyph.
