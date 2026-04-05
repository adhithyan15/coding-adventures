# Barcode 2D

## Overview

This spec defines the shared architecture for **two-dimensional barcode** work in
the coding-adventures monorepo.

1D barcodes are one-dimensional: a horizontal stream of alternating bars and
spaces, where information is encoded in the widths of those elements. A single
horizontal scan across the symbol is enough to decode it.

2D barcodes are fundamentally different. They encode information in a
two-dimensional grid of dark and light **modules** (square cells). Decoding
requires reading the entire grid, not just a single scan line. This lets 2D
barcodes store dramatically more information in a smaller physical space — a
QR code version 40 can hold over 7,000 digits; the most capable Code 128
barcode tops out at around 48 characters per inch.

The extra capacity comes at a cost in complexity:

- the symbol has a fixed, structured layout with dedicated regions for
  finder patterns, alignment patterns, error correction metadata, and data
- error correction is non-trivial (Reed-Solomon or BCH)
- encoding requires multiple passes: data encoding → error correction →
  module placement → masking

This spec defines the **shared data model and render pipeline** that all 2D
barcode implementations in this repo will use. Format-specific behavior lives
in separate spec files.

---

## The 2D Barcode Landscape

There are two structural families of 2D barcodes:

### Matrix codes

The data modules are arranged in a true two-dimensional grid. The entire area
of the symbol encodes information. A scanner reads the grid as a whole and
reconstructs the codewords from module positions.

| Format | Year | ECC | Typical use |
|--------|------|-----|-------------|
| QR Code | 1994 | Reed-Solomon GF(256) | Universal: URLs, payments, boarding passes, packaging |
| Data Matrix | 1989 | Reed-Solomon GF(256) | PCB markings, medicine, aerospace, US mail |
| Aztec Code | 1995 | Reed-Solomon GF(16/256) | Boarding passes (IATA), rail tickets (Eurostar, Amtrak) |
| MaxiCode | 1992 | Reed-Solomon GF(64) | UPS package routing |
| MicroQR | 2004 | Reed-Solomon GF(256) | Small footprint, single finder pattern |
| rMQR | 2020 | Reed-Solomon GF(256) | Rectangular micro QR for very narrow spaces |
| Han Xin | 2007 | Reed-Solomon GF(256) | Chinese standard GB/T 21049; native kanji/hanzi support |

### Stacked linear codes

A stacked linear code is essentially multiple 1D rows printed on top of each
other. Each row is a mini 1D barcode; the rows share a common start/stop
structure. Technically 2D (they need a 2D scan), but the encoding logic is
closer to a 1D format.

| Format | Year | ECC | Typical use |
|--------|------|-----|-------------|
| PDF417 | 1991 | Reed-Solomon GF(929) | Driver's licences (AAMVA), boarding passes (IATA), USPS |
| MicroPDF417 | 1994 | Reed-Solomon GF(929) | Compact variant of PDF417 |
| Codablock F | 1989 | Code 128 check digit | Industrial, healthcare |
| GS1 DataBar Expanded Stacked | 2006 | — | Produce, coupons |

### Implementation priority

The formats are not equally complex to implement. The difficulty scales with:

- how many structural regions the symbol has
- whether data and ECC are interleaved into blocks
- how mask scoring works
- how format/version metadata is embedded

Ranked easiest to hardest:

1. **Data Matrix ECC200** — simpler placement algorithm, Aztec-style finder
2. **QR Code** — moderate complexity; well-documented; universal use case
3. **Aztec Code** — compact and elegant, variable-size bullseye finder
4. **PDF417** — GF(929) not GF(256); row-based encoding; longest spec
5. **MaxiCode** — hexagonal dot grid; unusual symbol geometry

The first target is **QR Code** because it is the most recognizable format, has
the richest teaching material, and uses GF(256) Reed-Solomon which the repo
already provides through MA02.

---

## Why Reed-Solomon Is the Foundation

Every matrix barcode format uses some variant of Reed-Solomon error correction.
This is why MA00 (polynomial arithmetic), MA01 (GF(256)), and MA02 (Reed-Solomon
over GF(256)) were implemented before any 2D barcode spec.

The relationship between MA02 and the barcode formats:

| Format | Field | Generator roots | Notes |
|--------|-------|-----------------|-------|
| QR Code | GF(256), poly=0x11D | α^0 through α^{n-1} | b=0 convention |
| Data Matrix | GF(256), poly=0x12D | α^1 through α^{n} | b=1 — matches MA02 exactly |
| Aztec compact | GF(16), poly=0x13 | α^1 through α^{n} | Different field size |
| Aztec full | GF(256), poly=0x12D | α^1 through α^{n} | Matches MA02 exactly |
| MaxiCode | GF(64), poly=0x43 | α^1 through α^{n} | Different field size |
| PDF417 | GF(929) | α^3 through α^{n+2} | Completely different field |

