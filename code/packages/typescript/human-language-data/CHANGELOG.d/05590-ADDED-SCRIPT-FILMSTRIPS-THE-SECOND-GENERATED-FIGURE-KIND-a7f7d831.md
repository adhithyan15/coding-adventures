### Added — script filmstrips: the second generated figure kind

- `script-filmstrip` targets in `core/figure-generation.json` render one letter's
  handwriting as a strip of frames: frame N shows movements 1..N, earlier
  movements muted, the movement being added in ink over the finished glyph, and
  the segment's own authored label as its caption. Three ship as proof — Tamil
  **அ** (`TA-S119-letter-a`), Devanagari **आ** (`HI-S04-letter-aa`) and
  Perso-Arabic **چ** (`FA-C03-chist`).
- A target names its `script` and `glyph`; the geometry comes from
  `data/ductus/filmstrip-geometry.json`, generated and byte-checked by
  `script-ductus`. `check:figures` gates the SVGs as it always has.
- Frames arrive as SVG fragments escaped by `script-ductus`'s audited
  serialiser. `assertSafeFilmstripMarkup` re-checks each one before it can reach
  a committed file: a five-tag and sixteen-attribute allowlist (an allowlist, not
  an `on*` denylist, so `href`/`style`/`filter` are refused today rather than the
  day a live tag is added), balanced nesting so a fragment cannot close the
  positioning group and draw a forged citation loose on the figure, and text that
  no real serialiser could have written — a stray bracket, a bare `&`, an unknown
  entity or a control character. Every regex in it scans linearly, because the
  input is a file on disk.
- Each frame is placed in a **nested SVG viewport**, not a `<g transform>`, so a
  frame clips to its own panel whatever its contents' transforms say. `transform`
  has to be on the allowlist, and one `translate` with the right numbers would
  otherwise drop an allowlisted `<text>` exactly where the citation line goes:
  containment is geometry, so it gets a geometry answer rather than an assertion.
- Long letters wrap onto further rows at six frames per row rather than running
  off the page. The citation prints under the strip; a source's full note on
  variation goes into `<desc>` so it travels in the file without burying the art.
