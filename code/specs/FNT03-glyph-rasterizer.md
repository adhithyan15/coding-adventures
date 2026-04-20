# FNT03 — Glyph Rasterizer

## Overview

FNT03 is the **last keystone** of the device-independent text
pipeline. It takes a vector glyph outline (from FNT02) and
produces a pixel-coverage bitmap ready to be composited into a
paint target.

```
Font bytes
    │
    ▼
FNT00 (font-parser)        ─ table parsing, metrics
    │
    ▼ glyph_id
FNT02 (glyph-parser)       ─ outline extraction
    │
    ▼ GlyphOutline
FNT03 (this spec)          ─ outline → pixel-coverage bitmap
    │
    ▼ GlyphBitmap
Paint backend              ─ tints by fill color, composites
    │
    ▼
target pixel grid
```

The rasterizer answers one question:

```
Given a vector outline and a target pixel size, what is the
per-pixel coverage mask I composite onto the target surface?
```

Output is deliberately **single-channel grayscale coverage**, not
pre-tinted RGBA. The paint layer multiplies the coverage mask by
the fill color at composite time. Separating geometry from color
means the same rasterized bitmap can be reused across multiple
fill colors (text in blue, same glyph in red — one rasterization,
two composites) and matches the conceptual model of every
production rasterizer (FreeType, Skia, CoreGraphics).

FNT03 is the end of the geometry half of the pipeline. Downstream
of FNT03 is pure color compositing — the paint backend's job.

---

## Scope and explicit non-goals

### What FNT03 handles

- Scan conversion of quadratic Bezier outlines (from FNT02) into
  per-pixel coverage values.
- Anti-aliasing via analytical or super-sampled coverage.
- Subpixel origin positioning — a glyph whose baseline origin is
  at `(123.7, 456.3)` produces a bitmap whose left edge aligns
  with `x=123` but whose pixel values reflect the 0.7-pixel
  horizontal offset.
- The TrueType non-zero winding fill rule (always; TrueType
  outlines are non-zero).
- Flattening of `QuadTo` commands into short line segments for
  scan conversion, using adaptive subdivision.
- Bounding-box-sized output — the output bitmap is just big enough
  to hold the glyph's ink, with a small padding margin.

### What FNT03 does NOT handle

- **Hinting / grid-fitting.** Outlines are rasterized as-is. The
  TrueType `cvt` / `fpgm` / `prep` instructions FNT02 parsed past
  are never interpreted. At ≥ 14px on modern DPI, the quality
  difference is subtle; at FNT05's pixel-aligned test sizes it's
  zero.
- **Subpixel rendering (RGB/BGR LCD anti-aliasing).** The output
  is grayscale coverage, not per-colour-channel coverage. LCD
  subpixel rendering is a display-specific optimization that
  trades one kind of aliasing for another. A future FNT03b can
  add it as an opt-in output mode.
- **Color fonts** (COLR / CPAL / sbix / SVG). FNT02 already
  delivers a single monochrome outline; FNT03 rasterizes it as
  monochrome coverage. Color layers are a future-spec concern.
- **Emoji rasterization.** Same — delivered as a flat outline.
- **GPU rasterization.** FNT03 is a CPU algorithm spec. A GPU
  tessellation + stencil-cover path is a separate spec (P2D08
  mentions a tessellator).
- **Outline stroking.** FNT03 always fills; it does not stroke.
  Outline-stroked glyphs (e.g. Silkroad Unicode style) produce
  visually degraded fills, but that's the caller's decision.
- **Kerning / shaping / layout.** FNT03 rasterizes one glyph at
  a time. The caller (paint backend) iterates over the
  PaintGlyphRun's glyphs and positions them.

---

## The GlyphBitmap output type

