# TXT00 — Text Interfaces: FontMetrics, TextShaper, TextMeasurer

## Overview

TXT00 defines the three **independent, pluggable interfaces** that sit between
the layout layer and the paint layer for all text rendering in the
coding-adventures stack.

```
Document producer (CommonMark parser, LaTeX parser, chart generator)
    │
    ▼
Layout engine  ─────────── asks about font globals ──▶  FontMetrics    (TXT00)
    │                ─────── asks to shape strings ───▶  TextShaper    (TXT00)
    │                ───── asks for a line width ─────▶  TextMeasurer  (TXT00)
    │
    ▼  emits positioned glyph runs
PaintScene (P2D00)  — contains PaintGlyphRun instructions
    │
    ▼
PaintVM (P2D01)  →  backend glyph rasterizer  →  pixels
```

The three interfaces are deliberately **orthogonal**. A consumer can mix and
match implementations: a `font-parser`-backed `FontMetrics`, a hand-written
HarfBuzz-style `TextShaper`, and the default `TextMeasurer`. Swapping one
does not force re-implementation of the others.

This separation is the central design commitment of this spec. Every
implementation package (TXT01, TXT02, TXT03, TXT04) targets exactly one of the
three interfaces. No package is allowed to bundle metrics retrieval with
shaping or shaping with measurement.

### What text is, in this stack

Text rendering is a four-stage pipeline. Each stage has distinct inputs,
outputs, and failure modes. This spec covers the first three stages; the
fourth is owned by the paint backends (P2D02–P2D05).

```
Stage 1 — FontMetrics         font handle       → global metrics (ascent, x-height, …)
Stage 2 — TextShaper          codepoints + font → positioned glyph run
Stage 3 — TextMeasurer        shaper output     → bounding box / line count
Stage 4 — Paint rasterizer    glyph run         → pixels
```

Only stage 2 (shaping) is computationally heavy and algorithmically subtle.
Stages 1 and 3 are cheap and mechanical. Splitting them into separate
interfaces prevents a "simple measurement" request from dragging in a full
shaping implementation, and conversely lets a complex shaper be written
without re-implementing metrics lookup.

---

## Why three interfaces and not one

A naive design would fold all three concerns into a single `TextEngine` trait
with `measure()`, `shape()`, and `metrics()` methods. This spec rejects that
design for three reasons:

1. **Shaping is pluggable, metrics is trivial.** Text shaping is an
   open-ended research problem (GSUB, GPOS, complex scripts, OpenType
   feature tuning). A contributor writing a new shaper should not have to
   re-derive `ascent` from the `hhea` table — that is a 20-line function
   that lives in `font-parser`. Keeping metrics in its own trait means every
   shaper gets metrics for free, and a metrics-only consumer (e.g., a
   line-height calculation in a layout engine) doesn't drag in a shaper.

2. **Measurement is usually a thin wrapper over shaping.** The width of a
   shaped line is the sum of its glyph advances. If measurement is its own
   trait, every implementation re-implements the same summation. If
   measurement is instead a function that *takes* a shaper, the logic is
   written once and every new shaper automatically gets measurement.

3. **Pluggability is asymmetric across the three.** Expect many
   `TextShaper` implementations (device-independent, CoreText, DirectWrite,
   Pango, and hand-rolled HarfBuzz-likes). Expect few `FontMetrics`
   implementations — font-parser is enough for most use cases. Expect zero
   third-party `TextMeasurer` implementations, since the default one (shape
   + sum advances) is correct by construction. Splitting the traits lets
   each interface evolve at its own rate.

---

## Interface 1: FontMetrics

