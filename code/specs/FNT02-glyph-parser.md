# FNT02 — Glyph Parser

## Overview

FNT02 defines a TrueType **glyph outline parser** for the
coding-adventures monorepo. It sits between FNT00 (the metrics-only
parser) and FNT03 (the rasterizer):

```
Font bytes
    │
    ▼
FNT00 (font-parser)          ─ parses header, metrics, cmap, hmtx, kern
    │
    ▼                            ShapedRun with glyph IDs
TXT02 / TXT04 shapers          ←────────
    │
    ▼  glyph_id
FNT02 (glyph-parser)         ─ reads `glyf` + `loca`, returns GlyphOutline
    │
    ▼  GlyphOutline
FNT03 (glyph-rasterizer)     ─ scan-converts outline to pixels
    │
    ▼
pixel grid
```

The glyph parser answers one question:

```
Given a parsed font and a glyph ID, what is the vector outline
I need to draw that glyph?
```

It does NOT answer:

- "What glyph ID does codepoint U+0041 map to?" (FNT00, cmap)
- "How far does the pen advance after drawing this glyph?" (FNT00, hmtx)
- "What pixels does this outline rasterize to at 16px?" (FNT03)
- "Is this glyph composed of multiple other glyphs?" (FNT02 handles
  this internally; the output is always a flat outline)

FNT02's scope is **pure TrueType outline extraction**, producing a
backend-agnostic `GlyphOutline` ready to feed to any rasterizer —
FNT03, a GPU tessellator, a path renderer, a CoreGraphics `CGPath`.

---

## Scope and explicit non-goals

### What FNT02 handles

- Simple TrueType glyphs (`numberOfContours >= 0` in the `glyf`
  table).
- Composite TrueType glyphs (`numberOfContours < 0`), flattened to
  a single outline before output.
- Quadratic Bezier contours (native TrueType format).
- The `loca` table in both short (16-bit) and long (32-bit) offset
  formats, selected via `head.indexToLocFormat`.
- Conversion of TrueType's implicit on-curve points into explicit
  `move_to` / `line_to` / `quad_to` commands.