```
GlyphBitmap {
  /// Width of the coverage bitmap in pixels.
  width: u32,

  /// Height of the coverage bitmap in pixels.
  height: u32,

  /// Raw coverage data. One byte per pixel, 0..=255, row-major,
  /// top-left origin.
  ///
  /// Value semantics:
  ///   0   = pixel is fully outside the glyph
  ///   255 = pixel is fully inside the glyph
  ///   N   = pixel is N/255 covered (linear, premultiplication
  ///          applied at composite time)
  ///
  /// The coverage values are in linear space before gamma
  /// correction. See "Coverage vs. gamma" below.
  pixels: Vec<u8>,

  /// Horizontal offset from the glyph's baseline-origin x to the
  /// left edge of the bitmap, in target pixels. Usually negative
  /// for characters with left-side bearing (e.g., italic 'f' whose
  /// ink bleeds left of the pen position).
  bearing_x: i32,

  /// Vertical offset from the glyph's baseline-origin y to the
  /// TOP edge of the bitmap, in target pixels. Positive = the top
  /// edge is above the baseline (usual case for ascenders).
  /// Negative = top edge below baseline (would be unusual — most
  /// glyphs have at least some pixels above baseline).
  bearing_y: i32,

  /// The glyph ID this bitmap was rasterized from. Passed through
  /// for caching keys and debug logging.
  glyph_id: u16,

  /// Pixel size the glyph was rasterized at. Carried for cache
  /// invalidation; a bitmap rasterized at 16.0 is not a valid
  /// hit for a 16.5 query.
  size: f32,
}
```

### Composition onto a target

To composite a `GlyphBitmap` onto a target pixel grid at the
glyph's baseline-origin `(ox, oy)`:

```
target_x_start = ox + bitmap.bearing_x
target_y_start = oy - bitmap.bearing_y   // y goes down in screen space

for row in 0 .. bitmap.height:
    for col in 0 .. bitmap.width:
        coverage = bitmap.pixels[row * bitmap.width + col] / 255
        pixel_rgba = premultiply(fill_color, coverage)
        target[target_x_start + col, target_y_start + row]
            = blend_over(target[..], pixel_rgba)
```

This is the canonical alpha-over composite. The paint backend's
glyph_run handler implements this loop; FNT03 does not.

### Why grayscale instead of RGBA

Two reasons:

1. **Reusability.** The same rasterized bitmap is valid for every
   fill color. A layout with 10,000 glyphs in 5 colors needs 2,000
   rasterizations, not 10,000.
2. **Cache locality.** A glyph-bitmap cache keyed on
   `(glyph_id, size)` hits far more often than one keyed on
   `(glyph_id, size, fill_color)`.

Production rasterizers uniformly use this approach (FreeType
`FT_LOAD_RENDER` returns grayscale; CoreGraphics returns per-
pixel alpha when you ask for a bitmap context with no color
channels).

---

## Rasterization algorithm

FNT03 does not prescribe a single algorithm. The interface is
what matters. Any implementation that produces a conforming
`GlyphBitmap` from a conforming `GlyphOutline` is valid.

This section describes **the reference algorithm** — the one the
first implementation SHOULD use, and the one test fixtures are
keyed against for expected coverage values.

### Reference algorithm — analytical edge coverage

The algorithm used by Skia, cairo, and (in concept) FreeType's
smooth rasterizer:

1. **Flatten** all `QuadTo` commands into sequences of `LineTo`
   segments using adaptive subdivision. Subdivide until each
   segment's deviation from the true curve is below **0.25
   pixels** (1/4-pixel flatness tolerance). This is the industry
   default for smooth text at display sizes.

2. **Build an edge table.** For each line segment, compute the
   top and bottom scanline rows it touches, and record:
   `(top_y, bottom_y, x_at_top, slope_dx_dy, direction)`.
   `direction` is `+1` for segments going down, `-1` for up —
   this encodes the non-zero winding contribution.

3. **Walk scanlines top-to-bottom.** For each scanline `y`:
   a. Add edges that start at this scanline to the active edge
      list.
   b. For each pixel column `x`, compute the signed-area
      contribution of each active edge clipped to the pixel's
      unit square. Sum the contributions; the absolute value is
      the pixel's coverage (clamped to 1.0).
   c. Advance active edges (`x += slope_dx_dy`).
   d. Remove edges whose `bottom_y <= y + 1`.

4. **Emit `GlyphBitmap`.** Multiply coverage by 255, clamp to
   `0..=255`, pack into a `Vec<u8>`.

### Subpixel origin and padding

