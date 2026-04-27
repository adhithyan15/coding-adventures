# data-matrix (Go)

A complete **Data Matrix ECC200 encoder** conforming to ISO/IEC 16022:2006.

## What is Data Matrix?

Data Matrix is a two-dimensional matrix barcode invented in 1989 and standardised
as ISO/IEC 16022:2006. The ECC200 variant uses Reed-Solomon over GF(256) and is the
dominant form worldwide.

**Where it is used:**

- **PCB traceability** — every board carries a Data Matrix for automated assembly tracking
- **Pharmaceuticals** — US FDA DSCSA mandates Data Matrix on unit-dose packaging
- **Aerospace parts marking** — dot-peened or laser-etched marks that survive decades
- **Medical devices** — GS1 DataMatrix on surgical instruments
- **USPS** — registered mail and customs forms

## How it differs from QR Code

| Feature | QR Code | Data Matrix |
|---------|---------|-------------|
| GF(256) polynomial | 0x11D | **0x12D** |
| RS root convention | b=0 (α⁰…αⁿ⁻¹) | **b=1 (α¹…αⁿ)** |
| Finder pattern | Three 7×7 squares | **L-shaped border** |
| Data placement | Two-column zigzag | **Diagonal "Utah"** |
| Masking | Yes (8 masks) | **None** |
| Symbol sizes | 40 versions | **36 sizes** |

## Encoding pipeline

```
input string
  → ASCII encoding      (chars+1; digit pairs packed)
  → symbol selection    (smallest symbol that fits)
  → pad to capacity     (scrambled pad codewords)
  → RS blocks + ECC     (GF(256)/0x12D, b=1 convention)
  → interleave blocks   (data + ECC round-robin)
  → grid init           (L-finder + timing + alignment borders)
  → Utah placement      (diagonal, no masking)
  → ModuleGrid          (true = dark, false = light)
```

## Usage

```go
import (
    datamatrix "github.com/adhithyan15/coding-adventures/code/packages/go/data-matrix"
    barcode2d  "github.com/adhithyan15/coding-adventures/code/packages/go/barcode-2d"
)

// Encode to a ModuleGrid.
grid, err := datamatrix.EncodeString("Hello World", datamatrix.Options{})
if err != nil {
    log.Fatal(err)
}
fmt.Printf("Symbol: %dx%d\n", grid.Rows, grid.Cols) // 16×16

// Encode to a pixel-resolved PaintScene.
scene, err := datamatrix.EncodeToScene(
    []byte("Hello World"),
    datamatrix.Options{},
    barcode2d.DefaultBarcode2DLayoutConfig,
)
```

### Options

```go
type Options struct {
    // Shape: SymbolShapeSquare (default), SymbolShapeRectangular, SymbolShapeAny
    Shape SymbolShape
}
```

| Shape | Description |
|-------|-------------|
| `SymbolShapeSquare` | Select from 24 square sizes (10×10 … 144×144) |
| `SymbolShapeRectangular` | Select from 6 rectangular sizes (8×18 … 16×48) |
| `SymbolShapeAny` | Try both, pick smallest |

## Symbol sizes

### Square (24 primary sizes)

| Symbol | Data CW | ECC CW | Max chars |
|--------|---------|--------|-----------|
| 10×10  | 3       | 5      | 1         |
| 12×12  | 5       | 7      | 3         |
| 14×14  | 8       | 10     | 6         |
| 16×16  | 12      | 12     | 10        |
| 18×18  | 18      | 14     | 16        |
| 20×20  | 22      | 18     | 20        |
| 22×22  | 30      | 20     | 28        |
| 24×24  | 36      | 24     | 34        |
| 26×26  | 44      | 28     | 42        |
| 32×32  | 62      | 36     | 60        |
| ...    | ...     | ...    | ...       |
| 144×144 | 1558   | 620    | 1556      |

### Rectangular (6 sizes)

| Symbol | Data CW | ECC CW |
|--------|---------|--------|
| 8×18   | 5       | 7      |
| 8×32   | 10      | 11     |
| 12×26  | 16      | 14     |
| 12×36  | 22      | 18     |
| 16×36  | 32      | 24     |
| 16×48  | 49      | 28     |

## Dependencies

| Package | Role |
|---------|------|
| `barcode-2d` | `ModuleGrid` type + `Layout()` for pixel rendering |
| `gf256` | Transitive via barcode-2d |
| `paint-instructions` | `PaintScene` type (output of `EncodeToScene`) |

Note: this package implements its own GF(256)/0x12D field tables and RS encoder
internally (different polynomial from the `gf256` and `reed-solomon` packages
which use 0x11D). The MA02 `reed-solomon` package uses b=1 convention and 0x12D
so it could be used directly; this package inlines the RS encoder for simplicity
and to avoid the dependency.

## Technical notes

### GF(256)/0x12D

Data Matrix uses primitive polynomial 0x12D = x⁸+x⁵+x⁴+x²+x+1. This is
**different from QR Code's 0x11D**. Tables are pre-built at package init time.

### Pad scrambling

Unused codeword slots are filled with scrambled values to prevent degenerate
placement patterns:

```
scrambled_pad(k) = 129 + (149 × k mod 253) + 1
if > 254: subtract 254
```

where k is the 1-indexed position of the pad byte in the full codeword stream.

### Utah placement

The diagonal placement algorithm starts at (row=4, col=0) and zigzags
upward-right then downward-left. Four special "corner patterns" handle
boundary wrap conditions. No masking is applied after placement.

## Testing

```bash
go test ./... -v -cover
```

Coverage: 95.4%

## Version

0.1.0 — ASCII encoding, all 36 symbol sizes, full Utah placement, b=1 RS ECC.

Future (v0.2.0): C40, Text, X12, EDIFACT, Base256 encoding modes.
