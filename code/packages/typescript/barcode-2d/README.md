# @coding-adventures/barcode-2d

Shared 2D barcode abstraction layer for the coding-adventures monorepo.

Provides `ModuleGrid` (the universal intermediate representation for every 2D
barcode format) and `layout()` (the single function that converts abstract
module coordinates into pixel-level `PaintScene` instructions).

## What this is

Every 2D barcode encoder — QR Code, Data Matrix, Aztec Code, PDF417, MaxiCode —
produces the same kind of output: a 2D boolean grid where `true` = dark module
and `false` = light module. This package names that grid `ModuleGrid` and
provides a single `layout()` function to convert it into paint instructions.

## Pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid                   ← produced by the encoder
  → layout()                     ← this package
  → PaintScene                   ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout()` are in abstract module units. Only
`layout()` multiplies by `moduleSizePx` to produce pixel coordinates.

## Usage

```typescript
import { makeModuleGrid, setModule, layout } from "@coding-adventures/barcode-2d";

// Start with an all-light 21×21 grid (QR Code version 1 size).
let grid = makeModuleGrid(21, 21);

// An encoder would call setModule() many times to paint finder patterns,
// timing strips, data bits, and error correction bits.
grid = setModule(grid, 0, 0, true);
grid = setModule(grid, 0, 1, true);
// ... (actual encoding logic in qr-code package)

// Convert to a PaintScene at 10 px/module with a 4-module quiet zone.
const scene = layout(grid, { moduleSizePx: 10, quietZoneModules: 4 });

// scene is now ready for paint-vm to render to SVG, Canvas, Metal, etc.
```

## Supported module shapes

| Shape    | Format(s)                          | Instruction type |
|----------|------------------------------------|------------------|
| `square` | QR Code, Data Matrix, Aztec, PDF417 | `PaintRect`      |
| `hex`    | MaxiCode                           | `PaintPath`      |

## API

### `makeModuleGrid(rows, cols, moduleShape?): ModuleGrid`

Create an all-light (false) grid of the given dimensions. The `moduleShape`
defaults to `"square"`.

### `setModule(grid, row, col, dark): ModuleGrid`

Return a new grid (immutable) with one module changed. Throws `RangeError` if
the coordinates are out of bounds.

### `layout(grid, config?): PaintScene`

Convert a `ModuleGrid` to a `PaintScene`. The `config` parameter is a partial
`Barcode2DLayoutConfig`; unset fields use `DEFAULT_BARCODE_2D_LAYOUT_CONFIG`.

Throws `InvalidBarcode2DConfigError` if:
- `moduleSizePx <= 0`
- `quietZoneModules < 0`
- `config.moduleShape !== grid.moduleShape`

### `DEFAULT_BARCODE_2D_LAYOUT_CONFIG`

| Field            | Default     |
|------------------|-------------|
| moduleSizePx     | 10          |
| quietZoneModules | 4           |
| foreground       | `"#000000"` |
| background       | `"#ffffff"` |
| showAnnotations  | `false`     |
| moduleShape      | `"square"`  |

## Spec

See `code/specs/barcode-2d.md` for the full specification.
