# Script filmstrips: the core contract

Two kinds exist:

- **`etymology-route`** reads only the ordered `roots` asserted by that lesson and
  renders them through `paint-vm-svg`.
- **`script-filmstrip`** prints how a letter is written: one frame per labelled
  movement, the finished glyph behind in pale grey, the strokes already written
  muted, the movement being added in ink, and the segment's own authored label as
  its caption. A target adds `script` and `glyph`:

  ```json
  { "kind": "script-filmstrip", "lessonId": "TA-S119-letter-a",
    "script": "tamil", "glyph": "அ",
    "output": "tamil/book/figures/TA-S119-letter-a-filmstrip.svg" }
  ```

  The geometry comes from per-script owners in `data/ductus/filmstrip-geometry.d/`, which
  `@coding-adventures/script-ductus` generates from its cited pen paths and the
  vendored font and byte-checks in its own suite. **Adding a filmstrip target is
  two steps**: declare it here, then run `npm run generate:filmstrip-ledger` in
  `script-ductus` before `npm run generate:figures` here. A missing entry fails
  with the command to run. Stroke order is never invented — an uncited letter has
  no ledger entry and therefore no figure. A one-frame strip (़, ्, ا) wraps its
  heading and citation at the width of a two-frame strip, not at its single
  150-unit panel, so it is not a tall column of text; its panel is unchanged.