- Bounding-box reporting for each glyph (from the glyph's header).

### What FNT02 does NOT handle

- **CFF / PostScript outlines.** Fonts with `sfntVersion ==
  0x4F54544F` ("OTTO") contain a `CFF` table instead of `glyf`.
  FNT02 v1 rejects these fonts with `UnsupportedFontFormatError`.
  A future FNT02b or separate `cff-parser` package can add them.
- **Cubic Bezier output.** The output always uses quadratic
  segments (native TrueType). Consumers that need cubics (Metal's
  `MTLPath`, some SVG paths) convert as a post-processing step.
  This conversion is described in G2D02 (not this spec).
- **Glyph hinting.** The `cvt`, `fpgm`, `prep`, and instruction
  streams inside simple glyphs are parsed past but not executed.
  Hinting is a grid-fitting step for low-resolution display; at
  modern display densities it matters less, and at test fixture
  sizes (16px and up on integer grids) it's unnecessary.
- **Color glyphs** (`COLR`, `CPAL`, `sbix`, `SVG`). A color glyph
  is treated as its fallback black outline. Color layer
  compositing is deferred to a future spec (FNT06 or similar).
- **Variable fonts.** A variable font's `gvar` table describes
  per-axis deltas. FNT02 v1 does NOT apply them; outlines come
  from the default instance only. Callers needing a specific
  instance must either pre-instance the font file or use a
  variable-font-aware resolver (TXT05 future).
- **Glyph substitution or positioning.** GSUB and GPOS are the
  shaper's territory (TXT02, TXT04). By the time FNT02 is called,
  the shaper has already decided which glyph ID to ask for.

---

## The GlyphOutline type

```
GlyphOutline {
  /// The glyph's design-space bounding box, in font design units.
  /// Copied verbatim from the glyph header (xMin, yMin, xMax, yMax).
  /// May not be tight — the TrueType format does not require it.
  bounds: BoundingBox { x_min, y_min, x_max, y_max : i16 },

  /// Ordered list of contours. Each contour is a closed subpath.
  /// Order matters for fill-rule resolution (nonzero winding):
  /// inner contours ("holes") must wind opposite the outer contour.
  contours: Vec<Contour>,

  /// The glyph ID this outline was generated from. Carried through
  /// for debugging and for sanity-checks in the rasterizer.
  glyph_id: u16,
}

Contour {
  /// Ordered list of drawing commands. Every contour begins with
  /// exactly one MoveTo and ends implicitly at the same point
  /// (contours are always closed in TrueType).
  ///
  /// TrueType outlines use ONLY quadratic segments. Line segments
  /// emitted here correspond to consecutive on-curve points in
  /// the source data (no off-curve point between them).
  commands: Vec<Command>,
}

Command {
  MoveTo  { x: i16, y: i16 },
  LineTo  { x: i16, y: i16 },
  QuadTo  { cx: i16, cy: i16, x: i16, y: i16 },
  // Quadratic Bezier: one control point (cx, cy) and one endpoint.
  // No CubicTo — TrueType does not produce cubics natively.
}
```

All coordinates are in **font design units** — the same coordinate
space as FNT00's metrics. To render at font size `N` pixels with
`units_per_em = U`, multiply every coordinate by `N / U`. This
scaling is the rasterizer's job, not the parser's.

### Why not match P2D00's PathCommand?

P2D00's `PathCommand` union supports cubic Bezier, elliptical
arcs, and close commands. FNT02's `Command` is narrower on
purpose — it can only represent what TrueType actually stores.
The output is pure; no conversion losses.

Consumers that need a `PaintPath` (for a P2D00 PaintGlyphRun
rendered via the path-based fallback) convert FNT02's output
by mapping `LineTo → line_to`, `QuadTo → quad_to`, inserting a
final `close` after each contour. This is a five-line loop,
not a library.

---

## Algorithm — simple glyphs

A simple glyph (`numberOfContours >= 0`) lives at
`glyf_start + loca_offset`, where:

- `glyf_start` is the `glyf` table offset in the file (from
  FNT00's table record lookup).
- `loca_offset` is looked up in the `loca` table at index
  `glyph_id`.

### Step 1 — Locate the glyph via `loca`

```
if head.indexToLocFormat == 0:
    // Short offsets: u16 values, each stored as offset / 2
    loca_offset = u16be(loca[glyph_id * 2]) * 2
else:
    // Long offsets: u32 values, raw byte offsets
    loca_offset = u32be(loca[glyph_id * 4])
```

If `loca_offset == loca_offset_of_next` (next glyph starts at the
same byte), the glyph is **empty** — return a `GlyphOutline` with
no contours. The `.notdef` glyph and the glyph for the space
character are commonly empty (space has zero ink).

### Step 2 — Parse the glyph header

At `glyf_start + loca_offset`:

| Field             | Type | Offset | Description                                 |
|-------------------|------|--------|---------------------------------------------|
| `numberOfContours`| i16  |  0     | Number of contours. Negative = composite.   |
| `xMin`            | i16  |  2     | Bounding box left edge, design units        |
| `yMin`            | i16  |  4     | Bounding box bottom (positive = up)         |
| `xMax`            | i16  |  6     | Bounding box right edge                     |
| `yMax`            | i16  |  8     | Bounding box top                            |

If `numberOfContours < 0`, go to the composite-glyph algorithm
below. Otherwise continue.

### Step 3 — Read the simple glyph body

Starting at offset 10 from the glyph header:

| Field                  | Type             | Size                        |
|------------------------|------------------|-----------------------------|
| `endPtsOfContours[N]`  | u16 × N          | 2 × numberOfContours        |
| `instructionLength`    | u16              | 2                           |
| `instructions[I]`      | u8 × I           | instructionLength           |
| `flags[P]`             | u8 × P           | variable (see below)        |
| `xCoordinates[P]`      | u8 or i16 × P    | variable (see below)        |
| `yCoordinates[P]`      | u8 or i16 × P    | variable (see below)        |

Where `P = endPtsOfContours[N-1] + 1` is the total point count.

### Step 4 — Decode the `flags[]` array

Flags are **run-length encoded**. Each byte is a bitmask:

| Bit | Name              | Meaning                                               |
|-----|-------------------|-------------------------------------------------------|
| 0   | `ON_CURVE_POINT`  | This point is on the curve (not a Bezier control)     |
| 1   | `X_SHORT_VECTOR`  | x-coord is 1 byte, not 2                              |
| 2   | `Y_SHORT_VECTOR`  | y-coord is 1 byte, not 2                              |
| 3   | `REPEAT_FLAG`     | Next byte is a repeat count; repeat this flag N times |
| 4   | `X_IS_SAME_OR_POSITIVE_SHORT` | When X_SHORT: sign bit for the short value |
| 5   | `Y_IS_SAME_OR_POSITIVE_SHORT` | When Y_SHORT: sign bit                    |
| 6   | reserved          | must be 0                                             |
| 7   | `OVERLAP_SIMPLE`  | Hinting-related; FNT02 ignores                        |

Decode loop:

```
i = 0
flags = []
while flags.len() < P:
    byte = glyf[offset + i]; i += 1
    flags.push(byte)
    if byte & REPEAT_FLAG:
        count = glyf[offset + i]; i += 1
        for _ in 0..count:
            flags.push(byte)
```

The REPEAT_FLAG is a size optimization — most fonts have long
runs of identical flag bytes (consecutive on-curve points with
the same coordinate encoding).

### Step 5 — Decode the coordinate arrays

Each coordinate is **delta-encoded** relative to the previous
point. The first point's delta is relative to (0, 0) — so the
running sum yields absolute coordinates.

For each point i:

```
if flags[i] & X_SHORT_VECTOR:
    dx_bytes = 1 (unsigned u8)
    dx_signed = dx_bytes if (flags[i] & X_IS_SAME_OR_POSITIVE_SHORT)
                else -dx_bytes
else:
    if flags[i] & X_IS_SAME_OR_POSITIVE_SHORT:
        dx_signed = 0                       // "same as previous"
    else:
        dx_signed = i16be(next two bytes)   // signed 16-bit delta

// Same logic for Y with Y_SHORT_VECTOR and Y_IS_SAME_OR_POSITIVE_SHORT

x[i] = x[i-1] + dx_signed
y[i] = y[i-1] + dy_signed
```

The three cases (1-byte unsigned, 0-byte "same as previous",
2-byte signed) are a size optimization. Simple glyphs are mostly
straight lines and small deltas, so the 1-byte path dominates.

### Step 6 — Assemble contours

Given the decoded points and the `endPtsOfContours[]` array:

```
start = 0
for ci in 0..numberOfContours:
    end = endPtsOfContours[ci]
    contour_points = points[start..=end]
    emit_contour(contour_points, flags[start..=end])
    start = end + 1
```

### Step 7 — Emit commands for one contour

This is where TrueType's implied on-curve logic becomes explicit.
The input is a sequence of (point, on_curve) pairs, forming a
closed loop (the first and last are adjacent).

Rules:
1. If the first point is off-curve, shift the contour so it
   starts on-curve. If NO point is on-curve (all off), synthesize
   a starting on-curve point at the midpoint of the first and
   last off-curve points.
2. Emit `MoveTo(first_on_curve)`.
3. Walk the remaining points. For each:
   - If on-curve and previous was on-curve: `LineTo(this)`.
   - If on-curve and previous was off-curve: close out the pending
     quad with `QuadTo(prev_off, this)`.
   - If off-curve and previous was on-curve: buffer this as the
     control point for an upcoming quad.
   - If off-curve and previous was off-curve: an implied on-curve
     point exists at the midpoint. Emit
     `QuadTo(prev_off, midpoint)` and then buffer this new off-curve.
4. After the last point, close the contour by connecting back to
   the first point using the same rules as step 3.

The TrueType implied-midpoint rule is what makes the format
compact — two off-curve points imply an on-curve point between
them, which most glyphs take advantage of repeatedly. FNT02
**always** materializes these implied points, so the rasterizer
never sees consecutive off-curve flags.

### Complexity

Parsing a simple glyph is linear in the number of points: one
pass for flags decoding, one pass for coordinates, one pass for
command emission. No backtracking, no recursion. Memory
allocation is bounded by the glyph's point count (tens to low
hundreds for typical Latin letters).

---

## Algorithm — composite glyphs

A composite glyph (`numberOfContours < 0`) is a list of
**component references**: "glyph 42 transformed by matrix M
placed at (x, y), then glyph 17 transformed by matrix N placed at
(x', y'), ..."

Composite glyphs are common for:
- Accented Latin: `á` = `a` + `´` at the correct position
- Ligatures shared across weights
- Roman numerals built from base letters

Flattening: FNT02 resolves each component recursively and emits
a single flat `GlyphOutline` with all contours concatenated.
The caller never sees composite structure.

### Composite record format

After the glyph header (still 10 bytes), the body contains one
or more component records. Each record:

| Field            | Type | Offset | Description                                   |
|------------------|------|--------|-----------------------------------------------|
| `flags`          | u16  | 0      | Component flags                               |
| `glyphIndex`     | u16  | 2      | Glyph ID of the component                     |
| (arg1, arg2)     | various | 4   | Offset or anchor point index                  |
| (scale/rotation) | various | varies | Transform matrix                              |

Flag bits that matter for FNT02:

| Bit | Name                        | Meaning                                        |
|-----|------------------------------|------------------------------------------------|
| 0   | `ARG_1_AND_2_ARE_WORDS`      | args are i16 each (else i8)                    |
| 1   | `ARGS_ARE_XY_VALUES`         | args are x/y offsets (else anchor point IDs)   |
| 3   | `WE_HAVE_A_SCALE`            | Single uniform scale follows                   |
| 6   | `WE_HAVE_AN_X_AND_Y_SCALE`   | Two scales (x and y) follow                    |
| 7   | `WE_HAVE_A_TWO_BY_TWO`       | Full 2×2 matrix follows                        |
| 5   | `MORE_COMPONENTS`            | Another component record follows this one      |

FNT02 handles all four transform variants by normalizing to a 2×3
affine matrix:

```
[ a  b  tx ]
[ c  d  ty ]
[ 0  0   1 ]
```

Applied to each point `(x, y)` of the component as:

```
x' = a*x + b*y + tx
y' = c*x + d*y + ty
```

Anchor-point matching (`!ARGS_ARE_XY_VALUES`) is a rarely-used
feature where the component's placement is determined by aligning
a point in the parent's outline with a point in the component's
outline. FNT02 v1 implements it; most fonts never exercise it.

### Recursion depth limit

To protect against malformed or maliciously-deep composite chains,
FNT02 imposes a hard recursion limit: a composite may reference
other composites up to **10 levels deep**. Beyond that, return
`CompositeDepthExceededError`. The TrueType spec does not
define a limit, but real fonts never exceed 2–3 levels.

---

## Public API

```
// Create a glyph parser from a parsed font (FNT00 FontFile).
fn new_glyph_parser(font: &FontFile) -> Result<GlyphParser, FontError>

// Look up a glyph outline by glyph ID. Returns a flat outline
// regardless of whether the source glyph is simple or composite.
// Returns Ok(None) if the glyph ID is out of range.
// Returns Ok(Some(empty outline)) if the glyph has zero contours.
fn glyph_outline(parser: &GlyphParser, glyph_id: u16)
    -> Result<Option<GlyphOutline>, GlyphError>

// (Optional) Batch lookup for efficiency when rasterizing many
// glyphs. Implementations MAY share parse state across calls.
fn glyph_outlines(parser: &GlyphParser, ids: &[u16])
    -> Vec<Result<Option<GlyphOutline>, GlyphError>>
```

### Errors

```
GlyphError {
    UnsupportedFontFormat,            // CFF font, not TrueType
    GlyphIndexOutOfRange(u16),        // id >= numGlyphs
    MalformedContour,                 // points inconsistent with endPts
    MalformedComposite,               // composite record truncated
    CompositeDepthExceeded,           // > 10 levels of composite nesting
    MissingTable(&'static str),       // e.g. "glyf" not present
    InvalidFlagRun,                   // REPEAT_FLAG past end of flags
}
```

All of these are **programming or file-integrity errors**, not
runtime data errors. They indicate a malformed font file or a bug
in the parser — not something a caller recovers from by retrying.

---

## Relationship to FNT00

FNT02 **depends on FNT00** for parsing the font's header and
table directory. The expected setup:

```
1. Call font_parser::load(bytes) → FontFile           // FNT00
2. Call glyph_parser::new(&FontFile) → GlyphParser    // FNT02
3. For each glyph_id in a ShapedRun:
    outline = glyph_parser::glyph_outline(&GlyphParser, glyph_id)
    // pass to rasterizer
```

FNT02 does NOT re-parse the file. It reads table offsets from
FNT00's `FontFile` and indexes into the byte slice. No duplicate
work.

FNT02 requires these tables (checked at `new_glyph_parser` time):
- `head` (for `indexToLocFormat`)
- `maxp` (for `numGlyphs` bounds check)
- `loca`
- `glyf`

If any is missing, `MissingTable` is returned immediately. A
legitimate bitmap-only font (`sbix`/`EBDT`/`CBDT`, no outlines)
would trigger this; such fonts are outside FNT02 v1 scope.

---

## Package layout

One package per supported language:

```
glyph-parser    (TypeScript, Python, Ruby, Go, Perl, Lua, Haskell,
                 Swift, C#, F#, Elixir, Rust, Kotlin, Java)
```

Each package:
- Depends on that language's `font-parser` (FNT00).
- Exposes the three-function API above (language-idiomatic
  naming: `get_glyph_outline` in Python, `GlyphOutline` in Swift,
  etc.).
- Ships the same glyph-decoding algorithm. Cross-language
  conformance fixture: FNT05 test font's simple and composite
  glyphs, each with a pre-recorded expected `GlyphOutline`
  structure (committed as JSON in the fixture repo).

### Rust reference signature

```rust
pub struct GlyphParser<'a> {
    file: &'a font_parser::FontFile,
    loca_is_long: bool,
    num_glyphs: u16,
    glyf_offset: usize,
    glyf_length: usize,
    loca_offset: usize,
    loca_length: usize,
}

impl<'a> GlyphParser<'a> {
    pub fn new(file: &'a font_parser::FontFile)
        -> Result<GlyphParser<'a>, GlyphError>;

    pub fn glyph_outline(&self, glyph_id: u16)
        -> Result<Option<GlyphOutline>, GlyphError>;

    pub fn glyph_outlines(&self, ids: &[u16])
        -> Vec<Result<Option<GlyphOutline>, GlyphError>>;
}
```

Performance: no persistent cache inside GlyphParser v1. Callers
that rasterize the same glyph repeatedly should cache the
`GlyphOutline` themselves (it's cheap — typical Latin glyphs
have 20–50 commands). A future version may add an optional
internal LRU cache.

---

## Testing strategy

Every FNT02 package MUST include:

1. **Empty glyph.** A glyph with `endPtsOfContours[]` of length 0
   (numberOfContours == 0) produces an outline with
   `contours.is_empty()`. Tested against FNT05's space character.

2. **Simple glyph round-trip.** FNT05's `A` glyph parses to the
   committed reference `GlyphOutline` structure. Exact equality
   on commands + bounds.

3. **All tier-A glyphs.** Every printable ASCII character in FNT05
   parses without error and produces a non-empty outline.

4. **Delta encoding.** Synthesize a simple glyph with long runs
   of identical flags (to exercise REPEAT_FLAG); parse it; assert
   point count and coordinates match expectation.

5. **Short-vector coordinates.** Synthesize a glyph using all
   three coord encodings (1-byte, same-as-previous, 2-byte);
   parse it; assert correctness.

6. **Composite flattening.** Synthesize a composite "á" glyph
   referencing "a" and "´" components with a translation
   transform; parse it; assert the result is a flat outline
   containing the union of the components' contours at the
   correct positions.

7. **Composite with uniform scale.** Verify scale transform is
   applied correctly to component coordinates.

8. **Composite with 2×2 transform.** Verify rotation/shear
   matrices are applied correctly.

9. **Composite depth limit.** Synthesize a 12-level-deep
   composite chain; assert `CompositeDepthExceededError`.

10. **CFF rejection.** A font with `sfntVersion == 0x4F54544F`
    produces `UnsupportedFontFormatError` at
    `new_glyph_parser` time.

11. **Missing table.** A font with `glyf` removed produces
    `MissingTable("glyf")`.

12. **Out-of-range glyph ID.** `glyph_outline(num_glyphs)`
    returns `GlyphIndexOutOfRange`.

Coverage target: **95%+** of the parser branch space. FNT05 gives
us most of the common-case coverage; synthetic fixtures cover
the rare paths.

### Cross-language conformance

The same Inter Regular TTF (or an FNT05 build) parsed through
every language's FNT02 port MUST produce identical
`GlyphOutline` structures — same commands in the same order, same
coordinate values. A shared fixture JSON encodes these
expectations. Divergences indicate a bug in that language's port,
not in the spec.

---

## Relationship to sibling specs

| Spec  | Relationship                                                                            |
|-------|-----------------------------------------------------------------------------------------|
| FNT00 | **Upstream dependency.** FNT02 reads table offsets from FNT00's parsed `FontFile`.      |
| FNT01 | Sibling (text shaper — not yet defined). Shapers produce glyph IDs that FNT02 resolves.  |
| FNT03 | **Downstream.** Rasterizer consumes `GlyphOutline`. FNT02's output is FNT03's input.     |
| FNT04 | Sibling (Knuth-Plass layout). Not dependent on FNT02 directly.                           |
| FNT05 | **Primary test fixture.** FNT05's glyphs are the conformance corpus.                     |
| TXT02 | Transitive consumer via FNT03. TXT02 produces glyph IDs; the pipeline calls FNT02 for outlines. |
| P2D00 | `PaintGlyphRun.glyphs[].glyph_id` values are what FNT02 takes as input. The rasterizer bridges the gap. |

### The device-independent pipeline as of FNT02

```
FontQuery (TXT05)
    │
    ▼
FontFile (FNT00)
    │
    ▼
TextShaper.shape() → ShapedRun with glyph IDs    (TXT02 or TXT04)
    │
    ▼
Paint backend's glyph_run handler
    │
    ▼  for each glyph_id:
GlyphOutline  ←  FNT02 glyph_parser
    │
    ▼
pixel grid   ←  FNT03 glyph_rasterizer
    │
    ▼
composited PaintScene
```

With FNT02 specified, every arrow in the pipeline has either an
implementation (FNT00 exists) or a committed spec (the rest).
FNT03 is the last keystone before the device-independent path
can be implemented end to end.

---

## Non-goals (recap)

- CFF/PostScript outlines (separate future spec)
- Hinting (deferred indefinitely; modern DPI makes it less
  valuable and fixing-to-grid is a rasterizer concern anyway)
- Color glyphs (future spec)
- Variable fonts (`gvar` deltas) (future spec)
- GSUB/GPOS processing (shaper territory)
- Bitmap glyphs (`sbix`/`EBDT`/`CBDT`) (separate path, future spec)
- Caching strategy (caller's responsibility in v1)

---

## Open questions

- **Output coordinate type.** Currently `i16`, matching TrueType
  native. A future consumer (a tessellator operating in f32 NDC)
  might want a `f32`-version. Add a generic `<T: Scalar>` if the
  need arises; don't speculate now.

- **Whether to expose flag metadata alongside commands.** Bit 7
  (`OVERLAP_SIMPLE`) tells the rasterizer whether contours
  overlap intentionally. FNT02 currently drops it. FNT03 would
  need it if we implement accurate overlap handling. Deferred
  until FNT03 drafts.

- **Composite `USE_MY_METRICS` flag.** This flag (bit 9) tells a
  composite glyph to inherit its advance width from one of its
  components. It's relevant to `hmtx` lookups, not to outline
  geometry. Currently out of scope for FNT02; flag for follow-up.

- **Empty-contour handling in output.** A contour containing only
  off-curve points (theoretically malformed) is currently
  silently dropped. Should it error? Real fonts don't do this,
  but defensive parsers should decide a policy. Leaning toward
  "drop silently with a warning log"; finalize with the first
  implementation.
