# data-matrix

ISO/IEC 16022:2006 Data Matrix ECC200 encoder for Haskell.

## What is Data Matrix?

Data Matrix is a two-dimensional matrix barcode standardised as ISO/IEC 16022:2006.
ECC200 is the modern variant using Reed-Solomon error correction over GF(256).

## Where is Data Matrix used?

- **PCBs** — every modern board carries a tiny Data Matrix for traceability through automated assembly lines
- **Pharmaceuticals** — the US FDA DSCSA mandates Data Matrix on unit-dose packages
- **Aerospace parts** — etched/dot-peened marks survive decades of heat and abrasion
- **Medical devices** — GS1 DataMatrix on surgical instruments and implants
- **Postage** — USPS registered mail and customs forms

## Features

- 24 square symbols (10×10 → 144×144) and 6 rectangular symbols (8×18 → 16×48)
- ASCII encoding with digit-pair compression (two consecutive digits → one codeword)
- Reed-Solomon over GF(256)/0x12D, b=1 convention
- Multi-block interleaving for burst-error resilience
- L-shaped finder pattern + alternating timing border
- Diagonal "Utah" placement algorithm with 4 corner patterns
- No masking step (diagonal walk distributes bits naturally)
- Integration with `barcode-2d` for pixel rendering

## Quick start

```haskell
import CodingAdventures.DataMatrix

-- Encode "HELLO" automatically into the smallest fitting square symbol
case encode "HELLO" defaultOptions of
  Left err   -> putStrLn ("Error: " ++ show err)
  Right grid -> do
    putStrLn ("Symbol size: " ++ show (mgRows grid) ++ "×" ++ show (mgCols grid))
    -- → "Symbol size: 14×14"

-- Force a specific symbol size
case encodeAt "A" 12 12 of
  Left err   -> putStrLn ("Error: " ++ show err)
  Right grid -> putStrLn "Encoded to 12×12"

-- Rectangular symbols for constrained print areas
let opts = defaultOptions { dmShape = Rectangular }
case encode "HI" opts of
  Left err   -> print err
  Right grid -> putStrLn ("Rect: " ++ show (mgRows grid) ++ "×" ++ show (mgCols grid))
```

## API

| Function | Description |
|----------|-------------|
| `encode` | Encode a string; auto-selects smallest fitting symbol |
| `encodeAt` | Encode to a specific (rows, cols) symbol size |
| `encodeAndLayout` | Encode + convert to `PaintScene` for rendering |
| `defaultOptions` | Default options (square symbols) |

## Errors

| Error | When |
|-------|------|
| `InputTooLong` | Input exceeds maximum capacity (144×144 = 1558 codewords) |
| `InvalidSymbolSize` | `encodeAt` called with a non-ECC200 size |

## Encoding pipeline

```
input string
  → ASCII encoding      (ASCII char + 1; two adjacent digits → single codeword)
  → symbol selection    (smallest symbol whose capacity ≥ codeword count)
  → pad to capacity     (scrambled-pad codewords fill unused slots)
  → RS blocks + ECC     (GF(256)/0x12D, b=1 convention)
  → interleave blocks   (data round-robin then ECC round-robin)
  → grid init           (L-finder + timing border + alignment borders)
  → Utah placement      (diagonal codeword placement, NO masking)
  → ModuleGrid          (abstract boolean grid, True = dark)
```

## Key differences from QR Code

| Property | QR Code | Data Matrix ECC200 |
|----------|---------|-------------------|
| GF(256) poly | 0x11D | 0x12D |
| RS root start | b=0 (α⁰..) | b=1 (α¹..) |
| Finder | three corner squares | one L-shape |
| Placement | column zigzag | "Utah" diagonal |
| Masking | 8 patterns, penalty-scored | NONE |
| Sizes | 40 versions | 30 square + 6 rect |

## Dependencies

- `base >= 4.7`
- `vector >= 0.12`
- `containers >= 0.6`
- `barcode-2d` (for `ModuleGrid` and `layout`)
