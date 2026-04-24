# coding_adventures_barcode_2d

Shared 2D barcode abstraction layer. Converts an abstract `ModuleGrid` (boolean grid) into a `PaintScene` ready for the PaintVM.

## Where this fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid            ← produced by the encoder
  → Barcode2D.layout()    ← THIS PACKAGE converts to pixels
  → PaintScene            ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates *before* `layout()` are measured in abstract module units. Only `layout()` multiplies by `module_size_px` to produce real pixel coordinates — so encoders never need to know anything about screen resolution or output format.

## Installation

```ruby
gem "coding_adventures_barcode_2d"
```

## Usage

```ruby
require "coding_adventures_barcode_2d"

B2D = CodingAdventures::Barcode2D

# 1. Create an all-light 21×21 grid (QR Code version 1 dimensions)
grid = B2D.make_module_grid(21, 21)

# 2. Paint individual modules dark (immutable update — returns new grid)
grid = B2D.set_module(grid, 0, 0, true)
grid = B2D.set_module(grid, 0, 1, true)

# 3. Convert to a PaintScene
scene = B2D.layout(grid)
# scene.width  => 290  (21 + 2*4 quiet-zone modules) * 10px
# scene.height => 290
# scene.instructions => [background_rect, dark_rect_0_0, dark_rect_0_1]

# Custom config
scene = B2D.layout(grid,
  module_size_px: 4,
  quiet_zone_modules: 2,
  foreground: "#1a1a2e",
  background: "#eee",
)
```

### Hex modules (MaxiCode)

```ruby
grid = B2D.make_module_grid(33, 30, module_shape: "hex")
grid = B2D.set_module(grid, 16, 14, true)
scene = B2D.layout(grid, module_shape: "hex")
# Each dark module produces a PaintPath tracing a flat-top hexagon.
```

## API

### `Barcode2D.make_module_grid(rows, cols, module_shape: "square")`

Creates a new frozen `ModuleGrid` with all modules set to `false` (light).

### `Barcode2D.set_module(grid, row, col, dark)`

Returns a new frozen `ModuleGrid` with the module at `(row, col)` set to `dark`. The original grid is never modified. Raises `RangeError` for out-of-bounds coordinates.

### `Barcode2D.layout(grid, config = nil)`

Converts a `ModuleGrid` into a `PaintScene`. Accepts an optional config hash overriding any of the defaults:

| Key                  | Default     | Description                                  |
|----------------------|-------------|----------------------------------------------|
| `module_size_px`     | `10`        | Pixels per module                            |
| `quiet_zone_modules` | `4`         | Quiet-zone width in modules (on each side)   |
| `foreground`         | `"#000000"` | Dark module fill color                       |
| `background`         | `"#ffffff"` | Background fill color                        |
| `module_shape`       | `"square"`  | `"square"` or `"hex"` — must match the grid |

Raises `InvalidBarcode2DConfigError` for invalid config values or a shape mismatch.

## Module shapes

- **`"square"`** — QR Code, Data Matrix, Aztec Code, PDF417. Each module renders as a `PaintRect`.
- **`"hex"`** — MaxiCode (ISO/IEC 16023). Each module renders as a `PaintPath` tracing a flat-top hexagon with odd rows offset right by half a hex width.

## ModuleRole constants

`CodingAdventures::Barcode2D::ModuleRole` exposes symbolic role names for module annotations (used by visualizers, not by the renderer):

```
FINDER, SEPARATOR, TIMING, ALIGNMENT, FORMAT, DATA, ECC, PADDING
```

## License

MIT
