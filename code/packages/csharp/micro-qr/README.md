# CodingAdventures.MicroQR

ISO/IEC 18004:2015 Annex E **Micro QR Code** encoder for C#.

Micro QR Code is the compact variant of regular QR Code, designed for applications
where even the smallest standard QR (21×21 at version 1) is too large. Think
surface-mount component labels, circuit board markings, and miniature industrial tags.

## What makes Micro QR different?

Regular QR Code uses three identical 7×7 finder patterns (the "eyes" at three corners)
so a scanner can determine orientation from any angle. Micro QR uses only **one** finder,
placed in the top-left. Because there is only one, orientation is always unambiguous —
the data area is always to the bottom-right of the finder. This single omission is
responsible for most of the space saving.

## Symbol sizes

| Symbol | Size    | Max numeric | Max alphanumeric | Max bytes |
|--------|---------|-------------|-----------------|-----------|
| M1     | 11×11   | 5           | —               | —         |
| M2     | 13×13   | 10 (L)      | 6 (L)           | 4 (L)     |
| M3     | 15×15   | 23 (L)      | 14 (L)          | 9 (L)     |
| M4     | 17×17   | 35 (L)      | 21 (L)          | 15 (L)    |

Size formula: `size = 2 × version_number + 9`.

## Usage

```csharp
using CodingAdventures.MicroQR;

// Auto-select smallest symbol that fits (returns a ModuleGrid)
var grid = MicroQR.Encode("HELLO");     // → M2, 13×13
var grid = MicroQR.Encode("12345");     // → M1, 11×11
var grid = MicroQR.Encode("hello");     // → M3, 15×15 (lowercase needs byte mode)
var grid = MicroQR.Encode("https://a.b"); // → M4, 17×17

// Force a specific version and ECC level
var grid = MicroQR.Encode("MICRO QR", MicroQRVersion.M4, MicroQREccLevel.Q);

// grid.Rows and grid.Cols give the symbol size
// grid.Modules[row][col] is true for a dark module
```

## Dependencies

- `CodingAdventures.Barcode2D` — `ModuleGrid` type and `ModuleShape` enum
- `CodingAdventures.Gf256` — GF(256) arithmetic for Reed-Solomon ECC

## Encoding pipeline

```
input string
  → auto-select smallest symbol (M1–M4) and mode (numeric/alphanumeric/byte)
  → build bit stream (mode indicator + char count + data + terminator + padding)
  → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
  → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
  → zigzag data placement (two-column snake from bottom-right)
  → evaluate 4 mask patterns, pick lowest penalty
  → write format information (15 bits, single copy, XOR 0x4445)
  → ModuleGrid
```

## ECC levels

| Level      | Available in | Recovery |
|------------|-------------|---------|
| Detection  | M1 only     | error detection only |
| L          | M2, M3, M4  | ~7% of codewords |
| M          | M2, M3, M4  | ~15% of codewords |
| Q          | M4 only     | ~25% of codewords |

Level H (30%) is not available in any Micro QR symbol — the symbols are too small
to afford that much redundancy.

## Package matrix

This package is part of the coding-adventures 2D barcode stack. All 15 language
implementations produce identical `ModuleGrid` outputs for the same input.

## Running tests

```bash
dotnet test tests/CodingAdventures.MicroQR.Tests/CodingAdventures.MicroQR.Tests.csproj
```

Or via the BUILD script:

```bash
bash BUILD
```
