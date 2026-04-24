# Barcode2D (Swift)

Shared 2D barcode abstraction layer for the coding-adventures monorepo.

## What it does

This package provides the two building blocks every 2D barcode format needs:

1. **`ModuleGrid`** — the universal intermediate representation produced by every
   2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode). It is a 2D
   boolean grid: `true` = dark module, `false` = light module.

2. **`layout(grid:config:)`** — the single function that converts abstract module
   coordinates into pixel-level `PaintScene` instructions, ready for the
   PaintVM (P2D01) to render.

## Where it fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → layout()            ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout()` are measured in "module units" — abstract
grid steps. Only `layout()` multiplies by `moduleSizePx` to produce real
pixel coordinates. Encoders never need to know anything about screen
resolution or output format.

## Supported module shapes

| Shape | Used by | Rendering |
|-------|---------|-----------|
| `.square` (default) | QR Code, Data Matrix, Aztec, PDF417 | `PaintRect` |
| `.hex` | MaxiCode (ISO/IEC 16023) | `PaintRect` (hex tiling geometry) |

## Usage

```swift
import Barcode2D

// 1. Create a 21×21 QR Code v1 grid (all light)
var grid = makeModuleGrid(rows: 21, cols: 21)

// 2. Set dark modules (encoder's job)
grid = try setModule(grid: grid, row: 0, col: 0, dark: true)
grid = try setModule(grid: grid, row: 0, col: 1, dark: true)
// ... more setModule calls ...

// 3. Render to a PaintScene
let config = Barcode2DLayoutConfig(
    moduleSizePx: 10.0,
    quietZoneModules: 4,
    foreground: "#000000",
    background: "#ffffff"
)
let scene = try layout(grid: grid, config: config)
// scene is a PaintScene ready for any PaintVM backend
```

## API reference

### Types

| Type | Description |
|------|-------------|
| `ModuleGrid` | 2D boolean grid (rows × cols) with module shape |
| `ModuleShape` | `.square` or `.hex` |
| `ModuleRole` | Structural role of a module (finder, timing, data, ecc, …) |
| `ModuleAnnotation` | Per-module role metadata for visualizers |
| `AnnotatedModuleGrid` | `ModuleGrid` with a parallel annotation grid |
| `Barcode2DLayoutConfig` | Pixel-level rendering options |
| `Barcode2DError` | Error type (`.invalidConfig(String)`) |

### Functions

| Function | Description |
|----------|-------------|
| `makeModuleGrid(rows:cols:moduleShape:)` | Create an all-light grid |
| `setModule(grid:row:col:dark:)` | Immutable module update; returns new grid |
| `layout(grid:config:)` | Convert `ModuleGrid` → `PaintScene` |

### Barcode2DLayoutConfig defaults

| Field | Default | Notes |
|-------|---------|-------|
| `moduleSizePx` | `10.0` | QR at 210×210 px (v1, 4 quiet modules) |
| `quietZoneModules` | `4` | QR Code minimum per ISO/IEC 18004 |
| `foreground` | `"#000000"` | Black ink |
| `background` | `"#ffffff"` | White paper |
| `showAnnotations` | `false` | Opt-in for visualizers |
| `moduleShape` | `.square` | The common case |

## Dependencies

- `PaintInstructions` (local) — P2D00 rect instruction set

## Running tests

```sh
swift test
```

## Spec

See `code/specs/barcode-2d.md` for the full specification.