Glyph origins are in user-space units at the caller's size. A
glyph whose baseline origin is at `(123.7, 456.3)` has its
outline pre-offset by that fractional amount before
scan-conversion:

```
outline_x_pixel = outline_x_design * (size / units_per_em) + 0.7
outline_y_pixel = outline_y_design * (size / units_per_em) + 0.3
```

(The 0.3 is in outline coordinates; the bitmap's top-left is at
integer screen coordinate `y=456`, with sub-row offset baked
into the coverage values.)

The rasterizer pads the bitmap by at least 1 pixel on each side
of the glyph's integer-rounded bounding box to give room for
anti-aliased edges that bleed past the nominal bounds. The
final `width`, `height`, `bearing_x`, `bearing_y` are set
accordingly.

### Flatness tolerance is a quality knob

`0.25 pixels` is the default. Callers MAY tune:
- Lower (tighter) flatness → more segments, slower, crisper at
  high zoom
- Higher (looser) flatness → fewer segments, faster, visible
  faceting at high zoom

FNT03 does not expose this in the public API for v1 — the
default is baked in. A future revision can add
`RasterizeOptions { flatness: f32 }`.

---

## Public API

```
fn new_rasterizer() -> Rasterizer

fn rasterize(
    rasterizer:     &Rasterizer,
    outline:        &GlyphOutline,
    units_per_em:   u32,
    size:           f32,            // target pixel size
    subpixel_origin: (f32, f32),   // fractional part of baseline origin
) -> Result<GlyphBitmap, RasterizeError>
```

Notes:

- `units_per_em` is supplied separately because `GlyphOutline`
  doesn't carry it (FNT02's outline is in raw design units). The
  caller passes the value from FNT00's `font_metrics`.
- `subpixel_origin` is typically `(origin_x % 1.0, origin_y %
  1.0)`. Implementations MAY cache by rounded subpixel origin
  (e.g., snap to 1/4-pixel increments) to increase cache hits.

### Errors

```
RasterizeError {
    EmptyOutline,              // GlyphOutline has no contours
                               // (caller should handle this without
                               // calling rasterize — a .notdef or
                               // space glyph has no ink)
    MalformedOutline,          // Contour ends mid-QuadTo without
                               // a closing point, or other
                               // algorithmic impossibility
    BitmapTooLarge,            // Bounding box would produce a
                               // bitmap larger than a configurable
                               // cap (default: 4096 × 4096 pixels)
}
```

