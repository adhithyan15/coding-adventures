# coding_adventures_pdf417

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

## What is PDF417?

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes the format's geometry: each codeword
has exactly **4** bars and **4** spaces (8 elements total), and every codeword
occupies exactly **17** modules of horizontal space. "417" = 4 elements × 17
modules.

Unlike true matrix codes (QR, Data Matrix), PDF417 is a **stacked linear**
barcode. It is essentially many rows of a 1D-like encoding stacked vertically.
A single linear scanner sweeping one horizontal row can read that row
independently — row indicator codewords carry enough context to reconstruct
the full symbol from any row.

## Where PDF417 is used

| Application   | Detail                                                     |
|---------------|------------------------------------------------------------|
| AAMVA         | North American driver's licences and government-issued IDs |
| IATA BCBP     | Airline boarding passes (the barcode on your phone)        |
| USPS          | Domestic shipping labels                                   |
| US immigration | Form I-94, customs declarations                           |
| Healthcare    | Patient wristbands, medication labels                      |

## Pipeline position

```
Input data
  → encode()            ← THIS PACKAGE produces a ModuleGrid
  → layout()            ← barcode-2d converts to PaintScene pixels
  → PaintScene          ← consumed by paint-vm backend
  → output (SVG, Canvas, Metal, …)
```

## Encoding pipeline

```
raw bytes
  → byte compaction     (codeword 924 latch + 6-bytes→5-codewords, base-900)
  → length descriptor   (codeword[0] = total codewords in symbol)
  → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
  → dimension selection (auto: roughly square symbol)
  → padding             (codeword 900 fills unused slots)
  → row indicators      (LRI + RRI per row, encode R/C/ECC level)
  → cluster table lookup (codeword → 17-module bar/space pattern)
  → start/stop patterns (fixed per row)
  → ModuleGrid          (abstract boolean grid)
```

## Installation

```yaml
dependencies:
  coding_adventures_pdf417:
    path: ../pdf417
  coding_adventures_barcode_2d:
    path: ../barcode-2d
```

## Quick start

```dart
import 'package:coding_adventures_pdf417/coding_adventures_pdf417.dart';

// Encode a string (UTF-8 bytes) → ModuleGrid
final grid = encodeString('HELLO WORLD');
print(grid.rows);  // e.g. 21 module rows (7 logical × rowHeight 3)
print(grid.cols);  // 120 modules (69 + 17×3 columns)

// Encode raw bytes
final grid2 = encode([0x41, 0x42, 0x43]);

// Specify options
final grid3 = encode(
  'IATA BCBP DATA'.codeUnits,
  options: const Pdf417Options(
    eccLevel: 4,    // Level 4 = 32 ECC codewords (corrects 15 errors)
    columns: 5,     // 5 data columns
    rowHeight: 4,   // 4 module-rows per logical row
  ),
);

// Encode + layout → PaintScene (ready for SVG/Canvas backend)
import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';

final scene = encodeAndLayout(
  'HELLO'.codeUnits,
  layoutConfig: const Barcode2DLayoutConfig(
    moduleSizePx: 3,
    quietZoneModules: 2,
    foreground: '#000000',
    background: '#ffffff',
    moduleShape: ModuleShape.square,
  ),
);
print(scene.width);  // pixel width including quiet zone
```

## v0.1.0 scope

This release implements **byte compaction only** (codeword 924 latch).
All input is treated as raw bytes regardless of content. This produces valid,
scannable PDF417 symbols for any input, but is not as compact as:

- **Text compaction** (v0.2.0): ~2 ASCII chars per codeword
- **Numeric compaction** (v0.2.0): ~2.93 digits per codeword vs. 2.0 in text

## Symbol parameters

| Parameter       | Min | Max | Default       |
|-----------------|-----|-----|---------------|
| Rows            |   3 |  90 | auto-selected |
| Data columns    |   1 |  30 | auto-selected |
| ECC level       |   0 |   8 | auto-selected |
| Row height      |   1 |  10 | 3             |

**ECC auto-selection:**

| Data codewords ≤ | ECC Level | ECC codewords | Corrects errors |
|------------------|-----------|--------------|-----------------|
| 40               |     2     |      8       |       3         |
| 160              |     3     |     16       |       7         |
| 320              |     4     |     32       |      15         |
| 863              |     5     |     64       |      31         |
| ∞                |     6     |    128       |      63         |

## Symbol structure

Each row has fixed start (17 modules) and stop (18 modules) patterns and
exactly `(69 + 17 × columns)` modules total:

```
[START 17] [LRI 17] [data col 0..c-1, each 17] [RRI 17] [STOP 18]
 ←──────────────────────── 69 + 17c modules ─────────────────────→
```

PDF417 cycles through three codeword cluster tables (0, 1, 2) row by row.
The cluster for row `r` is `r % 3`. This lets a scanner identify any row
independently.

## Error types

| Type                    | Thrown when                              |
|-------------------------|------------------------------------------|
| `InvalidEccLevelError`  | `eccLevel` not in 0–8                    |
| `InvalidDimensionsError`| `columns` not in 1–30                   |
| `InputTooLongError`     | Data exceeds 90×30 symbol capacity       |

All error types extend `Pdf417Error`.

## Dependencies

- `coding_adventures_barcode_2d` — `ModuleGrid`, `layout()`, `PaintScene`

## Specification

`code/specs/pdf417.md` in the coding-adventures monorepo.
