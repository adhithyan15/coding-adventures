# glyph-parser (FNT02)

Reads **glyph outlines** out of a TrueType font's `glyf` and `loca` tables.

`font-parser` (FNT00) tells you how wide a glyph is. This tells you what shape
it is: contours of lines and quadratic curves, in font design units, ready for
a rasteriser, an SVG path, or a PDF content stream.

```rust
let bytes  = std::fs::read("Inter-Regular.ttf")?;
let font   = font_parser::load(&bytes)?;
let parser = glyph_parser::GlyphParser::new(&font)?;

// `None` means the glyph has no outline -- a space. Not an error.
if let Some(outline) = parser.glyph_outline(36)? {
    for contour in &outline.contours {
        for command in &contour.commands {
            match command {
                glyph_parser::Command::MoveTo { x, y } => { /* start a subpath */ }
                glyph_parser::Command::LineTo { x, y } => { /* straight edge   */ }
                glyph_parser::Command::QuadTo { cx, cy, x, y } => { /* curve   */ }
            }
        }
    }
}
```

## Where it sits

```
  FNT00  font-parser    table directory, cmap, metrics, kerning, MATH
     |
     v
  FNT02  glyph-parser   ← this crate: glyf + loca outlines
     |
     +--> PDF font embedding and subsetting   (PDF-3)
     +--> maths rendered as outlines to SVG   (TEX-4)
     +--> FNT03 rasteriser
```

## Why it matters

Engram is a language-study app, so Tamil, Hindi, Japanese and Chinese decks are
the normal case. None of that renders with PDF's base-14 fonts, so a PDF export
has to *embed* a font — and embedding a whole CJK face is megabytes where a
study sheet uses a few hundred glyphs. Subsetting is what makes the export
usable, and you cannot subset what you cannot parse.

For maths, a formula drawn as outlines renders identically on a machine that
does not have the font installed. For a flashcard that is correctness, not
styling.

## What it handles

- `loca` in **both** formats — short (`u16`, storing half the offset) and long
- Simple glyphs: run-length-encoded flags, and the short/same-or-positive delta
  packing for coordinates
- The implied on-curve points TrueType omits between consecutive control points
- Contours that begin off-curve, including the all-off-curve case
- Composite glyphs, flattened recursively: plain offsets, uniform scale, x/y
  scale, full 2×2 transforms, nesting, and anchor-point placement
- Empty glyphs (a space) as `Ok(None)` rather than an error

## What it does not

- **CFF/CFF2 outlines.** A different format, not a broken one — `GlyphParser::new`
  returns `UnsupportedFontFormat` so callers can fall back rather than report
  corruption.
- Bitmap-only fonts (`sbix`/`CBDT`), hinting, and variable-font deltas (`gvar`).
- Scaling to a pixel size. Coordinates come out in design units; multiply by
  `size / units_per_em` at the point of use. That is the rasteriser's job.

## Testing

Outline extraction is transcription, and transcription bugs produce something
that still looks like a glyph. A fixture written by the same hands as the
parser would agree with it perfectly and be wrong in the same way.

So the expectations come from **fontTools**, reading real shipping fonts —
Inter and two Noto faces, covering both `loca` formats and 2,104 composite
glyphs. Every glyph in every font is checked, 4,161 of them, plus a synthetic
font built by fontTools for the composite features no shipping font here
happens to use.

```bash
cargo test -p glyph-parser
python3 code/scripts/generate_glyph_oracle.py   # regenerate the fixture
```

See `tests/fixtures/PROVENANCE.md` for the fonts' origins and licences.

## Dependencies

`font-parser`, and nothing else — not even for the tests. The oracle fixture is
a line format precisely so the test can read it with `split_whitespace`.
