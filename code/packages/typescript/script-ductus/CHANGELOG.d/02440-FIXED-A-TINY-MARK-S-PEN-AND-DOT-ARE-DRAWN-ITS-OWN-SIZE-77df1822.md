### Fixed — a tiny mark's pen and dot are drawn its own size

- `src/filmstrip-ledger.ts`: new `penSizeFor(letter, options)`, with
  `TINY_STROKE_EXTENT` (150 font units) and `MIN_TINY_PEN_SCALE` (0.3).
  When a letter's whole pen path spans less than 150 units, the printed
  frames draw the pen line and the pen dot at `extent / 150` of the default
  26 and 34 units, never below 0.3 of them. `buildFilmstripEntry` applies it
  next to `captionSizeFor`; an explicit `penWidth` or `tipRadius` wins.
- Why: every frame's box is the ink plus fixed padding, so a dot-sized mark's
  panel zooms in on it, and the 68-unit dot was wider than the whole movement
  of Gujarati ં (60 units), Devanagari ं (52) and the nukta ़ (44). Those
  three are the only ledger entries under the threshold (the next is ゜ at
  185), so they are the only entries that change: ़ is drawn at 7.8 / 10.2,
  ं at 9 / 11.8, ં at 10.4 / 13.6. The viewBox does not depend on the pen,
  so the panels keep their size.
- `src/ductusview.ts` exports `DEFAULTS` and `penBounds` for this; the
  live app's own options are unchanged.
- Tests: ordinary letters unchanged, proportional scaling with no jump at
  the threshold, the floor, the real ़ and ં entries, explicit overrides, and
  a letter with no pen path. The ledger (`devanagari.json`,
  `gujarati.json`) is regenerated.
