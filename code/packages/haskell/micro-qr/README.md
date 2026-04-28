# micro-qr

ISO/IEC 18004:2015 Annex E Micro QR Code encoder for Haskell.

## What is Micro QR Code?

Micro QR Code is the compact variant of QR Code, designed for applications where even
the smallest standard QR (21×21 at version 1) is too large. Common use cases include
surface-mount component labels, circuit board markings, and miniature industrial tags.

## Symbol sizes

```
M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
formula: size = 2 × version_number + 9
```

## Features

- All four Micro QR symbols: M1 (11×11) through M4 (17×17)
- Encoding modes: Numeric (M1+), Alphanumeric (M2+), Byte (M3+)
- ECC levels: Detection (M1), L/M (M2–M4), Q (M4 only)
- Single 7×7 finder pattern at top-left
- Reed-Solomon error correction (GF(256)/0x11D, b=0 convention)
- 4 mask patterns with ISO penalty scoring
- 15-bit BCH format information (single copy, XOR mask 0x4445)
- Integration with `barcode-2d` for pixel rendering

## Quick start

```haskell
import CodingAdventures.MicroQR

-- Auto-select the smallest symbol
case encode "1" Nothing Nothing of
  Right grid -> putStrLn ("Size: " ++ show (mgRows grid))  -- "Size: 11"
  Left err   -> print err

-- Force M2 with L error correction
case encodeAt "HELLO" M2 L of
  Right grid -> putStrLn ("M2: " ++ show (mgRows grid) ++ "×" ++ show (mgCols grid))
  Left err   -> print err

-- Force M4 with Q for maximum error correction
case encodeAt "1" M4 Q of
  Right grid -> putStrLn ("M4/Q grid: 17×17")
  Left err   -> print err
```

## API

| Function | Description |
|----------|-------------|
| `encode` | Encode a string; auto-selects smallest fitting symbol |
| `encodeAt` | Encode to a specific version + ECC level |
| `encodeAndLayout` | Encode + convert to `PaintScene` for rendering |

## Symbol configurations

| Symbol | ECC | Size | Numeric | Alpha | Byte |
|--------|-----|------|---------|-------|------|
| M1 | Detection | 11×11 | 5 | — | — |
| M2 | L | 13×13 | 10 | 6 | 4 |
| M2 | M | 13×13 | 8 | 5 | 3 |
| M3 | L | 15×15 | 23 | 14 | 9 |
| M3 | M | 15×15 | 18 | 11 | 7 |
| M4 | L | 17×17 | 35 | 21 | 15 |
| M4 | M | 17×17 | 30 | 18 | 13 |
| M4 | Q | 17×17 | 21 | 13 | 9 |

## Errors

| Error | When |
|-------|------|
| `InputTooLong` | Input exceeds M4 capacity (35 numeric chars) |
| `ECCNotAvailable` | Requested ECC level not available for the symbol |
| `UnsupportedMode` | Input requires a mode not available in the chosen symbol |
| `InvalidConfiguration` | (version, ecc) combination is invalid |

## Key differences from regular QR Code

| Property | Regular QR | Micro QR |
|----------|-----------|----------|
| Finder patterns | 3 (one at each corner) | 1 (top-left only) |
| Timing strip location | Row 6, col 6 | Row 0, col 0 |
| Mask patterns | 8 | 4 |
| Format XOR | 0x5412 | 0x4445 |
| Format info copies | 2 | 1 |
| Quiet zone | 4 modules | 2 modules |
| Mode indicator bits | 4 | 0–3 (depends on version) |
| Block structure | Multi-block | Single block |

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

## Dependencies

- `base >= 4.7`
- `vector >= 0.12`
- `gf256` (for GF arithmetic utilities)
- `barcode-2d` (for `ModuleGrid` and `layout`)
