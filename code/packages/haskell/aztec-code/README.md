# aztec-code

ISO/IEC 24778:2008 Aztec Code encoder for Haskell.

## What is Aztec Code?

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
published as a patent-free format. Unlike QR Code (which places finder patterns
at three corners), Aztec Code uses a single **bullseye finder pattern at the
centre** of the symbol. A scanner finds the centre first, then reads outward in
a clockwise spiral — no large quiet zone is required.

## Where is Aztec Code used?

- **IATA boarding passes** — the barcode on every airline boarding pass worldwide
- **Eurostar and Amtrak rail tickets** — printed and on-screen mobile tickets
- **PostNL, Deutsche Post, La Poste** — European postal routing labels
- **US military ID cards** — Common Access Card (CAC)
- **US driver's licences** — AAMVA standard in many states

## Features

- Compact Aztec (1–4 layers, 15×15 to 27×27)
- Full Aztec (1–32 layers, 19×19 to 143×143)
- Binary-Shift byte-mode encoding (v0.1.0)
- GF(256)/0x12D Reed-Solomon ECC (same polynomial as Data Matrix)
- GF(16) Reed-Solomon for the mode message
- Bit stuffing (complement inserted after 4 identical consecutive bits)
- Bullseye finder pattern with concentric alternating rings
- Orientation marks (4 dark corner modules of the mode message ring)
- Reference grid for full symbols (alternating dark/light, every 16 modules from centre)
- Clockwise layer spiral data placement
- Integration with `barcode-2d` for pixel rendering

## Quick start

```haskell
import CodingAdventures.AztecCode
import Data.Char (ord)

-- Encode "Hello" automatically into the smallest fitting symbol
case encodeAztecCode "Hello" of
  Left err  -> putStrLn ("Error: " ++ show err)
  Right mat ->
    putStrLn ("Size: " ++ show (length mat) ++ "×" ++ show (length (head mat)))
-- → "Size: 15×15"

-- Encode raw bytes with custom ECC
let opts = defaultOptions { azMinEccPercent = 33 }
case encodeWithOptions (map ord "https://example.com") opts of
  Left err  -> print err
  Right g   -> putStrLn ("Rows: " ++ show (mgRows g))

-- Encode + layout (for PaintVM rendering)
import CodingAdventures.Barcode2D (defaultConfig)
case encodeAndLayout (map ord "A") defaultOptions defaultConfig of
  Left err   -> print err
  Right scene -> putStrLn "Got PaintScene"
```

## API

| Function | Description |
|----------|-------------|
| `encodeAztecCode` | Encode a `String`; returns `[[Bool]]` grid |
| `encodeWithOptions` | Encode bytes with options; returns `ModuleGrid` |
| `encodeAndLayout` | Encode + convert to `PaintScene` for rendering |
| `defaultOptions` | Default options (23% ECC, auto compact/full) |

## Errors

| Error | When |
|-------|------|
| `InputTooLong` | Input exceeds 32-layer full symbol capacity (~3471 bytes at 23% ECC) |

## Encoding pipeline

```
input bytes
  → Binary-Shift codewords from Upper mode
  → symbol size selection (smallest compact then full at requested ECC%)
  → pad to exact codeword count
  → GF(256)/0x12D Reed-Solomon ECC (b=1 convention, same as Data Matrix)
  → bit stuffing (insert complement after 4 identical consecutive bits)
  → GF(16) mode message (layers + codeword count, 5–6 RS check nibbles)
  → grid init (reference grid → bullseye → orientation marks → mode msg)
  → clockwise layer spiral data placement
  → ModuleGrid (abstract boolean grid, True = dark)
```

## Symbol size formula

```
Compact N layers: size = 11 + 4×N   (N ∈ {1,2,3,4})
Full N layers:    size = 15 + 4×N   (N ∈ {1..32})
```

## GF polynomials

| Purpose | Field | Primitive polynomial |
|---------|-------|---------------------|
| Data codewords (byte mode) | GF(256) | 0x12D (same as Data Matrix) |
| Mode message | GF(16) | 0x13 (x^4 + x + 1) |

Note: Aztec's GF(256) polynomial (0x12D) differs from QR Code (0x11D).

## v0.1.0 simplifications

1. **Byte-mode only** — all input via Binary-Shift from Upper mode.
   Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimisation is v0.2.0.
2. **GF(256) RS only** — 8-bit codewords throughout.
   GF(32) for 5-bit codewords is v0.2.0.
3. **Default ECC = 23%** — configurable via `azMinEccPercent`.

## Dependencies

- `base >= 4.7`
- `vector >= 0.12`
- `containers >= 0.6`
- `barcode-2d` — for `ModuleGrid` and `layout`
- `paint-instructions` — for `PaintScene`
