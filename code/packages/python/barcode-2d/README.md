# coding-adventures-barcode-2d

Shared 2D barcode abstraction layer for the coding-adventures monorepo.

This package provides two building blocks used by every 2D barcode format:

1. **`ModuleGrid`** — the universal intermediate representation produced by
   every 2D barcode encoder (QR Code, Data Matrix, Aztec Code, PDF417,
   MaxiCode).  It is a 2D boolean grid: `True` = dark module, `False` = light.

2. **`layout()`** — the single function that converts abstract module
   coordinates into pixel-level `PaintScene` instructions ready for the
   PaintVM (P2D01) to render.

## Pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → layout()            ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout()` are measured in "module units" — abstract
grid steps.  Only `layout()` multiplies by `module_size_px` to produce real
pixel coordinates.

## Supported module shapes

| Shape  | Formats                           | Instruction type     |
|--------|-----------------------------------|----------------------|
| square | QR Code, Data Matrix, Aztec, PDF417 | `PaintRectInstruction` |
| hex    | MaxiCode (ISO/IEC 16023)          | `PaintPathInstruction` |

## Installation

```bash
pip install coding-adventures-barcode-2d
```

## Usage

```python
from barcode_2d import (
    make_module_grid,
    set_module,
    layout,
    Barcode2DLayoutConfig,
)

# 1. Create a 21×21 grid (e.g. for QR Code v1)
grid = make_module_grid(21, 21)

# 2. Set dark modules (encoder would do this)
grid = set_module(grid, 0, 0, True)
grid = set_module(grid, 0, 1, True)

# 3. Convert to PaintScene
cfg = Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=4)
scene = layout(grid, cfg)
# scene.width == 290, scene.height == 290

# Pass scene to any PaintVM backend
# paint_vm_svg.execute(scene, context) → SVG string
```

### MaxiCode (hex grid)

```python
from barcode_2d import make_module_grid, set_module, layout, Barcode2DLayoutConfig

# MaxiCode is always 33 rows × 30 cols
grid = make_module_grid(33, 30, module_shape="hex")
grid = set_module(grid, 0, 0, True)

cfg = Barcode2DLayoutConfig(
    module_size_px=10,
    quiet_zone_modules=1,
    module_shape="hex",
)
scene = layout(grid, cfg)
# Dark modules become PaintPathInstruction (flat-top hexagons)
```

## API

### `make_module_grid(rows, cols, module_shape="square") → ModuleGrid`

Creates a grid with all modules set to `False` (light).

### `set_module(grid, row, col, dark) → ModuleGrid`

Returns a new `ModuleGrid` with one module changed.  The original is
never mutated.  Raises `IndexError` for out-of-bounds coordinates.

### `layout(grid, config=None) → PaintScene`

Converts a `ModuleGrid` to a `PaintScene`.

Raises `InvalidBarcode2DConfigError` if:
- `module_size_px <= 0`
- `quiet_zone_modules < 0`
- `config.module_shape` does not match `grid.module_shape`

### `Barcode2DLayoutConfig`

Frozen dataclass with rendering options:

| Field              | Default    | Description                        |
|--------------------|------------|------------------------------------|
| `module_size_px`   | `10`       | Pixels per module                  |
| `quiet_zone_modules` | `4`      | Quiet zone width in modules        |
| `foreground`       | `"#000000"` | Dark module colour                |
| `background`       | `"#ffffff"` | Background colour                 |
| `show_annotations` | `False`    | Opt-in annotation rendering        |
| `module_shape`     | `"square"` | Must match `grid.module_shape`     |

## Dependencies

- `coding-adventures-paint-instructions` — `PaintScene`, `PaintRectInstruction`,
  `PaintPathInstruction`, `PathCommand`

## Development

```bash
uv venv
uv pip install -e ../paint-instructions
uv pip install -e ".[dev]"
uv run pytest tests/ -v
uv run ruff check src/
uv run ruff format --check src/
```