```
FontMetrics {
  // All values are in the font's design units unless otherwise noted.
  // Divide by units_per_em and multiply by font_size to convert to user-space.

  units_per_em(font: FontHandle) → int
  // The font's design grid size. Common values: 1000 (PostScript), 2048
  // (TrueType), 1024 (some Asian fonts). A glyph 1000 units wide in a
  // font with units_per_em=2048 is half an em wide.

  ascent(font: FontHandle)      → int
  // Distance from baseline to the top of the tallest glyph (positive).
  // Used to compute the top of a line of text: line_top = baseline - ascent.

  descent(font: FontHandle)     → int
  // Distance from baseline to the bottom of the deepest descender
  // (returned as a NON-NEGATIVE integer; the sign convention is
  // "distance below baseline", not the signed value from the hhea table).

  line_gap(font: FontHandle)    → int
  // Recommended extra vertical space between consecutive lines of text.
  // Typical line height = (ascent + descent + line_gap) * font_size / units_per_em.

  x_height(font: FontHandle)    → int | null
  // Height of lowercase 'x' above the baseline. Null if the font does not
  // define it (older TrueType fonts without OS/2 table, or symbol fonts).

  cap_height(font: FontHandle)  → int | null
  // Height of a capital 'H' above the baseline. Null if not defined.

  family_name(font: FontHandle) → string
  // Human-readable family name from the font's `name` table.
  // Used for debug output, not for resolution. FontResolver (TXT05) owns
  // the mapping from name strings to FontHandle values.
}
```

### What goes in FontMetrics and what does not

FontMetrics is a **global** view of a font. Nothing in this interface takes
a string, a codepoint, or a glyph ID. Anything per-character is shaping
territory.

Specifically, the following are NOT in FontMetrics:

- **Glyph advance widths** — per-glyph, belongs in the shaper's output.
- **Kerning pair lookups** — per-glyph-pair, belongs in the shaper.
- **cmap lookup (codepoint → glyph_id)** — per-codepoint, belongs in the
  shaper.
- **Glyph bounding boxes** — per-glyph, belongs in glyph-parser (FNT02).
- **Glyph outlines** — per-glyph raster data, belongs in glyph-parser.

A consumer that only needs line height calls FontMetrics. A consumer that
needs to know how wide "Hello" is calls a TextShaper. This split keeps the
cheap path cheap.

### Why `FontHandle` and not a font file path

The `FontHandle` type is **implementation-defined** by each FontMetrics
backend. For the font-parser backend it is a parsed-font struct. For the
CoreText backend it is a `CTFontRef`. For the DirectWrite backend it is an
`IDWriteFont`. FontMetrics does not specify how handles are created; that
is FontResolver's job (TXT05).

The contract is: **a handle obtained from one backend's resolver is only
valid with that backend's FontMetrics and TextShaper**. This is enforced
by the font-binding invariant (see below).

---

## Interface 2: TextShaper

