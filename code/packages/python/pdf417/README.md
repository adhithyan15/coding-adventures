# pdf417 (Python)

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

## What is PDF417?

PDF417 (**P**ortable **D**ata **F**ile **417**) was invented by Ynjiun P.
Wang at Symbol Technologies in 1991.  The name encodes its geometry: each
codeword has exactly **4** bars and **4** spaces (8 elements), and every
codeword occupies exactly **17** modules of horizontal space.

Unlike true 2D matrix codes (QR, Data Matrix, Aztec), PDF417 is a **stacked
linear** barcode.  It is essentially many rows of a 1D-like encoding stacked
vertically.  A single linear scanner sweeping one horizontal row can read
that row independently — the row indicator codewords carry enough context to
reconstruct the full symbol from any row.

## Where PDF417 is used

| Application     | Detail                                            |
|-----------------|---------------------------------------------------|
| AAMVA           | North American driver's licences and government IDs |
| IATA BCBP       | Airline boarding passes                           |
| USPS            | Domestic shipping labels                          |
| US immigration  | Form I-94, customs declarations                   |
| Healthcare      | Patient wristbands, medication labels             |

## How it fits in the stack

```
pdf417 (this package)
   ├── barcode-2d    (ModuleGrid type, layout())
   └── paint-instructions  (PaintScene type)
```

## Usage

```python
from pdf417 import encode, grid_to_string

# Encode a string — auto-selects ECC level and symbol dimensions.
grid = encode("HELLO WORLD")

# grid.rows, grid.cols — symbol dimensions in modules
print(f"Symbol: {grid.rows} × {grid.cols} modules")

# grid_to_string() renders as '0'/'1' for debugging.
print(grid_to_string(grid))
```

### Options

```python
grid = encode(
    "HELLO WORLD",
    ecc_level=3,    # Reed-Solomon ECC level 0–8 (default: auto)
    columns=5,      # Number of data columns 1–30 (default: auto)
    row_height=4,   # Module-rows per logical row (default: 3)
)
```

### Error handling

```python
from pdf417 import (
    PDF417Error,
    InputTooLongError,
    InvalidDimensionsError,
    InvalidECCLevelError,
    encode,
)

try:
    grid = encode(very_long_string, columns=1)
except InputTooLongError as e:
    print(f"Input too long: {e}")
except PDF417Error as e:
    print(f"Encoding error: {e}")
```

## Algorithm overview

### Encoding pipeline

1. **Byte compact** the UTF-8 bytes (codeword 924 latch, 6→5 compression).
2. **Auto-select ECC level** (or use the ``ecc_level`` parameter).
3. **Length descriptor**: codeword 0 = total codewords in symbol.
4. **Reed-Solomon ECC** over GF(929) with b=3 convention, α=3.
5. **Choose dimensions**: `cols = ceil(√(total/3))`, clamped to 1–30.
6. **Pad** unused slots with codeword 900.
7. **Rasterize**: per row, emit start + LRI + data + RRI + stop.

### GF(929) — the prime field

PDF417 uses Reed-Solomon over **GF(929)**, not GF(256).  Since 929 is prime,
GF(929) is simply the integers modulo 929.  This means addition is ordinary
modular arithmetic (not XOR as in GF(256)):

```
add(a, b) = (a + b) mod 929
mul(a, b) via log/antilog tables for O(1) performance
```

The generator element α = 3 is the primitive root mod 929 (specified in
ISO/IEC 15438:2015, Annex A.4).

### Three-cluster encoding

Each row uses one of three codeword-to-barcode mappings (clusters), cycling
as `row % 3`:

```
row % 3 == 0  →  cluster 0
row % 3 == 1  →  cluster 1  (ISO calls this "cluster 3")
row % 3 == 2  →  cluster 2  (ISO calls this "cluster 6")
```

This makes each row's cluster identifiable from the bar/space patterns alone —
a scanner can verify which cluster it is reading and detect misalignment.

### Row indicators

Every row carries a Left Row Indicator (LRI) and Right Row Indicator (RRI).
Together they encode R (total rows), C (data columns), and L (ECC level).
A scanner reading any three consecutive rows (one of each cluster) can fully
recover R, C, and L — enabling robust partial-read recovery.

## v0.1.0 scope

This release implements **byte compaction only**.  All input (even pure ASCII
or digits) is encoded via the byte path.  Text and numeric compaction are
planned for v0.2.0 and will yield denser symbols for ASCII/digit inputs.

## Dependencies

- `coding-adventures-barcode-2d` — provides `ModuleGrid`, `make_module_grid`,
  `set_module`, `layout`, `Barcode2DLayoutConfig`.
- `coding-adventures-paint-instructions` — provides `PaintScene`.

## Development

```bash
uv venv
uv pip install -e ../paint-instructions
uv pip install -e ../barcode-2d
uv pip install -e ".[dev]"
pytest tests/ -v
```
