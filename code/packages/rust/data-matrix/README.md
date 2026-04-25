# data-matrix (Rust)

Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.

Encodes any string or byte slice into a valid, scannable Data Matrix ECC200
symbol. Outputs a `ModuleGrid` (abstract boolean grid) that can be passed to
`barcode-2d`'s `layout()` for pixel-level rendering.

## What is Data Matrix?

Data Matrix was invented in 1989 and standardised as ISO/IEC 16022:2006.
It is widely used where a small, high-density, damage-tolerant mark is needed:

| Application           | Detail                                             |
|-----------------------|----------------------------------------------------|
| PCB traceability      | Every board carries an etched Data Matrix          |
| Pharmaceuticals       | US FDA DSCSA mandates unit-dose Data Matrix marks  |
| Aerospace parts       | Rivets, shims, brackets — laser-etched in metal    |
| Medical devices       | Surgical instruments, GS1 DataMatrix               |

## Pipeline

```text
input bytes
  → ASCII encoding     (chars+1; digit pairs packed into one codeword)
  → symbol selection   (smallest symbol whose capacity ≥ codeword count)
  → pad to capacity    (scrambled-pad codewords fill unused slots)
  → RS blocks + ECC    (GF(256)/0x12D, b=1 convention)
  → interleave blocks  (data round-robin then ECC round-robin)
  → grid init          (L-finder + timing border + alignment borders)
  → Utah placement     (diagonal codeword placement, no masking!)
  → ModuleGrid
```

## Key differences from QR Code

| Property              | QR Code              | Data Matrix                         |
|-----------------------|----------------------|-------------------------------------|
| GF polynomial         | 0x11D                | **0x12D**                           |
| RS root convention    | b=0 (α^0..α^{n-1})  | **b=1 (α^1..α^n)**                  |
| Finder pattern        | Three 7×7 squares    | **L-shaped finder + clock border**  |
| Data placement        | Two-column zigzag    | **Utah diagonal zigzag**            |
| Masking               | 8 patterns evaluated | **No masking**                      |

## Usage

```rust
use data_matrix::{encode, encode_str, encode_and_layout, DataMatrixOptions};

// Encode a string to an abstract module grid:
let grid = encode_str("Hello World", Default::default()).unwrap();
assert_eq!(grid.rows, 16); // "Hello World" → 16×16 symbol
assert_eq!(grid.cols, 16);

// Encode raw bytes:
let grid = encode(b"\x01\x02\x03", Default::default()).unwrap();

// Encode and convert to a pixel-resolved PaintScene:
use barcode_2d::Barcode2DLayoutConfig;
let scene = encode_and_layout(b"Hello World", Default::default(), None).unwrap();
assert!(scene.width > 0.0);
```

## Symbol shapes

```rust
use data_matrix::{DataMatrixOptions, SymbolShape};

// Square only (default — 30 sizes from 10×10 to 144×144):
let opts = DataMatrixOptions { shape: SymbolShape::Square };

// Rectangular only (6 sizes: 8×18, 8×32, 12×26, 12×36, 16×36, 16×48):
let opts = DataMatrixOptions { shape: SymbolShape::Rectangular };

// Pick smallest of all 30 sizes:
let opts = DataMatrixOptions { shape: SymbolShape::Any };
```

## API

### `encode(input: &[u8], options: DataMatrixOptions) -> Result<ModuleGrid, DataMatrixError>`

Encode a byte slice into a Data Matrix ECC200 module grid.

- Selects the smallest symbol whose data capacity fits the ASCII-encoded codeword count.
- Returns `Err(DataMatrixError::InputTooLong)` if the input would require more than 1558 codewords (the 144×144 symbol capacity).

### `encode_str(input: &str, options: DataMatrixOptions) -> Result<ModuleGrid, DataMatrixError>`

Convenience wrapper around `encode()` for UTF-8 strings.

### `encode_and_layout(input: &[u8], options: DataMatrixOptions, config: Option<Barcode2DLayoutConfig>) -> Result<PaintScene, DataMatrixError>`

Encode and convert to a pixel-resolved `PaintScene` via `barcode-2d`'s `layout()`.
The default quiet zone is 1 module (narrower than QR's 4 modules because the
L-finder is inherently self-delimiting).

## Error types

- `DataMatrixError::InputTooLong(String)` — encoded codeword count exceeds maximum symbol capacity (1558 for 144×144).

## Dependencies

- `barcode-2d` — `ModuleGrid` type and `layout()` pixel geometry
- `paint-instructions` — `PaintScene` type (for `encode_and_layout` return type)

## Spec

See `code/specs/data-matrix.md` for the full specification.