```
TextShaper {
  shape(
    text:     string,
    font:     FontHandle,
    size:     float,             // in user-space units (typically pixels)
    options:  ShapeOptions
  ) → ShapedText

  // The shaper MUST also expose the font_ref string that is safe to embed
  // in PaintGlyphRun.font_ref when the caller's FontHandle is the primary
  // font (no fallback needed). See "The font-binding invariant" below.
  font_ref(font: FontHandle) → string
}

ShapeOptions {
  script:    string | null
  // ISO-15924 script code, e.g. "Latn", "Arab", "Hans", "Deva".
  // Null = auto-detect from the text's dominant script.
  // Shapers that do not support a script (e.g., a Latin-only shaper asked
  // for "Arab") MUST throw UnsupportedScriptError, not silently fall back.

  language:  string | null
  // BCP-47 language tag, e.g. "en", "tr", "vi". Affects shaping for
  // language-specific OpenType features (e.g., Turkish dotted-i).
  // Null = no language hint; shapers use their default mapping.

  direction: "ltr" | "rtl" | "ttb" | "btt"
  // Explicit direction. Bidi resolution is NOT done by the shaper — the
  // caller must split mixed-direction text into runs before calling.

  features:  { [tag: string]: bool | int }
  // OpenType feature toggles. Keys are 4-character OT tags ("liga", "kern",
  // "smcp", "ss01", …). Values are bool (on/off) or int (alt index).
  // Shapers that do not understand a tag MUST ignore it silently.
  // Shapers that support a tag MUST default to the OpenType-recommended
  // default state when the tag is absent.
}

ShapedRun {
  glyphs: Array<{
    glyph_id:  int      // backend-specific glyph ID — only meaningful to
                        // the shaper that produced it AND to a rasterizer
                        // bound to the same font.
    cluster:   int      // byte offset into the original text string for
                        // the codepoint(s) this glyph represents. Used for
                        // cursor positioning and hit-testing.
    x_advance: float    // how far the pen moves after this glyph, in
                        // user-space units at the requested size.
    y_advance: float    // usually 0 for horizontal text, non-zero for
                        // vertical writing.
    x_offset:  float    // position adjustment for this glyph relative to
                        // the pen (e.g., for combining marks, kerning).
    y_offset:  float    // vertical offset (e.g., superscript, diacritics).
  }>;

  // Total advance width of the run, equal to sum of glyph x_advance values.
  // Pre-computed so consumers don't re-sum it.
  x_advance_total: float

  // The font_ref string for the font binding that produced these glyph IDs.
  // MUST be embedded verbatim in any PaintGlyphRun that renders this run.
  // Every glyph inside a ShapedRun is bound to the SAME font_ref — the
  // run is the unit of font-binding homogeneity.
  font_ref: string
}

// Output of `shape()`. A single shaping call can produce MULTIPLE
// ShapedRuns when the shaper's font-fallback kicks in: characters that
// are not present in the primary font get rendered with a secondary
// font, and the resulting glyph IDs belong to *that* secondary font.
// Treating them as if they came from the primary font would violate
// the font-binding invariant and produce wrong output at paint time.
//
// For Latin-only text rendered with a font that covers all input
// codepoints, `runs` contains exactly one element and everything is
// tagged with the caller's primary font_ref — the degenerate case.
//
// For mixed-script / mixed-symbol content, `runs` contains one entry
// per contiguous same-font segment, emitted in source order. The
// layout engine's pen advances across segments by summing each
// ShapedRun's x_advance_total.
ShapedText {
  runs: Array<ShapedRun>
}
```

### What the shaper does, step by step

A shaper implementation is free to use any algorithm it wants, but
conceptually the pipeline is:

```
1. Unicode normalization     (NFC by default; not the shaper's job to re-do)
2. Script/language tagging   (from ShapeOptions or auto-detection)
3. cmap lookup               (codepoint → glyph_id via font's cmap table)
4. Apply GSUB substitutions  (ligatures, contextual forms, stylistic sets)
5. Apply GPOS positioning    (kerning, mark positioning, cursive attachment)
6. Apply fallback kerning    (if no GPOS, use the legacy 'kern' table)
7. Segment output by font    (one ShapedRun per contiguous same-font span,
                              including any fallback font used for
                              codepoints missing from the primary font)
8. Return ShapedText         (Array of ShapedRuns in source order)
```

A **naive shaper** (TXT02) skips steps 4 and 5 entirely, emits one glyph per
codepoint via cmap, and optionally applies legacy `kern`. This is enough
for basic Latin rendering of CommonMark content and is the minimum viable
implementation.

A **full shaper** (TXT04, hand-rolled HarfBuzz-like) implements the entire
OpenType shaping pipeline. This spec does not prescribe the algorithm.
What it prescribes is the *interface* — any shaper that produces a valid
`ShapedRun` for a given input is a conforming implementation, regardless of
which OpenType features it supports or how it implements them.

### What the shaper does NOT do

- **Line breaking.** Shapers receive a single line (or a single
  directional run). Breaking text into lines that fit a width is the
  layout engine's job, using measurement in a feedback loop.
