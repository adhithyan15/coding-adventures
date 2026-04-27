# kotlin/pdf417

ISO/IEC 15438:2015-compliant **PDF417** stacked linear barcode encoder for Kotlin.

## What is PDF417?

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes its geometry:

- **4** bars + **4** spaces per codeword = 8 elements per codeword
- Every codeword occupies exactly **17** horizontal modules
- "417" = **4** elements × **17** modules

Unlike 2D matrix barcodes (QR Code, Data Matrix), PDF417 is a *stacked linear*
barcode — a stack of 1D bar codes that share structural metadata. This makes
it readable by simpler linear scanners when scanned row by row.

## Where PDF417 is used

| Application    | Detail                                               |
|----------------|------------------------------------------------------|
| AAMVA          | North American driver's licences and government IDs  |
| IATA BCBP      | Airline boarding passes (PDF417 on your boarding pass) |
| USPS           | Domestic shipping labels                             |
| US immigration | Form I-94, customs declarations                      |
| Healthcare     | Patient wristbands, medication labels                |

## Where this fits in the stack

```
Input string / bytes
  → PDF417 encoder (THIS PACKAGE)
  → ModuleGrid                       ← 2D boolean grid
  → barcode-2d layout()
  → PaintScene
  → paint-vm backend (SVG, PNG, Canvas, Metal, terminal…)
```

This package produces only the abstract boolean `ModuleGrid`. Rendering to
pixels is handled by `barcode-2d` and `paint-instructions`.

## Quick start

```kotlin
import com.codingadventures.pdf417.encodeString
import com.codingadventures.pdf417.PDF417Options
import com.codingadventures.barcode2d.layout
import com.codingadventures.barcode2d.Barcode2DLayoutConfig

// Encode with defaults (auto ECC, auto columns)
val grid = encodeString("Hello, PDF417!")

// Encode with explicit options
val grid = encodeString(
    "IATA boarding pass data",
    PDF417Options(eccLevel = 5, columns = 10, rowHeight = 3)
)

// Render to a PaintScene (for SVG, PNG, etc.)
val scene = layout(grid, Barcode2DLayoutConfig(moduleSizePx = 3))
```

## API

### `encodeString(text: String, options: PDF417Options = PDF417Options()): ModuleGrid`

Encode a UTF-8 string. Returns a `ModuleGrid`.

### `encode(bytes: ByteArray, options: PDF417Options = PDF417Options()): ModuleGrid`

Encode raw bytes. Returns a `ModuleGrid`.

### `PDF417Options`

| Field        | Type    | Default  | Description                                                |
|-------------|---------|----------|------------------------------------------------------------|
| `eccLevel`  | `Int?`  | `null`   | ECC level 0–8. `null` = auto-select based on data length.  |
| `columns`   | `Int?`  | `null`   | Data columns 1–30. `null` = auto-select for square symbol. |
| `rowHeight` | `Int?`  | `null`   | Pixel rows per logical row (≥1). Default: 3.               |

### Error types

```kotlin
sealed class PDF417Error : RuntimeException {
    class InputTooLong(msg: String)       // data exceeds max symbol capacity
    class InvalidDimensions(msg: String)  // columns out of 1–30 range
    class InvalidECCLevel(msg: String)    // ECC level out of 0–8 range
}
```

## Encoding algorithm (v0.1.0: byte compaction only)

```
raw bytes
  → byte compaction   (latch 924 + 6-bytes-to-5-codewords base-900)
  → length descriptor (total codewords incl. ECC, not incl. padding)
  → Reed-Solomon ECC  (GF(929), b=3 convention, α=3, level 0–8)
  → dimension select  (auto: roughly square symbol)
  → padding           (codeword 900 fills unused grid slots)
  → row indicators    (LRI + RRI: R/C/ECC level per row)
  → cluster lookup    (codeword → 17-module bar/space pattern)
  → start/stop        (fixed 17- and 18-module patterns per row)
  → ModuleGrid        (boolean 2D grid)
```

Text compaction and numeric compaction are planned for v0.2.0.

## ECC level guide

| Level | ECC codewords | Approx. damage recovery |
|------:|:-------------:|:------------------------|
|   0   |       2       | minimal                 |
|   1   |       4       | ~7%                     |
|   2   |       8       | ~15%                    |
|   3   |      16       | ~25%                    |
|   4   |      32       | ~40%                    |
|   5   |      64       | ~50%                    |
|   6   |     128       | ~57%                    |
|   7   |     256       | ~64%                    |
|   8   |     512       | ~72%                    |

Auto-selection thresholds:

| Data codewords | Auto level |
|---------------:|:----------:|
|       ≤ 40     |     2      |
|       ≤ 160    |     3      |
|       ≤ 320    |     4      |
|       ≤ 863    |     5      |
|        > 863   |     6      |

## Symbol dimensions

A PDF417 symbol has:
- **Rows**: 3–90 logical rows
- **Data columns**: 1–30 per row
- **Module width** per row: `69 + 17 × columns` modules
  - 17 (start) + 17 (LRI) + 17×cols (data) + 17 (RRI) + 18 (stop) = 69 + 17c

## GF(929) Reed-Solomon

PDF417 uses Reed-Solomon over GF(929) — the prime field of integers modulo 929.
Unlike QR Code's GF(256), no primitive polynomial is needed: every non-zero
element has a multiplicative inverse by Fermat's little theorem.

- Prime: 929
- Generator: α = 3 (primitive root: 3^928 ≡ 1 mod 929)
- Convention: b = 3 (roots α^3 through α^{k+2} for k ECC codewords)
- ECC count: 2^(eccLevel+1)

## Dependencies

- `com.codingadventures:barcode-2d` — `ModuleGrid`, `ModuleShape`
- `com.codingadventures:paint-instructions` — (transitive, for `barcode-2d`)

## Building

```bash
gradle test
```

Or via the monorepo build tool:

```bash
./build-tool --diff-base origin/main
```

## Version

0.1.0