The key insight: **QR Code uses a slightly different RS convention (b=0) from
MA02 (b=1).** In practice this means the generator polynomial is:

```
MA02: g(x) = (x + α¹)(x + α²)···(x + α^n)
QR:   g(x) = (x + α⁰)(x + α¹)···(x + α^{n-1})
```

For encoding (computing the remainder), this is trivially handled by
adjusting the generator polynomial. The qr-code package should implement
its own `rs_encode_qr(data, ec_codewords)` function using the QR generator
rather than calling MA02 directly.

Data Matrix and Aztec full mode use exactly the MA02 convention and can
call MA02 directly.

---

## The Shared Data Model

### ModuleGrid

All matrix 2D barcodes reduce to a two-dimensional grid of modules. A module
is either **dark** (1) or **light** (0). The grid is square for QR, Data
Matrix, MicroQR, and MaxiCode; it is rectangular for PDF417, rMQR, and some
Data Matrix variants.

```
ModuleGrid {
  cols: u32,             // width in modules
  rows: u32,             // height in modules (equal to cols for square formats)
  modules: [[bool]],     // modules[row][col]; true = dark module
}
```

Every 2D barcode implementation must produce a `ModuleGrid` as an
intermediate output. This grid is the boundary between the symbology-specific
encoding logic and the rendering layer.

### Module roles

For visualizers and debugging, it is useful to annotate each module with its
**role** — why that module is dark or light. The annotation is optional and
rendering does not depend on it; the bare grid is sufficient.

```
ModuleRole =
  | "finder"          -- part of a finder pattern
  | "separator"       -- quiet zone around finder pattern
  | "timing"          -- timing strip (alternating dark/light)
  | "alignment"       -- alignment pattern center or ring
  | "format"          -- format information bit
  | "version"         -- version information bit (QR v7+)
  | "dark-module"     -- always-dark reserved module (QR only)
  | "data"            -- actual codeword bit
  | "ecc"             -- error correction codeword bit
  | "remainder"       -- padding remainder bits (QR only)
  | "masked"          -- data/ecc module after masking
```

A full annotated grid looks like:

```
ModuleAnnotation {
  role: ModuleRole,
  dark: bool,
  codeword_index: u16?,   -- which codeword this bit belongs to (null for structural)
  bit_index: u8?,         -- which bit within the codeword (null for structural)
}
```

The annotated grid powers color-coded visualizations: finder patterns are
shown in blue, timing in grey, data in black/white, ECC in green, etc.

---

## The Render Pipeline

The full 2D barcode pipeline has five stages. The key architectural principle
is that the **layout stage** is the only stage that knows about pixels. Every
stage before it works in abstract module units. Every stage after it works in
concrete pixel coordinates.

```
input string / bytes
    │
    ▼
[ encode ]
    Encode the input according to the format's character sets and
    encoding modes (numeric, alphanumeric, byte, kanji, etc.).
    Apply error correction (Reed-Solomon or BCH).
    Place codewords and structural elements into the module grid.
    Apply masking and format/version information.
    Output: ModuleGrid — a 2D boolean grid in abstract module units.
    No pixels, no coordinates. Just a grid of dark and light modules.
    │
    ▼
[ layout ]
    Convert the abstract ModuleGrid into concrete pixel coordinates.
    This is the ONLY stage that knows about module_size_px, quiet zones,
    pixel density, and output dimensions.

    Input:  ModuleGrid + Barcode2DLayoutConfig
    Output: PaintScene (P2D00) — a list of PaintRect instructions with
            fully resolved pixel coordinates. Every dark module is a
            PaintRect at an exact (x, y, width, height) in pixels.

    The paint layer downstream is completely dumb. It receives a PaintScene
    and executes it without knowing anything about modules, barcodes, or
    error correction.
    │
    ▼
[ PaintVM.execute(scene, context) ]
    Routes each PaintInstruction in the PaintScene to its registered
    backend handler (P2D01). The PaintVM has no knowledge of barcodes.
    It only knows "here is a rect, paint it."
    │
    ▼
[ backend ]
    paint-vm-svg     → SVG markup string
    paint-vm-canvas  → HTML Canvas drawing calls
    paint-metal      → Metal GPU render → PixelContainer → PNG bytes
    paint-vm-terminal → box-drawing characters (future)
```

### The layout step in detail

The layout step is where all the geometry is resolved. It takes a ModuleGrid
(which knows nothing about pixels) and a config (which specifies the desired
output dimensions), and produces a `PaintScene` (P2D00) with every coordinate
fully resolved in pixels.

