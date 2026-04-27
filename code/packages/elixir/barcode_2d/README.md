# coding_adventures_barcode_2d

Elixir port of the shared 2D barcode abstraction layer.

## What it does

This package provides the two building blocks every 2D barcode format needs:

1. **`ModuleGrid`** — the universal intermediate representation produced by
   every 2D barcode encoder (QR Code, Data Matrix, Aztec Code, PDF417,
   MaxiCode). It is a 2D boolean grid: `true` = dark module, `false` = light
   module.

2. **`layout/2`** — the single function that converts abstract module
   coordinates into pixel-level `PaintScene` instructions ready for the PaintVM
   to render.

## Where it fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → layout/2            ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout/2` are measured in "module units" — abstract
grid steps. Only `layout/2` multiplies by `module_size_px` to produce real
pixel coordinates. Encoders never need to know about screen resolution or
output format.

## Usage

```elixir
alias CodingAdventures.Barcode2D
alias CodingAdventures.Barcode2D.Barcode2DLayoutConfig

# 1. Create an all-light grid (21×21 = QR Code version 1).
grid = Barcode2D.make_module_grid(21, 21)

# 2. Paint some dark modules (here: just the top-left corner for demo).
{:ok, grid} = Barcode2D.set_module(grid, 0, 0, true)
{:ok, grid} = Barcode2D.set_module(grid, 0, 1, true)

# 3. Convert to a PaintScene with default config.
{:ok, scene} = Barcode2D.layout(grid)
# scene.width  == 290.0  ((21 + 2*4) * 10)
# scene.height == 290.0
# scene.instructions contains 1 background rect + 2 dark rects

# Custom config:
config = %Barcode2DLayoutConfig{
  module_size_px: 5.0,
  quiet_zone_modules: 2,
  foreground: "#000000",
  background: "#ffffff"
}
{:ok, scene} = Barcode2D.layout(grid, config)

# MaxiCode (hex modules, 33×30):
hex_grid = Barcode2D.make_module_grid(33, 30, :hex)
{:ok, hex_grid} = Barcode2D.set_module(hex_grid, 0, 0, true)
hex_config = %Barcode2DLayoutConfig{module_shape: :hex}
{:ok, hex_scene} = Barcode2D.layout(hex_grid, hex_config)
```

## Module shapes

- **`:square`** (default) — used by QR Code, Data Matrix, Aztec Code, PDF417.
  Each dark module becomes a `paint_rect`.

- **`:hex`** — used by MaxiCode (ISO/IEC 16023). Uses flat-top hexagons in an
  offset-row grid. Each dark module becomes a `paint_path` tracing six
  vertices. Odd rows are offset right by `hex_width / 2`.

## Hex geometry

```
hex_width  = module_size_px
hex_height = module_size_px * (sqrt(3) / 2)   -- vertical row step
circum_r   = module_size_px / sqrt(3)          -- center-to-vertex distance

Vertices at angles 0°, 60°, 120°, 180°, 240°, 300° from center.
```

## Error handling

Both `set_module/4` and `layout/2` return `{:ok, result}` or `{:error, reason}`.

`layout/2` returns an error when:
- `module_size_px <= 0`
- `quiet_zone_modules < 0`
- `config.module_shape` does not match `grid.module_shape`

## Running tests

```bash
cd code/packages/elixir/barcode_2d
mix deps.get
mix test --cover
```

## Dependencies

- [`coding_adventures_paint_instructions`](../paint_instructions) — provides
  `paint_rect/6` and `paint_scene/5`.
