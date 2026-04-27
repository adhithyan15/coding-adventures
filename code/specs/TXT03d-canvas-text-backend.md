# TXT03d — Canvas Text Backend (HTML Canvas 2D)

## Overview

TXT03d is the **browser-runtime** sibling to TXT03a/b/c. Where
CoreText, DirectWrite, and Pango are OS-level text stacks that
return addressable glyph IDs (`CGGlyph`, `u16` indices,
`PangoGlyph`), **HTML Canvas 2D does not expose glyph-level
primitives to JavaScript at all**. `ctx.measureText()` returns
widths and box metrics; `ctx.fillText()` accepts a string + font +
position and does shaping / fallback / rasterization internally.
There is no `ctx.drawGlyphs(glyph_ids[], positions[])`.

This forces a different tradeoff on the TXT00 pipeline:

- **FontMetrics** — implementable over `ctx.measureText()` and
  the computed CSS `font` shorthand. Returned values match the
  TXT00 contract, with caveats on `line_gap` / `x_height` /
  `cap_height` where the Canvas API is less detailed than
  CoreText / DirectWrite.
- **TextShaper** — **deliberately NOT implemented.** A TXT03d
  `shape()` could not produce a `ShapedRun` with real glyph IDs;
  there is no way to retrieve them from Canvas. Synthesizing
  fake glyph IDs would violate TXT00's font-binding invariant
  (opaque glyph tokens must be consumable by the matching paint
  backend, and no paint backend — not even paint-vm-canvas —
  can consume fake-ID glyphs through `fillText`).
- **PaintText emission** — TXT03d pairs with a new paint
  instruction, `PaintText` (see P2D00 amendment), which carries
  `{text, font_ref, font_size, x, y, fill}` directly. The browser
  does the final shaping + fallback + rasterization at paint
  time. This is exactly how `<canvas>`-based rendering libraries
  (Three.js, PixiJS Text, D3) have always worked; TXT03d just
  names it properly inside the TXT00 framework.

The font-binding invariant **still holds**, but on a coarser
grain: a `PaintText` with `font_ref: "canvas:..."` can only be
dispatched by a Canvas-capable paint backend. A Metal or Direct2D
backend must throw `UnsupportedFontBindingError` — the binding is
the runtime, not the glyph index table.

---

## When to use TXT03d

Use TXT03d when **all** of the following hold:

1. The target runtime is a web browser with Canvas 2D available.
2. Pixel-for-pixel reproducibility across browsers is not required.
3. The caller does not need per-glyph positioning (e.g., no
   hand-tuned kerning, no path-following text, no ligature
   inspection).

When any of these fail, prefer TXT01 + TXT02 (device-independent,
reproducible) or TXT01 + TXT04 (HarfBuzz, richer shaping) — both
run in the browser via wasm and produce real glyph IDs that a
Canvas backend can rasterize through a glyph-outline path if
needed.

TXT03d is the **fastest** path from a CommonMark AST to visible
pixels in a browser. It is the default for markdown-on-canvas and
for any app whose quality bar is "looks like native web text."

---

## The runtime handle

```
Canvas FontHandle  ≔  CssFontShorthand  (a string)
```

Canvas does not hand out a font object. Setting
`ctx.font = "16px Helvetica"` is idempotent and has no return
value; subsequent `measureText` / `fillText` calls use whatever
was last assigned. The TXT03d handle is therefore **just the CSS
font string** plus a canvas-context reference for measurement.

```typescript
interface CanvasFontHandle {
  css_font: string;       // e.g. "700 16px Helvetica, system-ui"
  px_size: number;        // the numeric size, parsed once for metric math
  measure_ctx: CanvasRenderingContext2D;  // a shared, invisible ctx for measurement
  ps_name_hint?: string;  // the primary family, used as the stable id key
}
```

The `measure_ctx` is a module-level `<canvas>` element whose
dimensions are 1x1; it is used only for `measureText()`. Sharing
one context across all measurements is safe because setting
`.font` is synchronous and the state does not persist between
calls.

## font_ref scheme

```
canvas:<id>

<id> format: "<primary_family>@<px_size>[:<weight>[:<style>]]"
Examples:
  "Helvetica@16"
  "Helvetica@16:700"
  "Helvetica@16:700:italic"
  "system-ui@14"

The paint backend (paint-vm-canvas) does not need a registry
lookup — it reconstructs the CSS font shorthand from <id> at
dispatch time and sets ctx.font before fillText. This makes
PaintText self-contained: a PaintText carries everything the
browser needs.
```

The canonical resolver produces the `<id>` deterministically so
that two equivalent font queries produce the same `font_ref` (and
therefore the same dispatch path).