```
layout(grid: ModuleGrid, config: Barcode2DLayoutConfig) → PaintScene

-- Compute pixel dimensions
module_size_px = config.module_size_px
quiet_zone_px  = config.quiet_zone_modules * module_size_px
total_width    = (grid.cols + 2 * config.quiet_zone_modules) * module_size_px
total_height   = (grid.rows + 2 * config.quiet_zone_modules) * module_size_px

-- Build PaintScene
instructions = []

-- Background rect (covers entire symbol including quiet zone)
instructions.push(PaintRect(
  x=0, y=0, width=total_width, height=total_height,
  fill=config.background
))

-- One PaintRect per dark module
for each row in 0..grid.rows:
  for each col in 0..grid.cols:
    if grid.modules[row][col] == dark:
      instructions.push(PaintRect(
        x      = quiet_zone_px + col * module_size_px,
        y      = quiet_zone_px + row * module_size_px,
        width  = module_size_px,
        height = module_size_px,
        fill   = config.foreground,
      ))

return PaintScene(
  width=total_width, height=total_height,
  background=config.background,
  instructions=instructions
)
```

Optimization: adjacent dark modules in the same row can be merged into a
single wider rectangle. This reduces draw calls from O(dark_modules) to
O(dark_runs). For a QR code that is ~35% dark, version 10 has roughly
3,700 dark modules; horizontal run-length encoding reduces this to ~600
rectangles. This optimization is optional in v0.1.0.

### Layout configuration

```
Barcode2DLayoutConfig {
  module_size_px:      f64     -- size of one module in pixels (default: 10)
  quiet_zone_modules:  u32     -- quiet zone width in modules (format-specific minimum)
  foreground:          string  -- dark module color (default: "#000000")
  background:          string  -- light module color and background (default: "#ffffff")
  show_annotations:    bool    -- color-code module roles in PaintScene metadata (default: false)
}
```

The layout config is the seam between barcode logic and the paint layer. Nothing
after this point knows about modules, error correction, or barcode formats.

### Quiet zones

Every 2D barcode format requires a minimum quiet zone — a margin of light
modules around the symbol that scanners use to locate the symbol boundary.

| Format | Minimum quiet zone |
|--------|--------------------|
| QR Code | 4 modules on all sides |
| Data Matrix | 1 module on all sides (+ L-finder quiet zone) |
| Aztec | 1 module on all sides (bullseye is self-locating) |
| PDF417 | 2 modules left/right, 2 rows top/bottom |

The default `quiet_zone_modules` value should match the format minimum.

---

## Public API Shape

All 2D barcode packages in this repo should expose:

```
encode(input, options) → ModuleGrid
  -- Encode input into a ModuleGrid (abstract module units, no pixels).
  -- This is the pure symbology step.

layout(grid, config) → PaintScene
  -- Translate a ModuleGrid into a pixel-resolved PaintScene (P2D00).
  -- This is where all coordinate computation happens.
  -- The PaintScene can be passed directly to PaintVM.execute().

encode_and_layout(input, options, config) → PaintScene
  -- Convenience: encode + layout in one step.

render_svg(input, options?, config?) → string
  -- Convenience: encode + layout + paint-vm-svg in one step.
  -- Returns an SVG string. Useful for tests and server-side rendering.

explain(input, options) → AnnotatedModuleGrid
  -- encode with per-module role annotations (for visualizers).
  -- The annotations are reflected in PaintScene instruction metadata
  -- when layout() is called with show_annotations: true.
```

The `options` struct is format-specific. The `config` parameter is always
`Barcode2DLayoutConfig` and controls the pixel geometry.

---

## PaintInstructions Compatibility (P2D00)

The `barcode-2d` layout step produces a `PaintScene` (P2D00) using only
`PaintRect` instructions. No new instruction types are needed.

| Need | P2D00 primitive |
|------|----------------|
| Dark module | `PaintRect(x, y, w, h, fill="#000000")` |
| Light background | `PaintRect(0, 0, total_w, total_h, fill="#ffffff")` as first instruction |
| Quiet zone | Handled by offset calculation in the layout step |
| Color annotations | `PaintRect` with custom `fill` per role + `metadata.role` annotation |
| Label text | `PaintGlyphRun` below symbol (optional, v0.2.0) |

2D barcodes slot directly into the PaintVM (P2D01) dispatch pipeline. Every
backend that handles `PaintRect` — and all backends must — can render a 2D
barcode without any changes.

---

## Metal Rendering

A QR code is a grid of rectangles. The `paint-metal` backend (to be created
as the Rust Metal PaintVM backend) will handle `PaintRect` instructions via
GPU render passes. No barcode-specific GPU code is needed — the Metal backend
is completely barcode-agnostic.

The intended pipeline once `paint-metal` exists:

