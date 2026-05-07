# AztecCode

Swift implementation of the Aztec Code encoder — ISO/IEC 24778:2008 compliant.

## What it does

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995. Unlike
QR Code (which anchors orientation using three square finder patterns in three
corners), Aztec Code uses a single **bullseye finder pattern at the center** of
the symbol. The scanner finds the center first, then reads data outward in a
clockwise spiral — no large quiet zone required.

## Where Aztec Code is used today

- **IATA boarding passes** — the barcode on every airline boarding pass
- **Eurostar and Amtrak rail tickets** — printed and on-screen tickets
- **PostNL, Deutsche Post, La Poste** — European postal routing
- **US military ID cards**

## Symbol variants

```
Compact: 1–4 layers,  size = 11 + 4*layers  (15×15 to 27×27)
Full:    1–32 layers, size = 15 + 4*layers  (19×19 to 143×143)
```

The encoder automatically selects the smallest symbol that fits the input at
the requested ECC level (default: 23%).

## Usage

```swift
import AztecCode

// Encode a string → [[Bool]] grid (true = dark module)
let grid = try AztecCode.encode("Hello, World!")
print(grid.count)          // 15 (compact 1-layer, 15×15)
print(grid[0].count)       // 15

// Encode raw bytes
let bytes: [UInt8] = [0x48, 0x65, 0x6C, 0x6C, 0x6F]
let grid2 = try AztecCode.encodeData(bytes)

// Get a ModuleGrid (for use with Barcode2D)
let moduleGrid = try AztecCode.encodeToGrid("Hello!")

// Higher error correction
let opts = AztecOptions(minEccPercent: 50)
let grid3 = try AztecCode.encode("Important data", options: opts)
```

## Encoding pipeline (v0.1.0)

```
input string / bytes
  → Binary-Shift codewords from Upper mode (byte-mode only, v0.1.0)
  → symbol size selection (smallest compact then full at requested ECC)
  → pad to exact codeword count
  → GF(256)/0x12D Reed-Solomon ECC (same polynomial as Data Matrix)
  → bit stuffing (insert complement after 4 consecutive identical bits)
  → GF(16)/0x13 mode message (RS-protected layer+codeword-count field)
  → ModuleGrid: bullseye → orientation marks → mode msg → data spiral
```

## Where this fits in the stack

```
paint-vm backends (SVG, Metal, Canvas)
         |
    paint-vm (P2D01)
         |
  paint-instructions (P2D00)
         |
      barcode-2d          ← ModuleGrid type, layout()
         |
    aztec-code (this)     ← produces ModuleGrid
         |
  GF(256)/0x12D RS        ← implemented inline (0x12D ≠ repo GF256 poly 0x11D)
  GF(16)/0x13 RS          ← implemented inline for mode message
```

## Dependency note

The repo's `GF256` package uses polynomial **0x11D** (QR Code). Aztec Code
requires **0x12D** (same as Data Matrix ECC200). This package implements both
the GF(256)/0x12D and GF(16)/0x13 RS encoders inline rather than depending on
the repo's `GF256` package.

## v0.1.0 simplifications

1. **Byte-mode only** — all input encoded via Binary-Shift from Upper mode.
   Multi-mode optimisation (Digit/Upper/Lower/Mixed/Punct) is v0.2.0.
2. **8-bit codewords only** — GF(256)/0x12D RS for all data.
3. **Default ECC 23%** — no per-symbol ECC knob in v0.1.0.
4. **Auto-select compact vs full** — no force-compact option.

## Tests

```bash
swift test
# or on macOS:
xcrun swift test
```

62 tests in 9 suites covering: GF(16) arithmetic, GF(256) RS encoding, bit
stuffing, mode message encoding, symbol size selection, bullseye structure,
full encode integration, and cross-language test vectors.