All errors are programming / input-integrity errors, not runtime
data errors. `EmptyOutline` is the only one callers should
actively branch on (it's common for whitespace glyphs); the others
indicate a bug upstream in FNT02.

### BitmapTooLarge — the DoS guard

A pathological font could declare a glyph with a bounding box
declaring `xMin=-10000, xMax=10000, yMin=-10000, yMax=10000`.
Rasterized at 16px with `units_per_em=1000`, that's a 320×320
pixel bitmap — fine. At 1024px it's 20480×20480 — 400 MB at
1 byte per pixel. Not fine.

FNT03 imposes a hard cap on bitmap dimensions: **4096×4096 pixels
by default** (configurable via `Rasterizer::with_max_bitmap_size`).
Requests that would exceed this return `BitmapTooLarge` before
any allocation happens.

---

## Coverage vs. gamma

Coverage values are **linear**, not gamma-corrected. This is a
deliberate choice.

If coverage were gamma-encoded (sRGB), compositing would require
a gamma-decode round-trip in the blender, and the coverage
fractions would not obey alpha-blending math. Text renderers
that don't distinguish have the "thin on white, thick on black"
problem — at half coverage the glyph looks too bold on dark
backgrounds and too thin on light ones.

**FNT03 output: linear coverage.** Paint backends composite in
linear space and convert to sRGB (or Display P3, or whatever the
target colour space is) at the final composite boundary. This
matches how Skia, Blink, WebKit, and Metal all handle text
compositing correctly.

Consumers that insist on pre-gamma-encoded coverage (wrong for
most backends, but required for some legacy pipelines) apply
their own gamma curve to the output. FNT03 does not do this for
them.

---

## Package layout

One package per supported language:

```
glyph-rasterizer    (TypeScript, Python, Ruby, Go, Perl, Lua,
                     Haskell, Swift, C#, F#, Elixir, Rust, Kotlin, Java)
```

Each package:

- Depends on that language's `glyph-parser` (FNT02) for the
  `GlyphOutline` input type.
- Exposes the `rasterize` function with language-idiomatic
  naming.
- Optionally exposes a `GlyphCache` helper that memoizes
  `(glyph_id, size, subpixel_origin)` keys (rounded to 1/4-pixel
  subpixel increments) to avoid redundant work in layout-heavy
  paths.

### Rust reference signature

```rust
pub struct Rasterizer {
    max_bitmap_pixels: u32,  // default 4096 * 4096
    flatness:          f32,  // default 0.25
}

impl Rasterizer {
    pub fn new() -> Self;
    pub fn with_max_bitmap_size(mut self, max_px: u32) -> Self;

    pub fn rasterize(
        &self,
        outline:         &GlyphOutline,
        units_per_em:    u32,
        size:            f32,
        subpixel_origin: (f32, f32),
    ) -> Result<GlyphBitmap, RasterizeError>;
}
```

Other languages follow the same shape. Performance expectations
(Rust baseline): rasterizing a typical 16px Latin glyph in
under 50 microseconds, with negligible allocation.

### Thread safety

A `Rasterizer` is **stateless** — reusable across threads without
locking. A `GlyphCache` (optional helper) is explicitly
per-thread unless the language's package documents otherwise.
Sharing a cache across threads requires a lock around the
internal map.

---

## Testing strategy

Every FNT03 package MUST include:

1. **Zero-input sanity.** An empty outline returns
   `EmptyOutline`; callers SHOULD check for empty outlines before
   calling, but the error case is tested for robustness.

2. **FNT05 glyph reference bitmaps.** For each Tier-A glyph in
   FNT05 at 16px with integer-pixel origin, rasterize and compare
   the output bitmap byte-for-byte with a committed reference.
   FNT05's pixel-aligned design makes these bitmaps deterministic
   down to the last byte.

3. **Subpixel positioning stability.** Rasterizing the same glyph
   at the same size with `subpixel_origin = (0.0, 0.0)` vs
   `(0.5, 0.5)` produces different bitmaps with different
   `bearing_x` / `bearing_y` but the same total coverage mass
   (sum of pixels within ~1% tolerance).

4. **Coverage linearity.** Rasterizing a simple rectangle glyph
   at 16px shows interior pixels at coverage 255 and edge pixels
   at coverages that reflect actual partial coverage (not
   gamma-encoded). Asserts coverage of a 1-pixel-thick 50%-tilted
   edge is ~127, not ~188 (the gamma-encoded value).

5. **Non-zero winding with nested contours.** A glyph with a
   hole (e.g., FNT05's `O`) produces zero coverage in the hole
   interior.

6. **Quad-flattening correctness.** A glyph with a shallow curve
   (far from any straight approximation) rasterizes without
   visible faceting at 16px (subjective — verified by snapshot
   comparison to the FNT05 reference).

7. **BitmapTooLarge guard.** A rasterizer with
   `max_bitmap_pixels = 100` refuses to rasterize a glyph whose
   bounding box would produce a 200×200 bitmap.

8. **Performance regression fence.** A benchmark that rasterizes
   the full FNT05 Tier-A glyph set at 16px completes in under
   10 ms on a reference machine. Advisory in CI, not blocking.

Coverage target: **90%+**.

### Cross-language conformance

The same input `GlyphOutline` passed through every language's
FNT03 port MUST produce **identical byte-for-byte** bitmaps when
the reference algorithm is followed (flatness 0.25, analytical
edge coverage, linear output). Divergence indicates a bug in
that language's port.

Implementations using alternative algorithms (super-sampling
instead of analytical) MAY produce bitmaps that differ by small
absolute pixel values but must be visually equivalent; their
conformance tests compare with a perceptual tolerance (PSNR
threshold) rather than exact equality.

The reference implementation (Rust) uses the analytical
algorithm and is the baseline.

---

## Non-goals (recap)

- Hinting / grid-fitting
- LCD subpixel rendering
- Color glyphs (COLR/CPAL/sbix/SVG)
- Emoji
- GPU rasterization (separate tessellator spec, P2D08-adjacent)
- Outline stroking
- Variable-font glyph deltas (FNT02 already stripped)

---

## Relationship to sibling specs

| Spec  | Relationship                                                                           |
|-------|----------------------------------------------------------------------------------------|
| FNT00 | Transitive upstream — the font-parser provided the metrics (`units_per_em`)            |
| FNT02 | **Direct upstream.** FNT03 consumes `GlyphOutline` structures from the glyph parser.   |
| FNT04 | Sibling (Knuth-Plass layout) — not dependent on FNT03                                  |
| FNT05 | **Primary test fixture.** FNT05 glyphs at 16px integer-origin are the byte-reference.  |
| TXT02 | Transitive upstream. TXT02 emits glyph IDs → FNT02 outlines → FNT03 bitmaps.           |
| P2D00 | Downstream consumer. A paint backend's `glyph_run` handler calls FNT03, composites the result onto its target surface using `PaintGlyphRun.fill` as the tint. |
| P2D05+| Each native paint backend that uses the device-independent path (as opposed to CoreText / DirectWrite / Pango) consumes FNT03 output. |

### The full device-independent pipeline

```
Author text:
  "Hello world"
    │
    ▼  layout engine
CommonMark / document AST
    │
    ▼  FontResolver (TXT05)
FontHandle (font-parser-bound)
    │
    ▼  TextShaper (TXT02) + FontMetrics (TXT01)
ShapedRun: [glyph_id, x_offset, y_offset, x_advance] × N
    │
    ▼  layout positions each glyph
PaintGlyphRun (P2D00):
  kind: "glyph_run",
  baseline_origin: (ox, oy),
  font_ref: "font-parser:<hash>",
  glyphs: [...]
    │
    ▼  paint backend's glyph_run handler
for each glyph:
    outline  = FNT02.glyph_outline(glyph_id)
    bitmap   = FNT03.rasterize(outline, units_per_em, size,
                               subpixel_origin)
    composite_over_target(target, bitmap, ox, oy, fill_color)
    │
    ▼
pixels
```

With FNT00 + FNT02 + FNT03 + TXT00 + TXT01 + TXT02 + TXT05 all
specified, **every spec needed to render basic Markdown through
the device-independent path exists**. The remaining work is pure
implementation, plus two small amendments (UI04 reconciling its
text description with P2D00, and P2D05+ describing the
glyph-run routing by `font_ref` prefix).

For the device-dependent path (CoreText, DirectWrite, Pango),
FNT03 is NOT on the critical path — those backends rasterize
glyphs through OS APIs. A cross-platform consumer can freely mix
and match (device-indep shaper + CoreText rasterizer is
NOT legal due to the font-binding invariant; same binding from
shape through rasterize).

---

## Open questions

- **Subpixel origin snapping policy for caches.** Snapping all
  subpixel origins to 1/4-pixel increments gives up to 16 unique
  bitmaps per (glyph, size) pair. 1/8-pixel snapping gives 64.
  The right tradeoff depends on how much text is animated
  (scrolling, typing). FNT03's public API accepts arbitrary
  floats; the optional `GlyphCache` does the snapping. The
  snapping grid is an open question — propose 1/4 as the default.

- **Whether to expose "simple" vs "analytical" rasterization as
  a mode.** Some consumers (CI tests with time budgets) may
  prefer a faster super-sampled algorithm even at quality cost.
  The current spec says "pick one algorithm, document it", but
  a `RasterizationMode::Fast` enum value could be added. Deferred
  until a real consumer asks.

- **PixelContainer integration.** P2D00 defines a `PixelContainer`
  for full-color raster output. A `GlyphBitmap` is single-channel,
  so it deliberately doesn't use `PixelContainer`. If P2D00's
  types are renamed in a future cleanup (possibility flagged
  separately), FNT03's naming should be revisited for consistency.
  Currently `GlyphBitmap` stands alone.

- **Whether to rasterize in RGBA by default.** Some consumers find
  grayscale-plus-tint surprising. But RGBA-by-default wastes 4× the
  memory and defeats cache reuse. Keeping grayscale, documenting
  the composite formula, and providing an example helper is the
  right call.
