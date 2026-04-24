# barcode-2d (Rust)

Shared 2D barcode abstraction layer for the coding-adventures monorepo.

Provides `ModuleGrid` (the universal intermediate representation for every 2D
barcode format) and `layout()` (the single function that converts abstract
module coordinates into pixel-level `PaintScene` instructions).

## What this is

Every 2D barcode encoder — QR Code, Data Matrix, Aztec Code, PDF417, MaxiCode —
produces the same kind of output: a 2D boolean grid where `true` = dark module
and `false` = light module. This crate names that grid `ModuleGrid` and provides
a single `layout()` function to convert it into paint instructions.

## Pipeline

```text
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid                   ← produced by the encoder
  → layout()                     ← this crate
  → PaintScene                   ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout()` are in abstract module units. Only
`layout()` multiplies by `module_size_px` to produce pixel coordinates.

## Usage

```rust
use barcode_2d::{make_module_grid, set_module, layout, ModuleShape, Barcode2DLayoutConfig};

// Start with an all-light 21×21 grid (QR Code version 1 size).
let grid = make_module_grid(21, 21, ModuleShape::Square);

// An encoder would call set_module() many times to paint finder patterns,
// timing strips, data bits, and error correction bits.
let grid = set_module(&grid, 0, 0, true);
// ... (actual encoding logic in qr-code crate)

// Convert to a PaintScene at 10 px/module with a 4-module quiet zone.
let config = Barcode2DLayoutConfig::default(); // 10 px, 4 quiet modules
let scene = layout(&grid, &config).unwrap();

// scene is now ready for paint-vm to render to SVG, Metal, etc.
```

## Supported module shapes

| Shape              | Format(s)                          | Instruction type |
|--------------------|------------------------------------|------------------|
| `ModuleShape::Square` | QR Code, Data Matrix, Aztec, PDF417 | `PaintRect`  |
| `ModuleShape::Hex`    | MaxiCode                           | `PaintPath`  |

## API

### `make_module_grid(rows, cols, module_shape) -> ModuleGrid`

Create an all-light (false) grid of the given dimensions.

### `set_module(grid, row, col, dark) -> ModuleGrid`

Return a new grid with one module changed. Panics if coordinates are out of
bounds (programming error in the encoder).

### `layout(grid, config) -> Result<PaintScene, Barcode2DError>`

Convert a `ModuleGrid` to a `PaintScene`. Returns `Err(InvalidConfig(…))` if:
- `module_size_px <= 0.0`
- `config.module_shape != grid.module_shape`

### `Barcode2DLayoutConfig::default()`

| Field              | Default     |
|--------------------|-------------|
| module_size_px     | 10.0        |
| quiet_zone_modules | 4           |
| foreground         | `"#000000"` |
| background         | `"#ffffff"` |
| show_annotations   | `false`     |
| module_shape       | `Square`    |

## Spec

See `code/specs/barcode-2d.md` for the full specification.
