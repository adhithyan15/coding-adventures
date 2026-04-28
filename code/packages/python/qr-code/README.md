# coding-adventures-qr-code

A complete, ISO/IEC 18004:2015 compliant QR Code encoder in Python.

Given any UTF-8 string (or raw bytes) and an error-correction level, this
package produces a `ModuleGrid` — an abstract 2-D boolean grid where
`True` = dark module.  The grid integrates directly with the
`barcode-2d` / `paint-vm` rendering pipeline.

## Where this fits

```
Input string
  → qr-code (this package)   ← encodes to ModuleGrid
  → barcode-2d layout()      ← converts modules to pixel coordinates
  → PaintScene               ← consumed by paint-vm backends
  → SVG / ASCII / Metal / …
```

## Installation

```bash
pip install coding-adventures-qr-code
```

Or from the monorepo (editable):

```bash
uv pip install -e code/packages/python/qr-code
```

## Quick start

```python
from qr_code import encode, encode_to_scene

# Encode to abstract module grid
grid = encode("Hello, World!", level="M")
print(f"Symbol size: {grid.rows} × {grid.cols}")
# → Symbol size: 21 × 21

# Encode and convert to PaintScene for rendering
from barcode_2d import Barcode2DLayoutConfig
scene = encode_to_scene(
    "Hello, World!",
    level="M",
    config=Barcode2DLayoutConfig(module_size_px=10, quiet_zone_modules=4),
)
```

## API

### `encode(data, *, level="M", version=0, mode=None) -> ModuleGrid`

Main entry point.

| Parameter | Type | Description |
|-----------|------|-------------|
| `data` | `str \| bytes` | Input to encode |
| `level` | `"L"` / `"M"` / `"Q"` / `"H"` | Error correction level |
| `version` | `int` 0–40 | Force a specific version (0 = auto) |
| `mode` | `None` / `"numeric"` / `"alphanumeric"` / `"byte"` | Force encoding mode (None = auto) |

Returns a `ModuleGrid` of size `(4v+17) × (4v+17)` where `v` is the chosen version.

Raises `InputTooLongError` if the input exceeds QR Code v40 capacity.

### `encode_to_scene(data, *, level, version, mode, config) -> PaintScene`

Convenience wrapper that calls `encode()` and passes the result through
`barcode_2d.layout()`.

## Error correction levels

| Level | Recovery | Use case |
|-------|----------|----------|
| L | ~7% | Maximum data density, clean environment |
| M | ~15% | General purpose (default) |
| Q | ~25% | Outdoor / industrial |
| H | ~30% | Logo overlay, high damage risk |

## Encoding modes

| Mode | Characters | Density |
|------|-----------|---------|
| Numeric | `0-9` | 3.33 bits/char |
| Alphanumeric | `0-9`, `A-Z`, ` $%*+-./:` | 5.5 bits/char |
| Byte | Any UTF-8 | 8 bits/byte |

Mode is auto-selected (most compact that covers the entire input).

## Encoding pipeline

```
Input string
  → mode selection       (numeric / alphanumeric / byte)
  → version selection    (smallest v1–40 that fits at chosen ECC)
  → bit stream assembly  (mode indicator + char count + data + padding)
  → block splitting      (ISO block table: group-1 + group-2 blocks)
  → RS ECC computation   (GF(256), b=0 convention, generator 0x11D)
  → interleaving         (round-robin across blocks)
  → grid initialisation  (finder, separator, timing, alignment, dark module)
  → zigzag data placement
  → mask evaluation      (8 candidates, lowest 4-rule penalty wins)
  → format info write    (BCH(15,5), XOR 0x5412)
  → version info write   (BCH(18,6) for v7+)
  → ModuleGrid
```

## Dependencies

- `coding-adventures-barcode-2d` — `ModuleGrid`, `layout()`, `PaintScene`
- `coding-adventures-gf256` — GF(256) arithmetic (multiply, ALOG table)
- `coding-adventures-paint-instructions` — `PaintScene` type (transitive)

## Testing

```bash
cd code/packages/python/qr-code
mise exec -- uv run pytest tests/ -v --cov=qr_code
```

## Specification

See `code/specs/` for the QR Code specification document.