```
qr_code::encode(input)         → ModuleGrid
barcode_2d::layout(grid, cfg)  → PaintScene    (pixel-resolved PaintRect list)
paint_metal::create_vm()       → PaintVM<MetalContext>
vm.execute(scene, ctx)         → renders to offscreen Metal texture
vm.export(scene, opts)         → PixelContainer
paint_codec_png::encode(px)    → Vec<u8>       (PNG bytes)
```

For windowed display (blocking, until window is closed):
```
paint_metal::show_in_window(scene)
```

The paint layer is completely dumb — it receives a `PaintScene` full of
already-computed pixel coordinates and executes them on the GPU. It has no
knowledge of QR codes, module grids, or error correction.

### Swappable backends

This is the key property of the PaintVM architecture: **the barcode encoder
never changes when the backend changes.**

```
-- Today: SVG backend
let vm = paint_vm_svg::create_vm();
vm.execute(scene, &mut svg_context);    // → SVG string

-- Tomorrow: Metal backend (once paint-metal is built)
let vm = paint_metal::create_vm();
vm.execute(scene, &mut metal_ctx);      // → renders to Metal texture
vm.export(scene, opts)                  // → PixelContainer

-- PNG from Metal
let pixels = vm.export(scene, default_opts);
let png_bytes = paint_codec_png::encode(&pixels);  // → Vec<u8>

-- Native window from Metal
paint_metal::show_in_window(&scene);    // blocks until window is closed
```

The `scene` object is identical in all three cases — it comes from
`barcode_2d::layout(grid, config)` and is just a list of `PaintRect`
instructions with resolved pixel coordinates. The barcode packages have
no knowledge of which backend will execute the scene.

### paint-metal status

`paint-metal` does not yet exist. It is the next backend to be built after
the barcode-2d and qr-code packages. Its spec will follow as P2D02.

**Initial implementation uses Canvas (TypeScript) and SVG (all 9 languages).**
Once `paint-metal` is built as a Rust PaintVM backend, it slots in directly
with no changes to the barcode encoder. Passing a `paint-codec-png` encoder
to the painter produces PNG bytes; calling `show_in_window` produces a native
macOS window. Both paths use the same `PaintScene`.

---

## Spec Series Roadmap

| Spec | Package | Format | ECC field | Status |
|------|---------|--------|-----------|--------|
| barcode-2d.md | barcode-2d | Shared 2D abstraction | — | **this spec** |
| qr-code.md | qr-code | QR Code | GF(256) | next |
| data-matrix.md | data-matrix | Data Matrix ECC200 | GF(256) | planned |
| aztec-code.md | aztec-code | Aztec Code | GF(16/256) | planned |
| pdf417.md | pdf417 | PDF417 | GF(929) | planned |
| micro-qr.md | micro-qr | MicroQR | GF(256) | planned |

The `barcode-2d` package is a thin shared library — the ModuleGrid type,
the render config, and the `to_draw_scene` translation. Individual format
packages depend on it.

---

## Package Matrix

| Language | Package | Namespace |
|----------|---------|-----------|
| Rust | `code/packages/rust/barcode-2d/` | `barcode_2d` |
| TypeScript | `code/packages/typescript/barcode-2d/` | `@coding-adventures/barcode-2d` |
| Python | `code/packages/python/barcode-2d/` | `barcode_2d` |
| Go | `code/packages/go/barcode-2d/` | `barcode2d` |
| Ruby | `code/packages/ruby/barcode_2d/` | `CodingAdventures::Barcode2D` |
| Elixir | `code/packages/elixir/barcode_2d/` | `CodingAdventures.Barcode2D` |
| Lua | `code/packages/lua/barcode-2d/` | `coding_adventures.barcode_2d` |
| Perl | `code/packages/perl/barcode-2d/` | `CodingAdventures::Barcode2D` |
| Swift | `code/packages/swift/barcode-2d/` | `Barcode2D` |

---

## Dependency Stack

```
paint-metal (P2D02, Rust)   paint-vm-svg (TypeScript)   paint-vm-canvas (TypeScript)
      │                           │                             │
      └───────────────────────────┴─────────────────────────────┘
                                  │
                          paint-vm (P2D01)
                                  │
                       paint-instructions (P2D00)
                                  │
                          barcode-2d (this)          MA02 reed-solomon
                                  │                         │
                   ┌──────────────┴───────────┐    MA01 gf256
                   │              │            │            │
               qr-code    data-matrix   aztec-code   MA00 polynomial
                                  │
                               pdf417
```

The `barcode-2d` package sits at the boundary between barcode logic and the
paint stack:

- **Below the line**: barcode-specific code. Knows about modules, codewords,
  Reed-Solomon, finder patterns, masks.
- **Above the line**: completely barcode-agnostic. Knows only about painting
  rectangles at pixel coordinates.

The layout step in `barcode-2d` is the only code that crosses this boundary.
