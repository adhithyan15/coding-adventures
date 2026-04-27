# CodingAdventures.PDF417

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

## Overview

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes the format's geometry: each codeword has
exactly **4** bars and **4** spaces (8 elements), and every codeword occupies
exactly **17** modules of horizontal space.

PDF417 is a *stacked linear* barcode — not a true 2D matrix code. It is
essentially many rows of a 1D-like encoding stacked vertically. A single linear
scanner sweeping one horizontal row can read that row independently; row
indicator codewords carry enough context to reconstruct the full symbol.

### Where PDF417 is used

| Application | Detail |
|---|---|
| AAMVA | North American driver's licences and government IDs |
| IATA BCBP | Airline boarding passes (the code on your phone) |
| USPS | Domestic shipping labels |
| US immigration | Form I-94, customs declarations |
| Healthcare | Patient wristbands, medication labels |

## Encoding Pipeline

```
raw bytes
  → byte compaction       (codeword 924 latch + 6-bytes→5-codewords base-900)
  → length descriptor     (total codewords in data region)
  → Reed-Solomon ECC      (GF(929), b=3 convention, α=3)
  → dimension selection   (auto: roughly square symbol)
  → padding               (codeword 900 fills unused slots)
  → row indicators        (LRI + RRI per row encode R/C/ECC level)
  → cluster table lookup  (codeword → 17-module bar/space pattern)
  → start/stop patterns   (fixed per row)
  → ModuleGrid            (abstract boolean grid)
```

## Usage

```csharp
using CodingAdventures.PDF417;
using CodingAdventures.Barcode2D;

// Encode a string using default options (byte compaction, auto ECC, auto dimensions).
ModuleGrid grid = PDF417Encoder.Encode("HELLO WORLD");

// Pass to barcode-2d to produce a PaintScene for rendering.
// var scene = Barcode2D.Layout(grid, new Barcode2DLayoutConfig { ModuleSizePx = 3 });

// Encode raw bytes with custom options.
var options = new PDF417Options
{
    EccLevel  = 4,    // explicit ECC level (0–8)
    Columns   = 5,    // explicit column count (1–30)
    RowHeight = 4,    // module-rows per logical row (default: 3)
};
ModuleGrid grid2 = PDF417Encoder.Encode(new byte[] { 0x41, 0x42, 0x43 }, options);
```

## Key Concepts

### Three Clusters

PDF417 uses three different bar/space pattern tables (clusters 0, 1, 2),
cycling row by row. A scanner can detect which cluster a row belongs to and
verify its row indicator codewords — this makes individual rows self-identifying
even without the rest of the symbol.

### GF(929) Reed-Solomon

Unlike QR Code (which uses GF(256)), PDF417 uses Reed-Solomon error correction
over GF(929) — the integers modulo 929. This is possible because 929 is prime,
so modular integer arithmetic defines a valid field. The generator uses the b=3
convention: roots are α^3, α^4, ..., α^{k+2} where α=3 is the primitive root.

### Byte Compaction (v0.1.0)

This release implements byte compaction only. Every 6 input bytes are converted
to 5 base-900 codewords, achieving a 6/5 = 1.2 byte/codeword compression ratio.
Remaining bytes (1–5) are encoded one codeword each. Text and numeric compaction
(higher density for ASCII/digit content) are planned for v0.2.0.

### Row Indicators

Each row carries a Left Row Indicator (LRI) and a Right Row Indicator (RRI).
Together they encode three metadata values distributed across the three clusters:
- **R_info** = (rows − 1) / 3 (total row count)
- **C_info** = cols − 1 (column count)
- **L_info** = 3 × ecc_level + (rows − 1) mod 3 (ECC level + row parity)

A scanner reading any three consecutive rows can recover all symbol metadata.

## Dependencies

| Package | Role |
|---|---|
| `CodingAdventures.Barcode2D` | `ModuleGrid` type and `Layout()` function |

GF(929) field arithmetic is implemented inline (no separate gf929 package needed
for C# since the tables are a small static initializer).

## Error Types

| Exception | When |
|---|---|
| `InvalidECCLevelException` | `EccLevel` outside 0–8 |
| `InvalidDimensionsException` | `Columns` outside 1–30 |
| `InputTooLongException` | Data exceeds 90×30 symbol capacity |

## Dependency Stack Position

```
paint-vm-svg   paint-vm-canvas   paint-metal
       └──────────────┬───────────────────┘
                      │
               paint-vm (P2D01)
                      │
           paint-instructions (P2D00)
                      │
               barcode-2d
                      │
                  pdf417    ← this package
```

## Version

v0.1.0 — byte compaction only.  Text and numeric compaction are planned for v0.2.0.
