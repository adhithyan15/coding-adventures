# CodingAdventures.Barcode2D (F#)

Universal 2D barcode module-grid abstraction and pixel-layout engine.

## What this package does

This package provides the two building blocks that every 2D barcode format
needs:

1. **`ModuleGrid`** — the universal intermediate representation produced by
   every 2D barcode encoder (QR Code, Data Matrix, Aztec, PDF417, MaxiCode).
   It is a 2-D boolean grid: `true` = dark module, `false` = light module.

2. **`Barcode2D.layout`** — the single function that converts abstract module
   coordinates into pixel-level `PaintScene` instructions ready for the
   PaintVM (P2D01) to render.

## Where this fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → Barcode2D.layout    ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout` are measured in "module units" — abstract grid
steps. Only `layout` multiplies by `ModuleSizePx` to produce real pixel
coordinates. Encoders never need to know anything about screen resolution or
output format.

## Supported module shapes

| Shape    | Used by                              | Rendering        |
|----------|--------------------------------------|------------------|
| `Square` | QR Code, Data Matrix, Aztec, PDF417  | `PaintRect`      |
| `Hex`    | MaxiCode (ISO/IEC 16023)             | `PaintPath` (flat-top hexagon) |

## Usage

```fsharp
open CodingAdventures.Barcode2D

// 1. Create an all-light 21×21 QR Code v1 grid.
let grid = Barcode2D.makeModuleGrid 21 21 Square

// 2. Paint dark modules (shown here manually; real encoders do this
//    algorithmically for finder patterns, timing strips, data bits, etc.).
let grid2 = Barcode2D.setModule grid 0 0 true
let grid3 = Barcode2D.setModule grid2 0 20 true

// 3. Convert to a PaintScene with default settings (10 px/module, 4-module
//    quiet zone, black on white).
let scene = Barcode2D.layout grid3 Barcode2D.defaultConfig

// 4. Or override specific fields.
let bigScene =
    Barcode2D.layout grid3 { Barcode2D.defaultConfig with ModuleSizePx = 20.0 }
```

## API reference

### Types

| Type | Description |
|------|-------------|
| `ModuleShape` | `Square` or `Hex` — controls rendering path in `layout` |
| `ModuleGrid` | `{ Rows; Cols; Modules: bool[][] array; ModuleShape }` |
| `AnnotatedModuleGrid` | `{ Grid: ModuleGrid; Roles: string option[][] }` — for visualizers |
| `Barcode2DLayoutConfig` | `{ ModuleSizePx; QuietZoneModules; DarkColor; LightColor }` |

### Functions (all in `[<RequireQualifiedAccess>]` module `Barcode2D`)

| Function | Signature | Description |
|----------|-----------|-------------|
| `makeModuleGrid` | `int → int → ModuleShape → ModuleGrid` | Create an all-light grid |
| `setModule` | `ModuleGrid → int → int → bool → ModuleGrid` | Immutable module update |
| `layout` | `ModuleGrid → Barcode2DLayoutConfig → PaintScene` | Render to pixel scene |
| `defaultConfig` | `Barcode2DLayoutConfig` | Default layout settings |
| `VERSION` | `string` | Package version |

### Default layout config

| Field              | Default   | Note                              |
|--------------------|-----------|-----------------------------------|
| `ModuleSizePx`     | `10.0`    | 10 px/module → 210×210 for QR v1  |
| `QuietZoneModules` | `4`       | QR Code minimum (ISO/IEC 18004)   |
| `DarkColor`        | `#000000` | Black ink                         |
| `LightColor`       | `#ffffff` | White paper                       |

## Immutability

`ModuleGrid` is intentionally immutable. `setModule` returns a new grid with
only the affected row re-allocated (structural sharing). This makes it trivial
to backtrack during QR mask evaluation:

```fsharp
let baseGrid = buildDataModules input
let best =
    [0..7]
    |> List.map (fun mask -> applyMask baseGrid mask)
    |> List.minBy evaluateMaskPenalty
```

## Dependencies

- `CodingAdventures.PaintInstructions` (F#) — provides `PaintScene`,
  `PaintInstruction`, `PathCommand`, `PaintRect`, `PaintPath`, and helper
  functions.

## Version

`0.1.0`
