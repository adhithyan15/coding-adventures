# UI09 — layout-text-measure: TextMeasurer Implementations

## Overview

`TextMeasurer` is the interface defined in `layout-ir` (UI02) that layout
algorithms call to measure text. This spec documents the standard set of
`TextMeasurer` implementations shipped in the coding-adventures stack and
their trade-offs.

Each implementation is a **separate package**. Layout algorithms never import
a measurer — they accept one as a parameter. The caller wires up the right
measurer for the context.

---

## The interface (defined in `layout-ir`)

```
TextMeasurer {
  measure(
    text:     string,
    font:     FontSpec,
    maxWidth: float | null
  ) → MeasureResult
}

MeasureResult {
  width:     float    // measured width in logical units
  height:    float    // measured height in logical units
  lineCount: int      // number of lines if wrapped
}
```

---

## Package 1: `layout-text-measure-estimated`

**Available in:** all 9 languages

**Depends on:** `layout-ir`

A fast, zero-dependency, font-independent text measurer that uses a fixed
character-width model. Suitable for:

- Headless environments with no font access
- Server-side layout (when pixel-perfect accuracy is not required)
- Testing and CI (deterministic, no platform fonts needed)
- Early-stage layout passes where approximate sizes are sufficient

### Model

```
estimated_width(text, font) =
    len(text) × font.size × avgCharWidthMultiplier

estimated_height(text, font, maxWidth) =
    line_count(text, font, maxWidth) × font.size × font.lineHeight
```

Default `avgCharWidthMultiplier = 0.6` (a reasonable approximation for
proportional Latin fonts at typical weights). The multiplier is configurable
at construction time.

For multi-line measurement (when `maxWidth` is not null):

```
chars_per_line = floor(maxWidth / (font.size × avgCharWidthMultiplier))
line_count     = ceil(len(text) / chars_per_line)
```

### Constructor

```
new_estimated_measurer(opts?) → TextMeasurer

opts {
  avgCharWidthMultiplier: float    // default 0.6
}
```

### Trade-offs

| Property | Value |
|---|---|
| Accuracy | Low — approximation only |
| Speed | Very fast — O(n) string length |
| Dependencies | None |
| Platform | All (pure math) |
| Determinism | Yes — same result everywhere |

---

## Package 2: `layout-text-measure-canvas`

**Available in:** TypeScript only

**Depends on:** `layout-ir`

Uses `CanvasRenderingContext2D.measureText()` for accurate browser font
measurements. Suitable for:

- Browser-side layout before Canvas rendering
- Any TypeScript context where a `CanvasRenderingContext2D` is available

### Model

The measurer creates an offscreen canvas and holds a reference to its 2D
context. For each `measure()` call:

1. Set `ctx.font` from the `FontSpec`:
   ```
   ctx.font = `${italic ? "italic " : ""}${weight} ${size}px "${family}"`
   ```
2. Call `ctx.measureText(text)` → `TextMetrics`
3. Width = `metrics.width` (in CSS pixels = logical units at `devicePixelRatio = 1`)
4. Height = `metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent`
   (full bounding box height for the measured string)
5. For multi-line (`maxWidth` not null): binary-search word-wrap boundaries
   to find line count and total height

### Font loading

`measureText` returns inaccurate results if fonts are not loaded. The Canvas
text measurer should be created after `await document.fonts.ready`. This is
the caller's responsibility, not the measurer's.

### Constructor

```
new_canvas_measurer(ctx: CanvasRenderingContext2D) → TextMeasurer
```

The context is used only for measurement — it is never drawn to.

### Trade-offs

| Property | Value |
|---|---|
| Accuracy | High — uses actual browser font metrics |
| Speed | Moderate — `measureText` is fast but not free |
| Dependencies | Browser Canvas API |
| Platform | Browser / Node.js with canvas package |
| Determinism | No — depends on installed fonts |

---

## Package 3: `layout-text-measure-rs`

**Available in:** Rust; FFI surface callable from all other languages

**Depends on:** `layout-ir` (Rust port)

A font-metric-based text measurer using `fontdue` (pure Rust font rasterizer
and layout engine). Suitable for:

- Server-side layout with font accuracy (PDF generation, native apps)
- Rust-based Canvas or Metal rendering pipelines
- All other languages via the FFI surface

### Model

1. Load font data (TTF/OTF bytes) at construction time
2. For each `measure()` call:
   - Lay out glyphs using `fontdue::layout::Layout`
   - Return bounding box of laid-out glyphs in logical units

### Font loading

Fonts must be loaded from bytes. The constructor accepts raw font bytes:

```rust
pub fn new(font_bytes: &[u8]) -> Result<RustTextMeasurer, MeasureError>
```

A `FontCache` struct is provided for loading multiple font families by name.

### FFI surface (C ABI)

The Rust crate exposes a C-compatible API for other languages to call:

```c
// Create a measurer from font bytes
TMeasurer* tm_new(const uint8_t* font_data, size_t font_len);

// Measure text — returns result by writing to out_*
void tm_measure(
    const TMeasurer* m,
    const char* text,
    float size,
    int   weight,
    bool  italic,
    float line_height,
    float max_width,     // -1.0 = unconstrained
    float* out_width,
    float* out_height,
    int*   out_line_count
);

// Free the measurer
void tm_free(TMeasurer* m);
```

Each language's `layout-text-measure-rs` package is a thin wrapper around
this C ABI:

- **Lua** — `ffi.load` + cdecl bindings
- **Perl** — `FFI::Platypus` bindings
- **Python** — `ctypes` or `cffi` bindings
- **Ruby** — `Fiddle` or `ffi` gem bindings
- **Go** — `cgo` bindings
- **Elixir** — NIF or `Zigler` bindings

### Trade-offs

| Property | Value |
|---|---|
| Accuracy | High — real glyph metrics |
| Speed | Moderate — font layout is non-trivial |
| Dependencies | Rust + fontdue crate; font bytes at runtime |
| Platform | All (Rust compiles everywhere) |
| Determinism | Yes — same font bytes → same metrics |

---

## Choosing a measurer

| Context | Recommended measurer |
|---|---|
| Browser Canvas rendering | `layout-text-measure-canvas` |
| CI / unit tests | `layout-text-measure-estimated` |
| Server-side PDF generation | `layout-text-measure-rs` |
| Native Metal / Direct2D app (Rust) | `layout-text-measure-rs` |
| Native Metal / Direct2D app (Swift) | `layout-text-measure-canvas` using `CoreText` (future) |
| Approximate layout, any language | `layout-text-measure-estimated` |

---

## Measurer composition

Two measurers can be composed for a "fast-first, accurate-second" approach:

```
layout_flexbox(tree, constraints, estimated_measurer)   ← first pass: fast layout
    ↓  produces approximate PositionedNode tree
    ↓  convert to initial render
layout_flexbox(tree, constraints, canvas_measurer)      ← second pass: accurate layout
    ↓  produces refined PositionedNode tree
    ↓  re-render with accurate positions
```

This pattern is useful for streaming / progressive rendering where an initial
paint appears quickly before fonts are fully measured. The compositor
implements this pattern; it is not part of any measurer package.