- **Bidi resolution.** The layout engine splits mixed-direction text into
  uniform-direction runs before calling the shaper. The Unicode Bidi
  Algorithm (UAX #9) is a separate concern.
- **Vertical metrics for line height.** That is FontMetrics.
- **Bidi resolution.** The layout engine splits mixed-direction text
  before calling.
- **Line breaking above the shaper level.** Shapers receive a single
  logical run; the layout engine decides where lines break.

### Font fallback (when the shaper supports it)

Shapers that integrate with an OS text stack (CoreText, DirectWrite,
Pango) automatically perform **font fallback**: when a codepoint in the
input isn't present in the caller's primary font, the shaper picks a
system fallback font that does cover it. The emitted glyph IDs belong
to that fallback font, NOT to the caller's primary font.

Because of this, `shape()` returns [`ShapedText`] — a sequence of one
or more [`ShapedRun`]s — rather than a single run. Each element of the
returned array is a contiguous span of glyphs that all share one
`font_ref`. Crossing from one `ShapedRun` to the next marks a
font-binding boundary.

For a shaper without font fallback (the naive TXT02 shaper, or a
TXT04 invocation on text fully covered by the primary font), the
output is a single-element `ShapedText` whose sole `ShapedRun` is
tagged with the caller's `FontHandle`'s `font_ref`. Producers that
emit `PaintGlyphRun` instructions MUST emit one `PaintGlyphRun` per
`ShapedRun` so each paint instruction preserves the binding invariant
from its segment's actual font.

Shapers that DO NOT support font fallback emit the font's `.notdef`
glyph (conventionally glyph_id 0) for any unmapped codepoint and
return a single `ShapedRun` tagged with the caller's `font_ref`.

---

## Interface 3: TextMeasurer

`TextMeasurer` is **not a parallel trait**. It is a concrete function that
wraps a `TextShaper` and returns a bounding box.

```
MeasureResult {
  width:      float   // total x-advance of the shaped run
  ascent:     float   // in user-space units at the requested size
  descent:    float   // in user-space units, non-negative
  line_count: int     // 1 for single-line; >1 if wrapped (see below)
}

measure(
  shaper:   TextShaper,
  metrics:  FontMetrics,
  text:     string,
  font:     FontHandle,
  size:     float,
  max_width: float | null,
  options:  ShapeOptions
) → MeasureResult
```

### The default implementation

```
function measure(shaper, metrics, text, font, size, max_width, options):
  if max_width is null:
    run = shaper.shape(text, font, size, options)
    units_per_em = metrics.units_per_em(font)
    scale = size / units_per_em
    return MeasureResult {
      width:      run.x_advance_total,
      ascent:     metrics.ascent(font)  * scale,
      descent:    metrics.descent(font) * scale,
      line_count: 1
    }

  // Wrapped path: shape the whole string, then walk glyphs to find
  // word-break opportunities that fit in max_width.
  run = shaper.shape(text, font, size, options)
  lines = greedy_wrap(run, max_width, text)  // splits on Unicode line break opportunities
  return MeasureResult {
    width:      max(line.x_advance_total for line in lines),
    ascent:     metrics.ascent(font)  * scale,
    descent:    (metrics.descent(font) + metrics.line_gap(font)) * scale * (len(lines) - 1)
                + metrics.descent(font) * scale,
    line_count: len(lines)
  }
```

`greedy_wrap` uses the `cluster` field in each glyph to map back to
codepoint positions and split on line-break opportunities (UAX #14).
Line breaking is described in more detail in TXT06 (future, optional).

### Why measurement is not a trait

A trait would imply there are meaningfully different ways to implement it.
There aren't. Every correct implementation does the same thing: call the
shaper, look at the total advance, optionally wrap, return the bounding
box. Making it a trait would invite incorrect re-implementations that
diverge from the shaper's view of the world (e.g., a measurer that sums
advance widths from the font's `hmtx` table without going through the
shaper, missing kerning and ligatures).

Consumers that need to customize measurement (e.g., to add tracking or
letter-spacing) should compose: call `shape()`, adjust advances, then
sum. They do not need a new trait for that.

---

## The font-binding invariant

This is the load-bearing rule of the entire text pipeline:

> Glyph IDs are **opaque tokens** that belong to the font binding that
> produced them. A `ShapedRun` produced by shaper A over font handle
> `f_A` can only be correctly rendered by a rasterizer that understands
> `f_A` through the same binding.

In concrete terms:

- Glyph IDs from a `font-parser`-backed shaper index into the font file's
  `glyf` / `CFF` tables. A rasterizer that wants to draw these glyphs
  must use `glyph-parser` (FNT02) to look up outlines in the same font
  file.
- Glyph IDs from CoreText are opaque `CGGlyph` values bound to a
  specific `CTFontRef`. They can only be rendered via CoreText (e.g.,
  `CTFontDrawGlyphs`) using the same `CTFontRef`.
- Glyph IDs from DirectWrite are bound to an `IDWriteFontFace`. They can
  only be rendered via `ID2D1RenderTarget::DrawGlyphRun` with the same
  font face.

Mixing bindings is undefined behavior. A `CGGlyph` value of 42 is not the
same glyph as a font-parser glyph ID of 42 in the same font file. They
happen to be derived from the same `glyf` table but they are routed
through different code paths and may differ (e.g., CoreText synthesizes
glyphs for characters missing from the font).

### How the invariant is enforced in practice

Every `ShapedRun` carries a `font_ref: string` field. Every
`PaintGlyphRun` instruction carries the same `font_ref` field (defined in
P2D00). Paint backends use `font_ref` as a routing key:

```
font_ref begins with "font-parser:" → use glyph-parser rasterization path
font_ref begins with "coretext:"    → use CoreText drawing path
font_ref begins with "directwrite:" → use DirectWrite drawing path
font_ref begins with "pango:"       → use Pango/FreeType drawing path
```

A paint backend that encounters an unknown `font_ref` scheme MUST throw
`UnsupportedFontBindingError`. This keeps wrong-binding bugs loud and
early — the same principle as P2D01's `UnknownInstructionError`.

The exact `font_ref` scheme strings are registered in TXT05 (FontResolver)
so that there is a single source of truth.

### Analogy

This is the same invariant as file descriptors: a file descriptor of 3
from process A is not meaningful to process B, even if both processes
opened the same file. The OS kernel treats each process's descriptor
table as a local namespace.

Glyph IDs are shaper-local descriptors into a font-binding-local glyph
table. They do not survive crossing the binding boundary.

---

## Relationship to P2D00 PaintGlyphRun

`PaintGlyphRun` (defined in P2D00) is the wire-format output of shaping.
Each `ShapedRun` in a `ShapedText` produces **one** `PaintGlyphRun`;
a ShapedText with N runs produces N PaintGlyphRuns emitted back to
back along the same baseline. The layout engine's pen advances across
segments by summing each run's `x_advance_total`.

```
ShapedText → PaintGlyphRun[] conversion (done by the layout engine):

pen = 0
for shaped_run in shaped_text.runs:
    emit PaintGlyphRun {
      kind:      "glyph_run",
      x:         baseline_origin_x + pen,   // advance across segments
      y:         baseline_origin_y,
      font_ref:  shaped_run.font_ref,       // propagated verbatim
                                            // — this may differ per
                                            // segment when font fallback
                                            // kicks in.
      font_size: size,                      // the size passed to shape()
      glyphs:    shaped_run.glyphs.map(g => ({
        glyph_id: g.glyph_id,
        x_offset: g.x_offset,               // relative to the segment's
                                            // start; layout engine adds
                                            // the cumulative pen.
        y_offset: g.y_offset
      })),
      fill:      color_from_layout
    }
    pen += shaped_run.x_advance_total
```

One subtlety: P2D00's `PaintGlyphRun.glyphs[i].x_offset` is the
**absolute x offset from the baseline origin**, while the shaper's
`ShapedRun.glyphs[i].x_offset` is a **per-glyph adjustment relative to
the running pen position**. The conversion walks the run accumulating
`x_advance` and adds `x_offset` to produce the baked absolute offset.
This conversion is done once at layout time, so the paint VM never has
to track a running pen.

---

## Relationship to UI09 (layout-text-measure)

UI09 defines an earlier, simpler `TextMeasurer` interface that predates
TXT00. TXT00 **supersedes UI09** as the canonical text interface spec.
Implementations should migrate.

Key differences:

| Aspect                | UI09 `TextMeasurer`        | TXT00 three-trait split               |
|-----------------------|----------------------------|---------------------------------------|
| Scope                 | Measurement only           | Metrics + shaping + measurement       |
| Output                | `{width, height, lineCount}` | `ShapedRun` (+ measurement wrapper)   |
| Glyph positions       | Not exposed                | Full per-glyph positions              |
| Pluggable shaping     | No                         | Yes — separate trait                  |
| Font-binding explicit | No                         | Yes — `font_ref` contract             |
| Feeds PaintGlyphRun   | Indirectly via UI04 rewrite | Directly — shaper output is the wire format |

The UI09 measurer packages (`layout-text-measure-estimated`,
`layout-text-measure-canvas`, `layout-text-measure-rs`) remain valid for
callers that only need an approximate bounding box. They will be
re-expressed over TXT00 in a follow-up:

- `layout-text-measure-estimated` → a degenerate `TextShaper` that emits
  one glyph per codepoint with constant advance, plus the default measurer.
- `layout-text-measure-canvas` → a `TextShaper` wrapping
  `ctx.measureText()`; does not expose real glyph positions but returns
  correct widths.
- `layout-text-measure-rs` → replaced by TXT01 + TXT02.

UI09 will be marked "superseded by TXT00" in a future doc-only PR. No
existing code is broken by this spec.

---

## Relationship to UI04 (layout-to-paint)

UI04's current text section describes a simplified `PaintGlyphRun` with
fields like `text`, `maxWidth`, and `align`. This is **inconsistent with
P2D00's actual `PaintGlyphRun`**, which uses pre-shaped glyph IDs.

UI04 will be amended in a follow-up to:

1. Stop carrying `text` in paint instructions — strings are layout-layer
   only, converted to shaped glyphs before the paint layer sees them.
2. Drop `maxWidth` and `align` — these are layout-time concerns that
   must be resolved before emitting paint instructions.
3. Wire in a `TextShaper` as a required parameter to the
   layout-to-paint conversion.

This amendment is deferred to keep TXT00 focused. It is tracked as part
of the roadmap below.

---

## Error conditions

| Error                         | When                                                                     | How to fix                                                   |
|-------------------------------|--------------------------------------------------------------------------|--------------------------------------------------------------|
| `UnsupportedScriptError`      | Shaper called with a script code it cannot handle                        | Use a shaper that supports the script, or split the run      |
| `UnsupportedFontBindingError` | Paint backend received a `PaintGlyphRun` whose `font_ref` scheme is unknown | Use a paint backend that understands the shaper's binding   |
| `FontResolutionError`         | FontResolver could not map an abstract font ref to a concrete handle     | Install the font, or choose a different family               |
| `InvalidFontHandleError`      | FontMetrics or TextShaper received a handle from a different backend     | Don't cross bindings — use handles from the matching resolver |
| `ShapingFailedError`          | Shaper failed internally (e.g., malformed font, out-of-memory)           | Bug report; fall back to a different shaper if available     |

All of these are recoverable at the layout level (the layout engine can
retry with a different font or shaper). None of them should be caught
and silently ignored.

---

## Roadmap — sibling specs

TXT00 is the interface. The following specs are the implementations and
the integration points:

| Spec  | Package                    | Description                                                                  |
|-------|----------------------------|------------------------------------------------------------------------------|
| TXT00 | (this spec)                | FontMetrics, TextShaper, TextMeasurer interfaces                             |
| TXT01 | `text-metrics-font-parser` | `FontMetrics` over `font-parser`. Reproducible, cross-platform.              |
| TXT02 | `text-shaper-naive`        | Minimal device-independent `TextShaper`: cmap + legacy `kern`, no GSUB/GPOS. |
| TXT03 | `text-shaper-native`       | Device-dependent `TextShaper` per OS. Split into sub-specs:                  |
|       | TXT03a                     | CoreText shaper (macOS/iOS)                                                  |
|       | TXT03b                     | DirectWrite shaper (Windows)                                                 |
|       | TXT03c                     | Pango/HarfBuzz shaper (Linux)                                                |
| TXT04 | `text-shaper-harfbuzz`     | Hand-rolled HarfBuzz-like shaper. Replaces TXT02 once GSUB/GPOS are in.      |
| TXT05 | `font-resolver`            | Abstract font refs ("sans-serif", CSS family lists) → FontHandle values.    |
| TXT06 | `line-breaker` (optional)  | UAX #14 line breaking, used by the default measurer's wrap path.            |

Two integration amendments are also required:

- **UI04 amendment** — remove the `text` field from paint-layer text
  description; require a `TextShaper` parameter in layout-to-paint.
- **P2D02–P2D05 amendments** — each paint backend specifies how it
  dispatches `glyph_run` based on the `font_ref` scheme (font-parser path
  via glyph-parser outlines, or native path via OS text API).

The first end-to-end target is **CommonMark markdown → Metal** via:

```
commonmark-parser → document-ast → layout (with TXT01 + TXT03a)
  → PaintScene → paint-vm-metal (CoreText rasterizer for coretext: font_refs)
```

The later target is **LaTeX → any backend** via:

```
latex-parser → layout (with TXT01 + TXT04 once HarfBuzz-like is ready)
  → PaintScene → any paint-vm (glyph-parser rasterizer for font-parser: font_refs)
```

Both targets use the same TXT00 interfaces; only the shaper
implementation differs.

---

## Non-goals

TXT00 explicitly does NOT cover:

**Bidi resolution.** UAX #9 is owned by the layout engine. The shaper
receives uniform-direction runs.

**Line breaking and justification.** UAX #14 line breaking lives in a
future optional spec (TXT06). The default measurer can call it but does
not define it.

**Font subsetting / embedding.** Producing a subset font for PDF
embedding is a codec concern, not a text-interface concern.

**Glyph rasterization.** The process of turning a glyph ID into pixels is
owned by the paint backend (via glyph-parser for font-parser bindings, or
via OS APIs for native bindings).

**Font loading I/O.** The act of reading font bytes from a file, network,
or system font registry is owned by FontResolver (TXT05). TXT00 assumes
handles are already resolved.

**Typesetting above the line level.** Paragraph layout, page breaking,
widow/orphan control, column balancing — these are higher-level
concerns that build on top of TXT00.

---

## Open questions

The following design decisions are deliberately left to the first
implementation PRs and will be finalized then:

- Whether `FontHandle` needs a type-level tag (e.g., Rust phantom types,
  TypeScript branded types) to make cross-binding misuse a compile error
  rather than a runtime error. This is more easily decided when writing
  the first Rust implementation.

- Whether `ShapedRun` should carry an explicit `FontMetrics` reference
  so that consumers can call `measure()` without separately passing
  metrics. The current design keeps them separate for orthogonality;
  ergonomics may argue for coupling.

- Whether `ShapeOptions.features` should be typed (enum of known OT tags)
  or string-typed (open-ended). The current design uses strings to allow
  unknown tags to pass through; a typed variant would catch typos.

These are not blockers for TXT01 or TXT02 implementations; they affect
only the Rust/TypeScript trait signatures, which can evolve before the
first shaper lands.
