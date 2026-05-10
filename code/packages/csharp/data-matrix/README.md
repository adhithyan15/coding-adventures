# CodingAdventures.DataMatrix.CSharp

Native C# Data Matrix ECC 200 encoder for .NET. Encodes UTF-8 strings or raw
byte payloads into a `ModuleGrid` from `CodingAdventures.Barcode2D`.

```csharp
using CodingAdventures.DataMatrix;

// Square symbol (default)
var grid = DataMatrix.Encode("HELLO WORLD");

// Rectangular symbol
var rect = DataMatrix.Encode("HI", new DataMatrixOptions(DataMatrixSymbolShape.Rectangular));

// Binary payload
var bin = DataMatrix.EncodeBytes(new byte[] { 0x01, 0x02, 0xFF });
```

The encoder implements the complete ISO/IEC 16022:2006 ECC 200 pipeline:

- **ASCII encoding** — digit-pair compression (two decimal digits per codeword),
  extended ASCII via UPPER_SHIFT, and scrambled pad codewords.
- **GF(256)/0x12D Reed-Solomon** — self-contained GF arithmetic using Data
  Matrix's primitive polynomial `0x12D` (distinct from QR Code's `0x11D`).
  Generator polynomial roots α¹…αⁿ (b=1 convention).
- **Block interleaving** — data and ECC codewords round-robin interleaved across
  blocks for burst-error resilience.
- **All 30 symbol sizes** — 24 square symbols (10×10 through 144×144) and
  6 rectangular symbols (8×18 through 16×48).
- **Utah diagonal placement** — the signature Data Matrix algorithm that
  distributes codeword bits diagonally. No masking step required.
- **Four corner patterns** — handle data placement at the symbol boundary
  for symbols whose row/column count is not a multiple of the region size.
- **Multi-region alignment borders** — L-shaped internal dividers for large
  symbols (32×32 and above) that split data into 2×2 or 4×4 region grids.
- **Logical-to-physical coordinate mapping** — correct per-region coordinate
  translation for multi-region symbols.

## Symbol sizes

| Size | Data CW | ECC CW | Regions |
|------|---------|--------|---------|
| 10×10 | 3 | 5 | 1×1 |
| 12×12 | 5 | 7 | 1×1 |
| 14×14 | 8 | 10 | 1×1 |
| 16×16 | 12 | 12 | 1×1 |
| 18×18 | 18 | 14 | 1×1 |
| 20×20 | 22 | 18 | 1×1 |
| 22×22 | 30 | 20 | 1×1 |
| 24×24 | 36 | 24 | 1×1 |
| 26×26 | 44 | 28 | 1×1 |
| … | … | … | … |
| 144×144 | 1558 | 620 | 4×4 |

Rectangular symbols: 8×18 through 16×48 (6 sizes).

## Stack position

This package sits at the same layer as `CodingAdventures.AztecCode.CSharp` —
both encode 2D barcodes into the abstract `ModuleGrid` grid type from
`CodingAdventures.Barcode2D`. Downstream renderers (SVG, PNG, etc.) consume
`ModuleGrid` without knowing which symbology produced it.
