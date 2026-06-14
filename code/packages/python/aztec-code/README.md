# aztec-code (Python)

Aztec Code encoder — ISO/IEC 24778:2008 compliant.

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and published as a patent-free format. Unlike QR Code (which uses three square finder patterns at three corners), Aztec Code places a single **bullseye finder pattern at the center** of the symbol. The scanner finds the centre first, then reads outward in a spiral — no large quiet zone is needed.

## Where Aztec Code is used today

- **IATA boarding passes** — the barcode on every airline boarding pass.
- **Eurostar and Amtrak rail tickets** — printed and on-screen tickets.
- **PostNL, Deutsche Post, La Poste** — European postal routing.
- **US military ID cards.**

## Symbol variants

```
Compact: 1-4 layers,  size = 11 + 4*layers   (15x15 to 27x27)
Full:    1-32 layers, size = 15 + 4*layers   (19x19 to 143x143)
```

## Encoding pipeline (v0.1.0)

```
input string / bytes
  -> Binary-Shift codewords from Upper mode
  -> symbol size selection (smallest compact then full that fits at 23% ECC)
  -> pad to exact codeword count
  -> GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots)
  -> bit stuffing (insert complement after 4 consecutive identical bits)
  -> GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
  -> ModuleGrid  (bullseye -> orientation marks -> mode msg -> data spiral)
```

## Installation

```bash
pip install coding-adventures-aztec-code
```

## Usage

```python
from aztec_code import encode, encode_and_layout, AztecOptions

# Auto-select the smallest symbol (defaults to 23% ECC).
grid = encode("HELLO")
print(grid.rows, grid.cols)  # 15 15

# Higher ECC for noisy environments.
grid = encode("HELLO", AztecOptions(min_ecc_percent=50))

# Bytes input is also accepted (no UTF-8 conversion applied).
grid = encode(b"\x00\x01\x02\x03")

# Convert directly to a PaintScene.
scene = encode_and_layout("HELLO")
```

## v0.1.0 simplifications

1. **Byte-mode only** — all input is wrapped in a single Binary-Shift block from Upper mode. Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimisation is planned for v0.2.0.
2. **8-bit codewords** -> GF(256)/0x12D Reed-Solomon. GF(16) and GF(32) RS for 4-bit / 5-bit codewords on smaller symbols are v0.2.0.
3. **Default ECC = 23%.**
4. **Auto-select compact vs full** (force-compact / force-layers options are v0.2.0).

## Symbol-size selection

The encoder tries compact 1, 2, 3, 4 first, then full 1, 2, …, 32. It picks the smallest symbol whose data-codeword capacity (after subtracting the ECC budget) can hold the bit-stuffed data — using a conservative 20% upper bound on the bit-stuffing overhead.

If even a 32-layer full symbol cannot hold the input (~1914 bytes at 23% ECC), the encoder raises `InputTooLongError`.

## How this fits in the stack

```
Input data
  -> aztec_code.encode()      <- THIS PACKAGE
  -> ModuleGrid
  -> barcode_2d.layout()
  -> PaintScene
  -> paint-vm backend (SVG, PNG, terminal, ...)
```

## API

| Symbol                | Purpose                                                    |
|-----------------------|------------------------------------------------------------|
| `encode`              | string or bytes -> `ModuleGrid`                            |
| `encode_and_layout`   | string or bytes -> `PaintScene`                            |
| `layout_grid`         | `ModuleGrid` -> `PaintScene` (thin re-export)              |
| `explain`             | string or bytes -> `AnnotatedModuleGrid`                   |
| `AztecOptions`        | encoder options (currently `min_ecc_percent`)              |
| `AztecError`          | base exception                                             |
| `InputTooLongError`   | raised when the input exceeds maximum capacity             |

## License

MIT
