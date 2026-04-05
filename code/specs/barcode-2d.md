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

The full 2D barcode encoding pipeline:

```
input string / bytes
    │
    ▼
[ data encoding ]
    Encode the input according to the format's character sets and
    encoding modes (numeric, alphanumeric, byte, kanji, etc.).
    Output: a sequence of codeword bytes.
    │
    ▼
[ error correction ]
    Apply Reed-Solomon (or BCH) to produce error correction codewords.
    Block interleaving where required by the format.
    Output: final message polynomial (data + ECC codewords).
    │
    ▼
[ module placement ]
    Place the codewords and structural elements (finder patterns,
    timing strips, format info, version info, alignment patterns)
    into a ModuleGrid. Follow format-specific placement rules exactly.
    Output: ModuleGrid with every module assigned.
    │
    ▼
[ masking ]
    For formats that support masking (QR, Data Matrix, Aztec):
    evaluate all candidate mask patterns, score each by penalty rules,
    and apply the lowest-scoring mask to data modules only.
    Output: final masked ModuleGrid.
    │
    ▼
[ format information ]
    Encode the mask pattern index and error correction level into
    dedicated format information bits. Place these bits into the
    reserved format modules.
    Output: final ModuleGrid, fully populated.
    │
    ▼
[ draw instructions ]
    Translate the ModuleGrid into backend-neutral draw instructions.
    Each dark module becomes a filled rectangle. The symbol as a whole
    lives inside a quiet zone margin.
    Output: DrawScene.
    │
    ▼
[ renderer ]
    SVG, PNG (via Metal + draw-instructions-png), or native window.
```

### From ModuleGrid to DrawScene

The translation is simpler than 1D barcodes. Every dark module at grid
position (col, row) becomes a filled square:

```
module_size_px = total_px / cols   -- e.g., 300px / 21 columns = ~14px/module
quiet_zone_px  = quiet_zone_modules * module_size_px

for each row in 0..rows:
  for each col in 0..cols:
    if modules[row][col] == dark:
      emit DrawRect(
        x      = quiet_zone_px + col * module_size_px,
        y      = quiet_zone_px + row * module_size_px,
        width  = module_size_px,
        height = module_size_px,
        fill   = foreground_color,
      )
```

Optimization: adjacent dark modules in the same row can be merged into a
single wider rectangle. This reduces draw calls from O(dark_modules) to
O(dark_runs). For a QR code that is ~35% dark, version 10 has roughly
3,700 dark modules; horizontal run-length encoding reduces this to ~600
rectangles. This optimization is optional in v0.1.0.

### Render configuration

```
Barcode2DRenderConfig {
  module_size_px:      f64     -- size of one module in pixels (default: 10)
  quiet_zone_modules:  u32     -- quiet zone width in modules (format-specific minimum)
  foreground:          string  -- dark module color (default: "#000000")
  background:          string  -- light module color and background (default: "#ffffff")
  show_annotations:    bool    -- color-code module roles (default: false)
}
```

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
  -- encode input into a ModuleGrid

render(input, options) → DrawScene
  -- encode + translate to draw instructions in one step

render_svg(input, options) → string
  -- encode + render + SVG string in one step

explain(input, options) → AnnotatedModuleGrid
  -- encode with per-module role annotations (for visualizers)
```

The `options` struct is format-specific but always includes at minimum the
`Barcode2DRenderConfig` fields above.

---

## Draw Instructions Compatibility

The existing draw-instructions IR already supports all the operations needed
to render 2D barcodes:

| Need | IR primitive |
|------|-------------|
| Dark module | `DrawRect(x, y, w, h, fill="#000000")` |
| Light background | `DrawRect(0, 0, total_w, total_h, fill="#ffffff")` covering full symbol |
| Quiet zone | Handled by offset calculation in module placement |
| Color annotations | `DrawRect` with custom fill per role |
| Label text | `DrawText` below symbol |

No IR changes are needed. 2D barcodes slot directly into the existing
draw-instructions → SVG/PNG/Metal pipeline.

---

## Metal Rendering

Because a 2D barcode is a grid of rectangles and the draw-instructions-metal
backend already renders `DrawRect` primitives via GPU, a QR code can be
rendered to a native macOS window or PNG using Metal with zero additional GPU
code.

The existing pipeline:

```
qr_code::encode(input) → ModuleGrid
  → barcode_2d::to_draw_scene(grid, config) → DrawScene
  → draw_instructions_metal::render(scene) → PixelBuffer
  → draw_instructions_png::encode(buffer) → Vec<u8>
```

or for windowed display:

```
draw_instructions_metal_window::show_in_window(scene)
```

This is the direct payoff of the barcode → draw-instructions → Metal
architecture established with the 1D barcode pipeline.

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
draw-instructions-metal   draw-instructions-png
         │                        │
         └──────────┬─────────────┘
                    │
          draw-instructions        MA02 reed-solomon
                    │                       │
          barcode-2d (this)        MA01 gf256
                    │                       │
           ┌────────┴────────┐     MA00 polynomial
           │                 │
       qr-code          data-matrix   aztec-code   pdf417
```

Every format package depends on `barcode-2d` for the shared ModuleGrid
type and render translation, and on `MA02` (or a format-specific RS variant)
for error correction.
