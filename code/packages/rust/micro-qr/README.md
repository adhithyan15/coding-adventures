# micro-qr

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

Encodes any string into a scannable Micro QR Code symbol (M1–M4).
Outputs a [`ModuleGrid`] (abstract boolean grid) that can be passed to
`barcode-2d`'s `layout()` for pixel rendering.

## Symbol Sizes

| Symbol | Size | Max numeric | Max alphanumeric | Max bytes |
|--------|------|-------------|------------------|-----------|
| M1     | 11×11 | 5           | —               | —         |
| M2     | 13×13 | 10          | 6               | 4         |
| M3     | 15×15 | 23          | 14              | 9         |
| M4     | 17×17 | 35          | 21              | 15        |

## Usage

```rust
use micro_qr::{encode, MicroQRVersion, MicroQREccLevel};

// Auto-select smallest symbol
let grid = encode("HELLO", None, None).unwrap();
assert_eq!(grid.rows, 13); // M2 symbol (13×13)

// Force M4 with Q error correction
let m4 = encode("https://a.b", Some(MicroQRVersion::M4), Some(MicroQREccLevel::L)).unwrap();
assert_eq!(m4.rows, 17);

// Convert to a PaintScene for rendering
use micro_qr::layout_grid;
let scene = layout_grid(&grid, None).unwrap();
```

## Key Differences from Regular QR Code

| Feature | Regular QR | Micro QR |
|---------|-----------|----------|
| Finder patterns | 3 | 1 |
| Timing strips | Row 6, col 6 | Row 0, col 0 |
| Quiet zone | 4 modules | 2 modules |
| ECC levels | L, M, Q, H | Detection, L, M, Q |
| Mask patterns | 8 | 4 |
| Format info copies | 2 | 1 |
| Format XOR mask | 0x5412 | 0x4445 |
| Max capacity | 7089 numeric | 35 numeric |

## Dependencies

- `barcode-2d` — `ModuleGrid` type and `layout()` function
- `gf256` — GF(256) field arithmetic for the Reed-Solomon encoder
- `paint-instructions` — `PaintScene` type for the layout output
