# micro-qr (Python)

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

Micro QR Code is the compact variant of QR Code, designed for applications where even the smallest standard QR (21×21 at version 1) is too large. Common use cases include surface-mount component labels, circuit board markings, and miniature industrial tags.

## Symbol sizes

```
M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
formula: size = 2 × version_number + 9
```

## Key differences from regular QR Code

- **Single finder pattern** at top-left only (one 7×7 square, not three).
- **Timing at row 0 / col 0** (not row 6 / col 6).
- **Only 4 mask patterns** (not 8).
- **Format XOR mask 0x4445** (not 0x5412).
- **Single copy of format info** (not two).
- **2-module quiet zone** (not 4).
- **Narrower mode indicators** (0–3 bits instead of 4).
- **Single block** (no interleaving).

## Encoding pipeline

```
input string
  → auto-select smallest symbol (M1..M4) and mode
  → build bit stream (mode indicator + char count + data + terminator + padding)
  → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
  → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
  → zigzag data placement (two-column snake from bottom-right)
  → evaluate 4 mask patterns, pick lowest penalty
  → write format information (15 bits, single copy, XOR 0x4445)
  → ModuleGrid
```

## Installation

```bash
pip install coding-adventures-micro-qr
```

## Usage

```python
from micro_qr import encode, encode_at, layout_grid, encode_and_layout
from micro_qr import MicroQRVersion, MicroQREccLevel

# Auto-select smallest symbol
grid = encode("HELLO")
print(grid.rows, grid.cols)  # 13 13

# Force specific version + ECC
grid = encode("12345", version=MicroQRVersion.M1, ecc=MicroQREccLevel.Detection)
print(grid.rows)  # 11

# Encode and get PaintScene for rendering
scene = encode_and_layout("HELLO")
```

## Valid (version, ECC) combinations

| Version | ECC levels available |
|---------|---------------------|
| M1      | Detection           |
| M2      | L, M                |
| M3      | L, M                |
| M4      | L, M, Q             |

## Data capacities (maximum characters)

| Symbol | Numeric | Alphanumeric | Byte |
|--------|---------|--------------|------|
| M1     | 5       | —            | —    |
| M2-L   | 10      | 6            | 4    |
| M2-M   | 8       | 5            | 3    |
| M3-L   | 23      | 14           | 9    |
| M3-M   | 18      | 11           | 7    |
| M4-L   | 35      | 21           | 15   |
| M4-M   | 30      | 18           | 13   |
| M4-Q   | 21      | 13           | 9    |

## How this fits in the stack

```
Input data
  → micro_qr.encode()     ← THIS PACKAGE
  → ModuleGrid
  → barcode_2d.layout()
  → PaintScene
  → paint-vm backend (SVG, PNG, terminal, ...)
```

## License

MIT
