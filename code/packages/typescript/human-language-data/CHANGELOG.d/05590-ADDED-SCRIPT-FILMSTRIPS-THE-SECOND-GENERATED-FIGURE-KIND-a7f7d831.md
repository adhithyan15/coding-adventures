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
  serialiser. `assertSafeFilmstripMarkup` re-checks each one against a five-tag
  allowlist — no `<script>`, no `on*` attribute, no stray bracket — before it
  can reach a committed file.
- Long letters wrap onto further rows at six frames per row rather than running
  off the page. The citation prints under the strip; a source's full note on
  variation goes into `<desc>` so it travels in the file without burying the art.
