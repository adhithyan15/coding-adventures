# DataMatrix — Swift

ISO/IEC 16022:2006 ECC200 Data Matrix encoder for Swift.

## What it does

Encodes any string into a Data Matrix ECC200 barcode as a `ModuleGrid` — a
two-dimensional boolean grid where `true` = dark module and `false` = light
module. The grid is ready to render with the `Barcode2D` layout pipeline
(→ `PaintScene` → SVG / Metal / Canvas / terminal backend).

## Where Data Matrix is used

| Industry       | Application                                             |
|----------------|---------------------------------------------------------|
| Electronics    | PCB traceability — every board has a tiny dot-peened mark |
| Pharmaceuticals | FDA DSCSA mandates Data Matrix on unit-dose packages   |
| Aerospace      | Etched marks survive decades of heat and abrasion       |
| Medical devices | GS1 DataMatrix on surgical instruments and implants    |
| Postage        | USPS registered mail and customs forms                  |

## Key differences from QR Code

| Property       | QR Code            | Data Matrix ECC200    |
|----------------|--------------------|-----------------------|
| GF(256) poly   | 0x11D              | **0x12D**             |
| RS root start  | b = 0 (α⁰..)       | **b = 1 (α¹..)**      |
| Finder         | Three corner squares | **One L-shape**      |
| Placement      | Column zigzag      | **"Utah" diagonal**   |
| Masking        | 8 patterns scored  | **NONE**              |
| Symbol sizes   | 40 versions        | **30 square + 6 rect**|

## Usage

### Auto-select the smallest square symbol

```swift
import DataMatrix

let grid = try encode("Hello, World!")
print("\(grid.rows)×\(grid.cols)")  // e.g. "14×14"
```

### Force a specific symbol size

```swift
// Force 18×18 square
let grid = try encode("Hi", squareSize: 18)
assert(grid.rows == 18 && grid.cols == 18)
```

### Encode into a rectangular symbol

```swift
// Rectangular symbols are useful for narrow label areas
let grid = try encode("Hi", rows: 8, cols: 18)
assert(grid.rows == 8 && grid.cols == 18)
```

### Use DataMatrixOptions for shape control

```swift
var opts = DataMatrixOptions()
opts.shape = .rectangle   // only consider rectangular symbols
let grid = try encode("Hi", options: opts)

// Or allow both square and rectangular, pick smallest:
opts.shape = .any
let grid2 = try encode("Hi", options: opts)
```

### Debug: render as ASCII

```swift
let grid = try encode("A")
print(gridToString(grid))
// 1010101011
// 1010100010
// 1000000011
// ...
```

## Public API

```swift
// Version and field constants
public let dataMatrixVersion: String  // "0.1.0"
public let gf256Prime: Int            // 0x12D = 301

// Options
public struct DataMatrixOptions: Sendable {
    public var size: Int?             // nil = auto-select; else square side length
    public var shape: SymbolShape     // .square (default), .rectangle, .any
    public init()
}

public enum SymbolShape: Sendable { case square, rectangle, any }

// Errors
public enum DataMatrixError: Error {
    case inputTooLong(String)
    case invalidSize(String)
}

// Encoding functions
public func encode(_ data: String, options: DataMatrixOptions = .init()) throws -> ModuleGrid
public func encode(_ data: String, squareSize: Int) throws -> ModuleGrid
public func encode(_ data: String, rows: Int, cols: Int) throws -> ModuleGrid

// Utility
public func gridToString(_ grid: ModuleGrid) -> String
```

## Symbol sizes

### Square symbols (24 sizes)

| Symbol     | Data CW | ECC CW | Blocks |
|------------|---------|--------|--------|
| 10×10      | 3       | 5      | 1      |
| 12×12      | 5       | 7      | 1      |
| 14×14      | 8       | 10     | 1      |
| 16×16      | 12      | 12     | 1      |
| 18×18      | 18      | 14     | 1      |
| 20×20      | 22      | 18     | 1      |
| 22×22      | 30      | 20     | 1      |
| 24×24      | 36      | 24     | 1      |
| 26×26      | 44      | 28     | 1      |
| 32×32      | 62      | 36     | 2      |
| …          | …       | …      | …      |
| 144×144    | 1558    | 620    | 10     |

### Rectangular symbols (6 sizes)

| Symbol  | Data CW | ECC CW |
|---------|---------|--------|
| 8×18    | 5       | 7      |
| 8×32    | 10      | 11     |
| 12×26   | 16      | 14     |
| 12×36   | 22      | 18     |
| 16×36   | 32      | 24     |
| 16×48   | 49      | 28     |

## Encoding pipeline

```
input string (UTF-8)
  → ASCII mode encoding
      - two consecutive digits → one codeword = 130 + (d1×10 + d2)
      - ASCII char (0–127)    → one codeword = value + 1
      - Extended (128–255)   → two codewords: 235 then (value - 127)
  → symbol selection (smallest fitting size)
  → pad to capacity (scrambled EOM sequence)
  → Reed-Solomon ECC (GF(256)/0x12D, b=1 convention, per block)
  → interleave blocks (data round-robin, then ECC round-robin)
  → grid init (L-finder + timing border + alignment borders)
  → Utah diagonal placement (NO masking)
  → ModuleGrid
```

## Dependencies

- `Barcode2D` — provides `ModuleGrid`, `makeModuleGrid`, `setModule`
- `GF256` — Galois field arithmetic (package included for consistency; this
  encoder builds its own GF(256)/0x12D tables since the field polynomial
  differs from QR Code's 0x11D)
- `PaintInstructions` — `PaintScene` type for rendering pipeline

## Tests

72 tests across 16 suites covering:

- Package constants (`dataMatrixVersion`, `gf256Prime`)
- Auto-size selection for all capacity boundaries
- Grid structure validation (dimensions, module shape)
- Determinism (same input → same output, always)
- L-finder verification (bottom row + left column all dark)
- Timing border verification (alternating top + right, with corner rules)
- Empty input handling
- Error cases (too long, invalid size, forced size overflow)
- Explicit square and rectangular size overrides
- Digit-pair compaction
- Multi-region symbols (32×32, 64×64 with multiple data regions)
- Cross-language corpus tests for interoperability

## License

Part of the coding-adventures educational computing stack.
