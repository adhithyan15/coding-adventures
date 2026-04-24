# coding-adventures-barcode-2d (Lua)

Shared 2D barcode abstraction layer for Lua. Provides `ModuleGrid` (the
universal intermediate representation produced by every 2D barcode encoder) and
`layout()` (the single function that converts a grid into pixel-level
`PaintScene` instructions for the PaintVM).

## Where this fits

```
Input data
  -> format encoder (qr-code, data-matrix, aztec...)
  -> ModuleGrid          <- produced by the encoder
  -> layout()            <- THIS PACKAGE converts to pixels
  -> PaintScene          <- consumed by paint-vm (P2D01)
  -> backend (SVG, Metal, Canvas, terminal...)
```

All coordinates before `layout()` are measured in module units (abstract grid
steps). Only `layout()` multiplies by `module_size_px` to produce real pixel
coordinates. Encoders never need to know about screen resolution.

## Installation

```bash
luarocks make --local coding-adventures-barcode-2d-0.1.0-1.rockspec
```

## Usage

```lua
local b2d = require("coding_adventures.barcode_2d")

-- Create an all-light 21x21 grid (QR Code v1 size).
local grid = b2d.make_module_grid(21, 21)

-- Paint one dark module at row 1, col 1 (1-indexed).
grid = b2d.set_module(grid, 1, 1, true)

-- Convert to a PaintScene using default config (module_size=10, quiet_zone=4).
local scene = b2d.layout(grid)
-- scene.width  == 290  (21 + 2*4) * 10
-- scene.height == 290

-- Custom config:
local scene2 = b2d.layout(grid, {
    module_size_px     = 5,
    quiet_zone_modules = 2,
    foreground         = "#1a1a1a",
    background         = "#fafafa",
})
```

### Hex modules (MaxiCode)

```lua
-- MaxiCode uses 33x30 flat-top hexagonal grids.
local hex_grid = b2d.make_module_grid(33, 30, "hex")
hex_grid = b2d.set_module(hex_grid, 17, 15, true)

local scene = b2d.layout(hex_grid, { module_shape = "hex" })
-- scene.instructions[2].kind == "path"  (a 7-command hex polygon)
```

## API

### `b2d.make_module_grid(rows, cols [, module_shape])`

Returns a new `ModuleGrid` table with every module set to `false` (light).

- `rows` -- number of rows (height)
- `cols` -- number of columns (width)
- `module_shape` -- `"square"` (default) or `"hex"`

The returned table has fields: `rows`, `cols`, `modules` (2D table of booleans,
1-indexed), `module_shape`.

### `b2d.set_module(grid, row, col, dark)`

Returns a **new** `ModuleGrid` identical to `grid` except that module at
`(row, col)` is set to `dark`. Immutable -- the original grid is never modified.
Raises an error if `row` or `col` is out of bounds (1-indexed).

### `b2d.layout(grid [, config])`

Converts a `ModuleGrid` into a `PaintScene` table. Config fields (all optional):

| Field               | Default      | Description                           |
|---------------------|--------------|---------------------------------------|
| `module_size_px`    | `10`         | Pixels per module (must be > 0)       |
| `quiet_zone_modules`| `4`          | Quiet-zone border in module units     |
| `foreground`        | `"#000000"`  | Colour for dark modules               |
| `background`        | `"#ffffff"`  | Colour for light modules and border   |
| `show_annotations`  | `false`      | Reserved for future annotated grids   |
| `module_shape`      | `"square"`   | Must match `grid.module_shape`        |

Raises `"InvalidBarcode2DConfigError: ..."` if:
- `module_size_px <= 0`
- `quiet_zone_modules < 0`
- `config.module_shape` does not match `grid.module_shape`

## Testing

```bash
cd tests
busted . --verbose --pattern=test_
```

## Supported module shapes

- **Square** -- QR Code, Data Matrix, Aztec Code, PDF417. Each dark module
  becomes one `paint_rect` instruction.
- **Hex** (flat-top hexagons) -- MaxiCode (ISO/IEC 16023). Each dark module
  becomes one `paint_path` instruction with six vertices.