---

## Metrics implementation

```
impl FontMetrics for CanvasMetrics {
    // units_per_em is a synthetic 1000 — Canvas has no design-unit
    // concept. All other metrics are returned in units of (design
    // units) = (pixel metric * 1000 / px_size), so the TXT00
    // contract ("return in design units") is honored.
    fn units_per_em(_) = 1000

    fn ascent(font) = {
        set ctx.font = font.css_font
        let m = ctx.measureText("M")
        return round(m.fontBoundingBoxAscent * 1000 / font.px_size)
    }

    fn descent(font) = {
        set ctx.font = font.css_font
        let m = ctx.measureText("M")
        return round(m.fontBoundingBoxDescent * 1000 / font.px_size)
    }

    fn line_gap(_) = 0
        // Canvas TextMetrics does not expose the OpenType `lineGap`
        // (the designer-recommended extra space between lines).
        // Returning 0 is the universally safe default; CSS-driven
        // line-height supplies the real gap at layout time.

    fn x_height(font) = {
        set ctx.font = font.css_font
        let m = ctx.measureText("x")
        return round(m.actualBoundingBoxAscent * 1000 / font.px_size)
    }

    fn cap_height(font) = {
        set ctx.font = font.css_font
        let m = ctx.measureText("H")
        return round(m.actualBoundingBoxAscent * 1000 / font.px_size)
    }

    fn family_name(font) = font.ps_name_hint OR first token of css_font
}
```

Two notes on accuracy:

- `fontBoundingBoxAscent/Descent` are **per-glyph** in the Canvas
  spec (for the text passed to `measureText`). Passing `"M"` and
  `"x"` is the standard trick to approximate cap-height and
  x-height; it matches what browsers do internally when
  rasterizing with `ctx.font` against system fonts.
- Older Safari releases (<14) do not populate
  `fontBoundingBoxAscent`. TXT03d falls back to
  `actualBoundingBoxAscent` on those engines and flags a
  degraded-mode warning in development builds.

---

## Measurement for layout (TextMeasurer / UI09)

The UI09 `TextMeasurer` trait — the one layout-block calls to
decide line wraps — is trivially implementable over
`ctx.measureText`:

```
impl TextMeasurer for CanvasTextMeasurer {
    fn measure_run(text, font, size) -> MeasuredRun {
        ctx.font = build_css_font(font, size)
        let m = ctx.measureText(text)
        return MeasuredRun {
            width: m.width,
            ascent: m.fontBoundingBoxAscent,
            descent: m.fontBoundingBoxDescent,
            font_ref: "canvas:<id>",
        }
    }
}
```

A layout pass driven by `CanvasTextMeasurer` produces the same
`PositionedNode` tree it produces under `CoreTextMeasurer` — only
the baked-in font_ref differs. The layout algorithm itself is
backend-agnostic. This is the load-bearing property that makes
TXT03d fit into the existing stack.

---

## Shaping (intentionally absent)

TXT03d does **not** implement `TextShaper`. Calling `shape()` on
a TXT03d-paired handle MUST return `ShapingError::NotApplicable`
with a message pointing the caller at the PaintText emission
path. This is a spec-level defense against accidental use — a
future developer who wires a glyph-atlas renderer against a
canvas-backed measurer gets a clear error instead of silent
garbage.

The convention is: **if your pipeline requires ShapedRun, use
TXT01+TXT02 (parser+naive shaper) or TXT01+TXT04 (parser+HarfBuzz)
in the browser via wasm. TXT03d is for the Canvas-native path
only.**

---

## PaintText emission (UI04 amendment)

Layout-to-paint (UI04), when configured with a Canvas text
pipeline, emits a `PaintText` instruction per run instead of
shaping + emitting `PaintGlyphRun`. The layout engine calls:

```
ctx.emit_text(PositionedNode { text, font, size, x, y_baseline, fill }):
    let font_ref = canvas_font_ref(font, size)
    append PaintText {
        x,
        y: y_baseline,
        text,
        font_ref,
        font_size: size,
        fill,
    }
```

Font fallback is handled by the browser at paint time — unlike
the CoreText path (TXT03a), the TXT03d emitter does not have to
segment runs per physical font. One `PaintText` per logical
layout run suffices; if the browser falls back (e.g., "Helvetica"
for Latin, Apple Color Emoji for 🎉), it happens inside
`fillText` and is invisible to the paint IR.

This is a genuine simplification — the architectural invariant
pushed down one layer: TXT03a needed to emit one PaintGlyphRun
per fallback segment to stay faithful to the glyph-ID binding;
TXT03d emits one PaintText for the whole run because no glyph IDs
exist to go wrong.

