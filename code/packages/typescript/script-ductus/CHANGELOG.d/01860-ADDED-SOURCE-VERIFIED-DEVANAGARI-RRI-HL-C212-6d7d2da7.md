### Added — source-verified Devanagari ऋ (U+090B)

- `ऋ` enters `src/strokes/devanagari.ts` with four pen-down runs and three
  lifts, derived from the four buildup panels of the Commons file the letter's
  `strokeOrderSource` already cited: Saurmandal, *Devanagari ऋ stroke order.svg*
  (CC BY-SA 3.0). The panels were read directly, not from memory: each panel is
  an Inkscape group labelled `1`–`4` in which the parts written so far are black
  and the parts still to come are `#c9c9c9`, so which run adds which part of the
  letter is recorded in the file itself. The `Arrows` and `Start markers` layers
  give each run its start point and its direction.
- Panel 2's loop is **anticlockwise**. Its solid arrow crosses the top of the
  little ball travelling leftwards and its dotted half — the portion hidden
  behind the ink — climbs the ball's right side. `data/scripts/devanagari.json`
  had described that turn as clockwise; the prose is corrected to match the
  source it cites. Nothing else in the letter's entry changed.
- The pen path itself is fitted to Noto Sans Devanagari, so it is checkable
  rather than merely asserted: all four strokes sit fully on the glyph's own
  ink, every segment join is exact, and no part of the letter is left untraced.
- `tests/stroke-ownership.test.ts` pins move because of this one glyph, and were
  re-measured rather than reasoned about: 349 → 350 keys, devanagari 43 → 44,
  and both corpus hashes.
