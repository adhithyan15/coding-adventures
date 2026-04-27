# qr-code (Rust)

QR Code encoder — ISO/IEC 18004:2015 compliant.

Encodes any UTF-8 string into a scannable QR Code. Outputs an abstract
`ModuleGrid` (no pixel coordinates yet) that can be passed to `barcode-2d`'s
`layout()` function for pixel-level rendering.

## Pipeline

```
input string
  → mode selection    (numeric / alphanumeric / byte)
  → version selection (smallest version 1–40 that fits at the ECC level)
  → bit stream        (mode indicator + char count + data bits + padding)
  → blocks + RS ECC   (GF(256) b=0 convention, polynomial 0x11D)
  → interleave        (data CWs, then ECC CWs, round-robin across blocks)
  → grid init         (finder × 3, separator, timing, alignment, format, dark)
  → zigzag placement  (two-column snake from bottom-right corner)
  → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
  → finalize          (format info + version info v7+)
  → ModuleGrid        (abstract boolean grid, true = dark)
```

## Usage

```rust
use qr_code::{encode, encode_and_layout, EccLevel};
use barcode_2d::Barcode2DLayoutConfig;

// Just the module grid (abstract units — no pixels):
let grid = encode("https://example.com", EccLevel::M).unwrap();
assert_eq!(grid.rows, 25); // version 2 → 25×25

// Grid → pixel-resolved PaintScene (via barcode-2d):
let config = Barcode2DLayoutConfig::default();
let scene = encode_and_layout("HELLO WORLD", EccLevel::M, &config).unwrap();
assert!(scene.width > 0.0);
```

## API

### `encode(input, ecc) → Result<ModuleGrid, QRCodeError>`

Encodes a UTF-8 string into a QR Code module grid.

- Returns a `(4V+17) × (4V+17)` grid where `true` = dark module.
- Returns `Err(QRCodeError::InputTooLong)` if input exceeds version-40 capacity.
- Selects the minimum version that fits the input at the ECC level.
- Mode is auto-selected: numeric → alphanumeric → byte.

### `encode_and_layout(input, ecc, config) → Result<PaintScene, QRCodeError>`

Encodes and converts to a pixel-resolved `PaintScene` via `barcode-2d`'s
`layout()`. Accepts a `&Barcode2DLayoutConfig`.

## ECC Levels

| Level | Recovery | Notes                                  |
|-------|----------|----------------------------------------|
| L     | ~7%      | Highest data density                   |
| M     | ~15%     | General-purpose default                |
| Q     | ~25%     | Moderate damage expected               |
| H     | ~30%     | High damage risk, or overlaid logo     |

## Error Types

- `QRCodeError::InputTooLong` — input exceeds version-40 capacity at the chosen ECC level.

## Dependencies

- `barcode-2d`: `ModuleGrid` type, `layout()` pixel geometry
- `gf256`: GF(256) field arithmetic (`multiply`, `power`)
- `paint-instructions`: `PaintScene` type (for `encode_and_layout` return type)

## Spec

See `code/specs/qr-code.md` for the full specification.