---

## Script / direction / features

- **Script**: Canvas auto-detects via the browser's text stack.
  No override is exposed through `measureText` / `fillText`. A
  `ShapeOptions.script` override at the measurer boundary is
  dropped with a one-line warning in development builds.
- **Direction**: `ctx.direction` accepts `"ltr"` / `"rtl"` /
  `"inherit"`. TXT03d maps `MeasureOptions.direction` to this;
  RTL-aware layout engines see a correctly-measured RTL width.
- **Features**: `ctx.fontVariantCaps`, `ctx.fontKerning`,
  `ctx.fontStretch`, `ctx.fontVariantSubst` exist in modern
  Chromium/WebKit. TXT03d v1 exposes only `fontKerning` (on/off,
  default on). Feature tags from `MeasureOptions.features` that
  do not map to a Canvas property are dropped silently.

---

## Resolver integration (TXT05 sibling)

TXT05's browser resolver (`font-resolver-canvas`) takes a
`FontQuery` and returns a `CanvasFontHandle`. The resolver is
trivial — almost all queries succeed immediately because
`ctx.font = "..."` never fails; the browser silently substitutes
a default. The resolver's job is:

1. Build the CSS font shorthand from query fields
   (`family`, `weight`, `style`, `px_size`).
2. Compute the canonical `<id>` string.
3. Verify the font renders (measure `"M"`; if
   `width === 0`, the family string was unparseable).

The resolver does **not** verify the font is actually available
on the user's machine; that's fundamentally not observable from
the browser (privacy protection). If the system substitutes a
default, that substitution is baked into the measurement and the
paint will use the same substitution — consistent by
construction.

---

## Non-goals

**Glyph-level rendering.** If you need to draw each glyph
individually (animation on individual letters, path-following
text, custom ligature override), use TXT01+TXT02/TXT04 and a
glyph-rasterization paint path (paint-vm-canvas's `image`
instruction with per-glyph bitmaps, or paint-vm-webgl with a
glyph atlas). TXT03d is explicitly not for this.

**SVG text.** SVG rendering is its own (different) system;
`<text>` elements have independent measurement semantics. A
sibling TXT03e for SVG text (via `SVGTextContentElement.getComputedTextLength`)
is a candidate for a future spec.

**WebGPU text.** A WebGPU-targeted text pipeline needs real glyph
IDs and a glyph atlas. TXT03d does not apply there — it is
specifically for the `<canvas>` 2D context.

**Hitting pixel-identical output in headless tests.** Browser
builds differ in font hinting, subpixel positioning, and fallback
rules. Test TXT03d with structural-similarity (SSIM) or
landmark-based asserts, not byte-equal PNG diffs.

**Variable-font axis control.** Canvas exposes `fontWeight` and
`fontStretch` but not arbitrary OpenType variation axes. TXT03d
v1 honors weight + stretch only.

---

## Testing strategy

1. **Metric sanity.** For a pinned browser (Chromium in
   Playwright, version-locked in CI), assert that `ascent`,
   `descent`, `x_height`, `cap_height` for common fonts
   (Helvetica, Arial, Georgia, Times, system-ui) are within
   documented ranges.

2. **Measurement consistency.** `measure_run("Hello", "Helvetica",
   16)` returns a width within 1px of the measurement taken by a
   standalone `measureText` call — asserts the TextMeasurer
   wrapper adds no arithmetic drift.

3. **PaintText round-trip.** Build a synthetic PositionedNode
   tree, run it through `layout_to_paint(measurer=Canvas)`,
   assert the output contains exactly one `PaintText` per text
   run, with correctly-propagated `font_ref`, `fill`, and
   position.

4. **font-binding-invariant violation.** Attempt to dispatch a
   `PaintText { font_ref: "canvas:..." }` on a non-canvas backend
   (e.g., paint-vm-metal). Assert `UnsupportedFontBindingError`.

5. **PaintText dispatch.** paint-vm-canvas consumes a PaintText,
   sets `ctx.font`, calls `ctx.fillText`. Screenshot comparison
   (SSIM ≥ 0.92) against a reference PNG rendered by a pinned
   Chromium.

6. **Shaping refusal.** Calling `TXT03d.shape(...)` returns
   `ShapingError::NotApplicable` — assert error variant, assert
   the message includes a pointer to TXT01+TXT02 / TXT01+TXT04.

