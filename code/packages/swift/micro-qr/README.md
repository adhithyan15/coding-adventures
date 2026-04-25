# MicroQR (Swift)

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

## What it does

This package encodes strings into Micro QR Code symbols. Micro QR Code is
the compact variant of QR Code designed for applications where even the
smallest standard QR (21×21, version 1) is too large — surface-mount
component labels, circuit board markings, miniature industrial tags.

## Symbol sizes

| Symbol | Size     | Numeric cap | Alpha cap | Byte cap |
|--------|----------|-------------|-----------|----------|
| M1     | 11×11    | 5           | —         | —        |
| M2-L   | 13×13    | 10          | 6         | 4        |
| M2-M   | 13×13    | 8           | 5         | 3        |
| M3-L   | 15×15    | 23          | 14        | 9        |
| M3-M   | 15×15    | 18          | 11        | 7        |
| M4-L   | 17×17    | 35          | 21        | 15       |
| M4-M   | 17×17    | 30          | 18        | 13       |
| M4-Q   | 17×17    | 21          | 13        | 9        |

Size formula: `size = 2 × version_number + 9`.

## Key differences from regular QR Code

- **Single finder pattern** at top-left only (one 7×7 square, not three).
- **Timing at row 0 / col 0** (not row 6 / col 6).
- **Only 4 mask patterns** (not 8).
- **Format XOR mask 0x4445** (not 0x5412).
- **Single copy of format info** (not two).
- **2-module quiet zone** (regular QR needs 4).
- **Narrower mode indicators**: 0–3 bits (regular QR always uses 4).
- **Single ECC block** — no interleaving.

## Where it fits in the pipeline

```
input string
  → encode()        ← THIS PACKAGE
  → ModuleGrid      ← universal 2D barcode representation (barcode-2d)
  → layoutGrid()    ← pixel-level conversion (barcode-2d)
  → PaintScene      ← consumed by paint-vm (PaintInstructions)
  → backend (Metal, PNG, SVG, terminal…)
```

## Usage

### Auto-select symbol version

```swift
import MicroQR

// Auto-selects smallest symbol (M1..M4) that can hold the input
let grid = try encode("HELLO")
// grid.rows == 13  (M2 symbol — 5 alphanumeric chars fits in M2-L)
```

### Force a specific version or ECC level

```swift
// Force M4 with highest ECC (25% recovery)
let grid = try encode("HELLO", version: .M4, ecc: .Q)
// grid.rows == 17

// Use the explicit convenience API
let grid2 = try encodeAt("123", version: .M1, ecc: .detection)
// grid2.rows == 11
```

### Render to pixels

```swift
import Barcode2D

let grid = try encode("HELLO")
let scene = try layoutGrid(grid)   // uses 2-module quiet zone by default
// Render scene via PaintVM backend
```

## Encoding pipeline

```
input string
  → auto-select smallest symbol (M1..M4) and mode
  → build bit stream (mode indicator + char count + data + terminator + pad)
  → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
  → initialize grid (finder, L-shaped separator, timing at row 0/col 0)
  → zigzag data placement (two-column snake from bottom-right)
  → evaluate 4 mask patterns, pick lowest penalty
  → write format information (15 bits, single copy, XOR 0x4445)
  → ModuleGrid
```

## Error types

| Error | Cause |
|-------|-------|
| `inputTooLong` | Input exceeds M4 capacity (35 numeric chars max) |
| `eccNotAvailable` | Requested (version, ECC) combo is invalid |
| `unsupportedMode` | No mode can encode the input in the chosen symbol |
| `invalidCharacter` | Character outside the supported set for the mode |
| `layoutError` | Error from the barcode-2d rendering layer |

## Dependencies

- **GF256** — Galois Field GF(2^8) arithmetic for Reed-Solomon ECC
- **Barcode2D** — `ModuleGrid` type and `layout()` function
- **PaintInstructions** — `PaintScene` type for the render pipeline

## Part of the coding-adventures stack

This package sits in layer DT12 (2D barcode formats) of the educational
computing stack, alongside the QR Code, Data Matrix, and Aztec Code encoders.
All implementations — Swift, Rust, Python, Ruby, TypeScript, Go, Elixir — share
the same test corpus and must agree on which symbol version is selected for each
input.
