# data-matrix (Java)

A Java implementation of the **Data Matrix ECC200 encoder** — ISO/IEC 16022:2006 compliant.

Data Matrix is a two-dimensional matrix barcode used wherever small, high-density,
damage-tolerant marks are needed on physical objects: PCB traceability, pharmaceutical
packaging, aerospace parts marking, and medical device identification.

## Features

- Full Data Matrix ECC200 encoding pipeline (ISO/IEC 16022:2006)
- ASCII encoding with digit-pair optimization (two consecutive digits → one codeword)
- Reed-Solomon error correction over GF(256)/0x12D using the b=1 convention
- All 24 square symbol sizes (10×10 through 144×144)
- All 6 rectangular symbol sizes (8×18 through 16×48)
- Automatic symbol size selection (smallest fitting symbol)
- Multi-region symbol support with alignment borders (32×32 through 144×144)
- No masking — the Utah diagonal placement distributes bits without needing it
- Literate, well-commented code that teaches the algorithm step by step

## Usage

```java
import com.codingadventures.datamatrix.DataMatrix;
import com.codingadventures.barcode2d.ModuleGrid;

// Encode a string — automatically selects the smallest fitting symbol
ModuleGrid grid = DataMatrix.encode("Hello World", null);
assert grid.rows == 16;  // 11 codewords → 16×16 symbol
assert grid.cols == 16;

// Encode a single character — 10×10 is the smallest Data Matrix symbol
ModuleGrid small = DataMatrix.encode("A", null);
assert small.rows == 10;

// Encode digits — digit pairs are packed efficiently
ModuleGrid digits = DataMatrix.encode("1234", null);
assert digits.rows == 10;  // "12" + "34" = 2 codewords, fits in 10×10

// Use rectangular symbol shapes
DataMatrix.DataMatrixOptions opts = new DataMatrix.DataMatrixOptions(DataMatrix.SymbolShape.RECTANGULAR);
ModuleGrid rect = DataMatrix.encode("A", opts);
assert rect.rows == 8;   // 8×18 rectangular
assert rect.cols == 18;

// Check modules — true = dark, false = light
boolean module = grid.modules.get(0).get(0);  // top-left = always dark (L-finder)
```

## How Data Matrix ECC200 Works

### Symbol structure

Every Data Matrix symbol has an L-shaped "finder" and "timing" border:

```
D D D D D D D D D D   ← timing row (alternating dark/light, top)
D . . . . . . . . D   ← right column timing
D . data modules . L
D . (placed by    . D
D .  Utah algo)   . L
D . . . . . . . . D
D D D D D D D D D D   ← L-finder bottom row (all dark)
^
L-finder left col (all dark)
```

- **Left column** and **bottom row**: all dark — this is the "L-finder" that orients the scanner
- **Top row** and **right column**: alternating dark/light — the timing clock for module pitch

### Encoding pipeline

```
input string
  → ASCII encode   (chars+1; digit pairs "12" → 130+12=142; saves ~50% for digit strings)
  → symbol select  (smallest symbol whose dataCW capacity ≥ codeword count)
  → pad to capacity (first pad = 129; subsequent scrambled to avoid degenerate patterns)
  → RS ECC         (GF(256)/0x12D, b=1 convention, LFSR block encoder)
  → interleave     (data round-robin then ECC round-robin across blocks)
  → init grid      (L-finder + timing border + optional alignment borders for large symbols)
  → Utah placement (diagonal codeword placement — NO masking!)
  → ModuleGrid     (boolean grid: true=dark, false=light)
```

### Key differences from QR Code

| Feature | QR Code | Data Matrix |
|---------|---------|-------------|
| Field polynomial | 0x11D | 0x12D |
| RS convention | b=0 (roots α^0..α^{n-1}) | b=1 (roots α^1..α^n) |
| Finder pattern | Three 7×7 squares | L-shaped border (left+bottom) |
| Data placement | Two-column zigzag | Utah diagonal |
| Masking | 8 patterns evaluated | None — not needed |

### The Utah placement algorithm

Named after the US state of Utah because the 8-module shape used to place each
codeword resembles its outline. The algorithm scans the logical data matrix in a
diagonal zigzag, placing codeword bits at fixed offsets:

```
   col: c-2  c-1   c
row-2:  .   [1]  [2]
row-1: [3]  [4]  [5]
row  : [6]  [7]  [8]
```

Four special corner patterns handle edge cases at the boundary.

## Package matrix

| Language | Package |
|----------|---------|
| TypeScript | `code/packages/typescript/data-matrix/` |
| Java | `code/packages/java/data-matrix/` ← this package |
| Kotlin | `code/packages/kotlin/data-matrix/` |
| Go | `code/packages/go/data-matrix/` |
| Python | `code/packages/python/data-matrix/` |

## Dependencies

- `barcode-2d` — provides `ModuleGrid` and `ModuleShape`
- `gf256` — GF(256) field arithmetic (transitively through barcode-2d)

## Building

```bash
mise exec -- gradle test
```

Requires Java 21 and Gradle (via mise).

## Test coverage

61 unit and integration tests covering:
- GF(256)/0x12D field arithmetic (exp/log tables, multiplication, commutativity, associativity)
- ASCII encoding (single chars, digit pairs, extended ASCII, odd-length strings)
- Pad codeword generation (first-pad literal 129, subsequent scrambled)
- Reed-Solomon ECC encoding (LFSR block encoder, generator polynomial degrees)
- Symbol selection (square, rectangular, various sizes, error cases)
- Border patterns (L-finder, timing clock, corner invariants)
- Alignment borders for multi-region symbols (32×32)
- Full pipeline integration with cross-language corpus inputs
- Determinism (encoding same input twice produces identical output)
- Error handling (input too long, null options)

## Specification

See `code/specs/data-matrix.md` for the complete ISO/IEC 16022:2006 specification
and algorithm description.