7. **End-to-end Markdown on Canvas.** Render the same CommonMark
   test document (`test/fixtures/simple.md`) through both paths:
   - Path A: CommonMark → layout → CoreText → Metal (reference
     macOS bitmap)
   - Path B: CommonMark → layout → Canvas (headless Chromium
     bitmap)
   Assert: same node count in layout tree, same text content at
   each logical position, and overall SSIM ≥ 0.88. Pixel-identity
   is not expected.

CI runs the Canvas path on headless Chromium via Playwright.

---

## Package layout

```
text-native-canvas            (TypeScript; browser-only)
layout-text-measure-canvas    (TypeScript; browser-only, thin bridge)
font-resolver-canvas          (TypeScript; browser-only)
```

All three packages depend on:

- `text-interfaces-ts` (TXT00 ports)
- The browser runtime (no node.js; jsdom's Canvas shim is not
  a supported target)

`paint-vm-canvas` (already existing) is updated per the P2D00
amendment to handle the new `PaintText` instruction kind.

A Rust crate wrapping TXT03d does **not** exist — the runtime is
inherently browser-only. A wasm consumer that wants canvas text
must go through JS interop: call out to a TS-side TextMeasurer
and PaintText emitter via `wasm-bindgen`. A thin Rust-side
convenience crate (`text-native-canvas-wasm`) may expose the
bridge cleanly; that's deferred until there is a consumer
compiling Rust to wasm for this pipeline.

---

## Relationship to sibling specs

| Spec        | Relationship                                                                                         |
|-------------|------------------------------------------------------------------------------------------------------|
| TXT00       | Implements `FontMetrics`. Deliberately does not implement `TextShaper`.                              |
| TXT03a/b/c  | Sibling device-dependent backends for native OSes. TXT03d is the browser-runtime sibling.            |
| TXT01/02/04 | Orthogonal device-independent path; still usable in a browser via wasm when glyph IDs are required.  |
| TXT05       | Canvas resolver is the trivial case (no on-disk lookup needed).                                      |
| UI04        | Emits `PaintText` instead of `PaintGlyphRun` when driven by a Canvas text pipeline.                  |
| UI09        | `TextMeasurer` trait is implemented over `ctx.measureText`; the same layout algorithms apply.        |
| P2D00       | Adds `PaintText` instruction kind (P2D00 amendment).                                                 |
| P2D06       | `canvas:` scheme prefix routes `PaintText` to paint-vm-canvas.                                       |

### The Canvas-native pipeline

```
Markdown source              "## Hello → world"
    │
    ▼  commonmark parser (TypeScript or Rust/wasm)
MarkdownAST
    │
    ▼  UI06 (document-ast-to-layout)
LayoutNode tree
    │
    ▼  UI07 (layout-block)  + UI09 TextMeasurer backed by canvas measure_ctx
PositionedNode tree
    │
    ▼  UI04 (layout-to-paint)  — Canvas variant
PaintScene { PaintRect, PaintText, PaintText, ... }
    │
    ▼  paint-vm-canvas (P2D01 dispatch)
ctx.fillRect(...); ctx.font = "..."; ctx.fillText(...)
    │
    ▼  browser compositor
pixels in an on-screen <canvas>
```

The layout stages (UI06 → UI07 → UI04) are identical to the
CoreText path; only the measurer and the emitter plug-in differ.
This is the payoff of the TXT00 pluggable interfaces: one
layout engine, two runtimes.

---

## Open questions

- **Pooling the measure_ctx.** One shared measurement canvas per
  page is safe today; with Offscreen Canvas worker rendering, a
  per-worker measurer will be needed. Deferred until a
  worker-based consumer exists.

- **Font-loading API integration.** Modern browsers expose
  `document.fonts.load(css_font)` which returns a promise
  resolving when the font file is downloaded. TXT03d v1 assumes
  fonts are already loaded. A future revision should wait on
  `document.fonts.ready` before measuring; otherwise metrics
  reflect the fallback font and become wrong when the real font
  loads.

- **Emoji and color glyph rendering.** `ctx.fillText("🎉")` does
  the right thing on modern browsers (renders the color emoji
  from the system emoji font). No special handling is needed in
  TXT03d — it is deferred to the browser by design. A test case
  in the suite asserts that emoji round-trip without loss.

- **Sub-pixel positioning.** Canvas coordinates are floats; the
  browser may round them internally. Layout-block already rounds
  to integer pixel positions for stability. No action required
  in TXT03d.

- **Whether TXT03d should carve out its own file or live as
  TXT03d inside `TXT03-native-shapers.md`.** Kept as a separate
  file because the backend is not an OS, the package layout is
  TypeScript-only, and the shaping-opt-out is a material
  design divergence that benefits from its own narrative.
