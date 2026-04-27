# qr-code (Haskell)

ISO/IEC 18004:2015 QR Code encoder in Haskell.

Encodes any UTF-8 string into a scannable QR Code symbol. Outputs a `ModuleGrid`
(abstract boolean grid) that can be passed to `barcode-2d`'s `layout` function
for pixel rendering.

## What is a QR Code?

A QR Code (Quick Response code) is a 2D matrix barcode invented in 1994. The
symbol is a square grid of dark and light modules. Three finder patterns in the
corners let any scanner locate and orient the symbol. Reed-Solomon error
correction makes the symbol readable even if up to 30% of its area is damaged.

## Encoding pipeline

```
input string
  → mode selection    (numeric / alphanumeric / byte)
  → version selection (smallest v1–40 that fits at the ECC level)
  → bit stream        (mode indicator + char count + data + padding)
  → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
  → interleave        (data CWs round-robin, then ECC CWs)
  → grid init         (finder × 3, separators, timing, alignment, format, dark)
  → zigzag placement  (two-column snake from bottom-right)
  → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
  → finalize          (format info + version info v7+)
  → ModuleGrid
```

## Usage

```haskell
import QRCode
import CodingAdventures.Barcode2D (defaultConfig)

-- Encode a string and get the module grid
case encode "https://example.com" M of
  Left err   -> putStrLn ("Error: " ++ show err)
  Right grid -> print (mgRows grid)  -- 25 (version 2, 25×25)

-- Encode and get a PaintScene ready for rendering
case encodeAndLayout "HELLO WORLD" M defaultConfig of
  Left err   -> putStrLn ("Error: " ++ show err)
  Right scene -> -- pass scene to a paint-vm backend
```

## ECC Levels

| Level | Recovery | Use case |
|-------|----------|----------|
| L     | ~7%      | Clean environments (printed docs) |
| M     | ~15%     | Default for most QR codes |
| Q     | ~25%     | Industrial environments |
| H     | ~30%     | Maximum damage tolerance |

## Dependencies

- `barcode-2d` — `ModuleGrid` type and `layout()` for pixel rendering
- `gf256` — GF(256) field arithmetic for Reed-Solomon ECC

## Building

```
cabal build
cabal test
```

## Architecture

The encoder is a pure function — no I/O, no state, no exceptions. It follows the
standard QR encoding pipeline from ISO/IEC 18004:2015. All lookup tables (capacity,
block structure, alignment positions, format info positions) are embedded as Haskell
constants for reliability.

Reed-Solomon uses the QR-specific b=0 convention:
`g(x) = (x + α⁰)(x + α¹)···(x + α^{n-1})` where α = 2.

This is the Haskell implementation in the coding-adventures multi-language QR
encoder matrix. All language implementations should produce identical `ModuleGrid`
outputs for the same inputs.
