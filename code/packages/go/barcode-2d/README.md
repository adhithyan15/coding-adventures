# barcode-2d (Go)

Shared 2D barcode abstraction layer for Go.

This package provides the two building blocks that every 2D barcode format
encoder needs before it can produce renderable output:

1. **`ModuleGrid`** — the universal intermediate representation. A 2D boolean
   grid where `true` = dark module and `false` = light module. Every format
   (QR Code, Data Matrix, Aztec Code, PDF417, MaxiCode) produces one of these.

2. **`Layout()`** — the single function that converts abstract module coordinates
   into pixel-level `PaintScene` instructions, ready for the paint VM.

## Where this fits in the pipeline

```
Input data
  → format encoder  (qr-code, data-matrix, aztec, pdf417, maxicode)
  → ModuleGrid      ← produced by the encoder
  → Layout()        ← THIS PACKAGE converts to pixels
  → PaintScene      ← consumed by paint-vm
  → backend         (SVG, terminal, Metal, Direct2D, Canvas…)
```

All coordinates before `Layout()` are in *module units* — abstract grid steps
with no physical size. Only `Layout()` multiplies by `ModuleSizePx` to produce
real pixel coordinates, so encoders never need to know anything about screen
resolution or output format.

## Supported module shapes

| Shape  | Used by                          | Rendering      |
|--------|----------------------------------|----------------|
| Square | QR Code, Data Matrix, Aztec, PDF417 | `PaintRect` per dark module |
| Hex    | MaxiCode (ISO/IEC 16023)         | `PaintPath` hexagon per dark module |

## Installation

```go
import barcode2d "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
```

## Usage

### Creating a grid

```go
// Start with a 21×21 all-light grid (typical QR Code v1 size).
grid := barcode2d.MakeModuleGrid(21, 21, barcode2d.ModuleShapeSquare)
```

### Setting dark modules

`SetModule` is pure and immutable — it never modifies the input grid. Only
the changed row is re-allocated; all other rows are shared between old and new
grid.

```go
// Paint a dark module at row=2, col=3.
grid, err := barcode2d.SetModule(grid, 2, 3, true)
if err != nil {
    // err is non-nil only when row or col is out of bounds
    log.Fatal(err)
}
```

### Rendering to a PaintScene

```go
// Use default config (10 px modules, 4-module quiet zone, black on white).
scene, err := barcode2d.Layout(grid, nil)
if err != nil {
    log.Fatal(err)
}
// scene is a PaintScene ready for the paint VM.
```

Custom config:

```go
cfg := barcode2d.Barcode2DLayoutConfig{
    ModuleSizePx:     4,
    QuietZoneModules: 4,
    Foreground:       "#000000",
    Background:       "#ffffff",
    ModuleShape:      barcode2d.ModuleShapeSquare,
}
scene, err := barcode2d.Layout(grid, &cfg)
```

### MaxiCode (hex modules)

```go
// MaxiCode is always 33 rows × 30 cols with hex modules.
hexGrid := barcode2d.MakeModuleGrid(33, 30, barcode2d.ModuleShapeHex)
// ... encoder fills in dark modules via SetModule ...

cfg := barcode2d.Barcode2DLayoutConfig{
    ModuleSizePx:     10,
    QuietZoneModules: 1,
    Foreground:       "#000000",
    Background:       "#ffffff",
    ModuleShape:      barcode2d.ModuleShapeHex,
}
scene, err := barcode2d.Layout(hexGrid, &cfg)
```

## API reference

| Symbol | Description |
|--------|-------------|
| `ModuleGrid` | 2D boolean grid (rows × cols). The universal encoder output. |
| `ModuleShape` | `ModuleShapeSquare` or `ModuleShapeHex`. |
| `ModuleRole` | One of eight structural roles (Finder, Timing, Data, Ecc, …). |
| `ModuleAnnotation` | Per-module metadata for visualizers (role, codeword/bit index). |
| `AnnotatedModuleGrid` | `ModuleGrid` + parallel annotation slice. |
| `Barcode2DLayoutConfig` | Pixel-level rendering options. |
| `DefaultBarcode2DLayoutConfig` | 10 px modules, 4-module quiet zone, black on white. |
| `MakeModuleGrid(rows, cols, shape)` | Create an all-light grid. |
| `SetModule(grid, row, col, dark)` | Return a new grid with one module changed (immutable). |
| `Layout(grid, config)` | Convert a `ModuleGrid` to a `PaintScene`. |
| `Barcode2DError` | Base error type. |
| `InvalidBarcode2DConfigError` | Returned by `Layout()` for bad config. |
| `IsInvalidBarcode2DConfigError(err)` | Convenience type-check helper. |
| `Version` | Semantic version string `"0.1.0"`. |

## Dependencies

- [`paint-instructions`](../paint-instructions) — `PaintScene`, `PaintRect`, `PaintPath`, `PathCommand`.

## Testing

```bash
go test ./... -v -cover
go vet ./...
```

Coverage: 100% of statements across 42 tests.
