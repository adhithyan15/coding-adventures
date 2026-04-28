# pdf417 (Java)

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

## What is PDF417?

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes its geometry: each codeword has exactly
**4** bars and **4** spaces, and every codeword occupies exactly **17** modules
of horizontal space. "417" = 4 elements × 17 modules.

Unlike true matrix codes (QR, Data Matrix, Aztec), PDF417 is a **stacked
linear** barcode — it is essentially many rows of 1D-like encoding stacked
vertically. A single horizontal scanner sweep can decode any row independently.

## Where it is used

| Application | Detail |
|---|---|
| AAMVA | North American driver's licences and government IDs |
| IATA BCBP | Airline boarding passes (the barcode on your phone) |
| USPS | Domestic shipping labels |
| US immigration | Form I-94, customs declarations |
| Healthcare | Patient wristbands, medication labels |

## Encoding pipeline (v0.1.0)

```
raw bytes
  → byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
  → length descriptor   (codeword 0 = total codewords in symbol)
  → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
  → dimension selection (auto: roughly square symbol)
  → padding             (codeword 900 fills unused slots)
  → row indicators      (LRI + RRI per row, encode R/C/ECC level)
  → cluster table lookup (codeword → 17-module bar/space pattern)
  → start/stop patterns (fixed per row)
  → ModuleGrid          (abstract boolean grid)
```

## Usage

```java
import com.codingadventures.pdf417.PDF417;
import com.codingadventures.barcode2d.ModuleGrid;

// Encode a string with default options:
ModuleGrid grid = PDF417.encode("HELLO WORLD");

// Encode with explicit options:
PDF417.PDF417Options opts = new PDF417.PDF417Options();
opts.eccLevel  = 4;   // ECC level 0–8 (default: auto)
opts.columns   = 5;   // data columns 1–30 (default: auto)
opts.rowHeight = 4;   // module-rows per logical row (default: 3)
ModuleGrid grid = PDF417.encode("HELLO WORLD".getBytes(), opts);

// ModuleGrid dimensions:
//   grid.rows == rows × rowHeight  (total pixel rows)
//   grid.cols == 69 + 17 × columns (total pixel columns)
```

## Key concepts

### GF(929) — a prime Galois field

PDF417 uses Reed-Solomon error correction over **GF(929)**, not GF(256) like
QR Code. Since 929 is prime, GF(929) is simply the integers modulo 929 — no
polynomial extension field needed. Arithmetic is modular integer arithmetic.

### Three cluster tables

Every row uses one of three "cluster" tables (0, 1, 2) based on `row % 3`.
The cluster tables map codeword values 0–928 to unique 17-module bar/space
patterns. A scanner can detect which cluster it is reading — if the cluster
does not match the row indicators, the row is flagged as potentially damaged.

### Row indicators (LRI/RRI)

Each row carries a Left Row Indicator and a Right Row Indicator. These encode
the total number of rows (R), data columns (C), and ECC level (L) distributed
across three consecutive rows. A scanner that reads any three consecutive rows
can recover R, C, and L.

## Error correction levels

| Level | ECC codewords | Corrects |
|-------|---------------|----------|
| 0 | 2 | 1 error |
| 2 | 8 | 3 errors (default minimum) |
| 4 | 32 | 15 errors (driver's licence grade) |
| 8 | 512 | 255 errors (maximum) |

## Package structure

```
pdf417/
  src/main/java/com/codingadventures/pdf417/
    PDF417.java         — main encoder + GF(929) + RS ECC
    ClusterTables.java  — 3 × 929 packed bar/space patterns (11 KB)
  src/test/java/com/codingadventures/pdf417/
    PDF417Test.java     — 53 unit + integration tests
  build.gradle.kts      — Gradle 8 build script
  settings.gradle.kts   — composite build configuration
  BUILD                 — mono-repo build script
  CHANGELOG.md
  README.md
```

## Dependencies

- `barcode-2d` — provides `ModuleGrid` and `ModuleShape`
- `paint-instructions` — provides `PaintScene` (transitive, compile-time only)
- JUnit 5 (test only)

## Building and testing

```bash
mise exec -- gradle --no-daemon --no-build-cache test
```

## v0.1.0 scope

This release implements **byte compaction only**. All inputs are treated as
raw bytes (codeword 924 latch). Text and numeric compaction (higher density for
pure ASCII / pure digit inputs) are planned for v0.2.0.
