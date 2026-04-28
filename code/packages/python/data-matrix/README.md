# coding-adventures-data-matrix

Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.

## What is Data Matrix?

Data Matrix is a two-dimensional matrix barcode invented in 1989 and
standardised as ISO/IEC 16022:2006. The ECC200 variant uses Reed-Solomon over
GF(256) and has displaced the older ECC000–ECC140 lineage worldwide.

Where it's used:
- **PCBs** — etched marks for traceability through automated assembly
- **Pharmaceuticals** — US FDA DSCSA mandates it on unit-dose packages
- **Aerospace parts** — dot-peened marks survive heat and abrasion
- **Medical devices** — GS1 DataMatrix on surgical instruments and implants
- **Postage** — USPS registered mail and customs forms

## Quick Start

```python
from coding_adventures.data_matrix import encode, encode_and_layout, SymbolShape

# Auto-select smallest square symbol
grid = encode("Hello, World!")
print(grid.rows, grid.cols)   # e.g. 16 16

# Force a specific size
grid = encode("A", size=(10, 10))

# Allow rectangular shapes
grid = encode("HELLO", shape=SymbolShape.Any)

# Encode and produce a PaintScene for rendering
scene = encode_and_layout("scan me")
```

## API

| Function | Description |
|----------|-------------|
| `encode(data, size=None, shape=SymbolShape.Square)` | Encode to `ModuleGrid` |
| `encode_at(data, rows, cols)` | Encode to specific symbol dimensions |
| `layout_grid(grid, config=None)` | Convert `ModuleGrid` → `PaintScene` |
| `encode_and_layout(data, ...)` | Encode + layout in one call |
| `grid_to_string(grid)` | Render as `'0'`/`'1'` text for debugging |

## In the Stack

```
barcode-2d         ← ModuleGrid, PaintScene, layout config
gf256              ← GF(256)/0x12D field arithmetic (RS ECC)
paint-instructions ← PaintScene rendering target
         ↓
data-matrix        ← this package
```

## Running Tests

```sh
cat BUILD   # shows the exact command
```
