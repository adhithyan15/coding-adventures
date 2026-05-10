# data-matrix (Kotlin)

Data Matrix ECC 200 encoder — ISO/IEC 16022:2006 compliant.

Part of the 2D barcode stack in [coding-adventures](../../../../).

## What is Data Matrix?

Data Matrix is a two-dimensional matrix barcode invented by RVSI Acuity CiMatrix in 1989 and
standardised as ISO/IEC 16022:2006.  The ECC 200 variant uses Reed-Solomon error correction over
GF(256) and is the dominant form worldwide.

Where it is used:

- **PCBs** — every printed-circuit board carries a Data Matrix for traceability through automated assembly lines.
- **Pharmaceuticals** — US FDA DSCSA mandates Data Matrix on unit-dose packages.
- **Aerospace** — etched / dot-peened marks on rivets and brackets survive decades of heat and abrasion.
- **Medical devices** — GS1 DataMatrix on surgical instruments and implants.
- **USPS** registered mail and customs forms.

## Where this package fits

```
paint-metal   paint-vm-svg   paint-vm-canvas
      └───────────┬──────────────┘
                  │
           paint-vm (P2D01)
                  │
       paint-instructions (P2D00)
                  │
              barcode-2d
                  │
            data-matrix   ← this package
                  │
     GF(256)/0x12D + RS (self-contained)
```

## Key differences from QR Code

| Feature              | QR Code               | Data Matrix           |
|----------------------|-----------------------|-----------------------|
| Primitive polynomial | 0x11D                 | **0x12D**             |
| RS generator roots   | α^0 … α^{n-1}        | **α^1 … α^n**        |
| Finder pattern       | 3 corner squares      | **L-bar + clock**     |
| Data placement       | Column zigzag         | **Utah diagonal**     |
| Masking              | 8 mask patterns       | **No masking**        |

## Encoding pipeline

```
input string
  → ASCII encoding      (chars+1; digit pairs packed into one codeword)
  → symbol selection    (smallest symbol whose capacity ≥ codeword count)
  → pad to capacity     (scrambled-pad codewords, ISO §5.2.3)
  → RS blocks + ECC     (GF(256)/0x12D, b=1 convention, LFSR encoder)
  → interleave blocks   (data round-robin then ECC round-robin)
  → grid init           (L-finder + timing border + alignment borders)
  → Utah placement      (diagonal codeword placement, NO masking)
  → Array<BooleanArray> (physical symbol, true = dark module)
```

## Usage

```kotlin
import com.codingadventures.datamatrix.DataMatrix
import com.codingadventures.datamatrix.SymbolShape

// Encode a string — auto-selects the smallest fitting square symbol
val grid: Array<BooleanArray> = DataMatrix.encode("Hello World")
println("${grid.size} × ${grid[0].size}")   // 16 × 16

// grid[r][c] == true  →  dark module
// grid[r][c] == false →  light module

// Prefer rectangular symbols
val rectGrid = DataMatrix.encode("ABCDE", SymbolShape.RECTANGULAR)
println("${rectGrid.size} × ${rectGrid[0].size}")   // 8 × 18

// Try both shapes, pick the smallest
val anyGrid = DataMatrix.encode("1234", SymbolShape.ANY)
```

## Symbol sizes

### Square (default)

| Symbol    | Data codewords | Max ASCII chars |
|-----------|----------------|-----------------|
| 10×10     | 3              | 1               |
| 12×12     | 5              | 3               |
| 14×14     | 8              | 6               |
| 16×16     | 12             | 10              |
| 18×18     | 18             | 16              |
| 20×20     | 22             | 20              |
| 22×22     | 30             | 28              |
| 24×24     | 36             | 34              |
| 26×26     | 44             | 42              |
| 32×32     | 62             | 60              |
| …         | …              | …               |
| 144×144   | 1558           | 1556            |

### Rectangular

| Symbol    | Data codewords |
|-----------|----------------|
| 8×18      | 5              |
| 8×32      | 10             |
| 12×26     | 16             |
| 12×36     | 22             |
| 16×36     | 32             |
| 16×48     | 49             |

## Building and testing

```bash
cd code/packages/kotlin/data-matrix
./gradlew test
```

Requires JDK 21 and Gradle (wrapper included).

## Running via the build tool

```bash
./build-tool   # from repo root — detects changed files and runs affected builds
```

## Error handling

```kotlin
import com.codingadventures.datamatrix.InputTooLongException

try {
    DataMatrix.encode("A".repeat(1559))   // exceeds 144×144 capacity
} catch (e: InputTooLongException) {
    println("Too long: ${e.encodedCW} codewords, max ${e.maxCW}")
}
```

## Cross-language verification

This package is one of 15 language implementations of the same spec.  All
implementations must produce **identical** `Array<BooleanArray>` outputs for
the same input.  Cross-language test vectors (JSON) live in `code/specs/`.

Reference implementations:

- TypeScript: `code/packages/typescript/data-matrix/`
- Go (most literate): `code/packages/go/data-matrix/`

## Spec

`code/specs/data-matrix.md`
