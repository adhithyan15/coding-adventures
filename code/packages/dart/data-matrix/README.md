# coding_adventures_data_matrix

Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.

Encodes strings into Data Matrix symbols (10×10 to 144×144 squares plus six
rectangular sizes) using ASCII mode with digit-pair compression, GF(256)/0x12D
Reed-Solomon error correction (b=1 convention), L-shaped finder + timing-clock
border, and the diagonal "Utah" placement algorithm. No masking step. Produces a
`ModuleGrid` consumable by the `coding_adventures_barcode_2d` package for
pixel-level rendering.

---

## Where Data Matrix is used

- **PCBs** — every modern circuit board carries an etched Data Matrix for
  traceability through automated assembly lines.
- **Pharmaceuticals** — US FDA DSCSA mandates Data Matrix on unit-dose packages.
- **Aerospace parts** — dot-peened marks survive decades of heat and abrasion that
  would destroy ink-printed labels.
- **Medical devices** — GS1 DataMatrix on surgical instruments and implants.
- **USPS registered mail and customs forms**.

---

## Key differences from QR Code

| Property        | QR Code              | Data Matrix ECC200      |
|-----------------|----------------------|-------------------------|
| GF(256) poly    | 0x11D                | **0x12D**               |
| RS root start   | b = 0 (α⁰ …)        | **b = 1 (α¹ …)**        |
| Finder          | three corner squares | **L-shape** (left + bottom) |
| Data placement  | column zigzag        | **"Utah" diagonal**     |
| Masking         | 8 patterns, scored   | **NONE**                |
| Sizes           | 40 versions          | **24 square + 6 rect**  |

---

## Quick start

```dart
import 'package:coding_adventures_data_matrix/data_matrix.dart';

// Auto-select smallest square symbol for "A" → 10×10
final grid = encode('A');
print(grid.rows);  // 10
print(grid.cols);  // 10

// Force a specific size
final big = encode('Hello', options: DataMatrixOptions(size: 18));
print(big.rows);   // 18

// Try any shape (square or rectangular), pick smallest by area
final any = encode('HELLO', options: DataMatrixOptions(shape: SymbolShape.any));

// Encode + render to PaintScene in one call
final scene = encodeAndLayout('Hello, World!');
```

---

## Public API

### Constants

| Name              | Value   | Description                                         |
|-------------------|---------|-----------------------------------------------------|
| `dataMatrixVersion` | `'0.1.0'` | Package version                                |
| `gf256Prime`      | `0x12D` | GF(256) primitive polynomial for Data Matrix ECC200 |
| `minSize`         | `10`    | Smallest supported square symbol dimension          |
| `maxSize`         | `144`   | Largest supported square symbol dimension           |

### Types

| Type               | Description                                              |
|--------------------|----------------------------------------------------------|
| `SymbolShape`      | `square` / `rectangle` / `any` shape preference enum    |
| `DataMatrixOptions`| Options for `encode()` (`size`, `shape`)                 |
| `DataMatrixError`  | Base class for all encoder errors                        |
| `InputTooLongError`| Input exceeds the largest symbol's capacity              |
| `InvalidSizeError` | Caller-provided `size` is not a valid DM dimension       |
| `ModuleGrid`       | Re-exported from `coding_adventures_barcode_2d`          |

### Functions

| Function           | Description                                              |
|--------------------|----------------------------------------------------------|
| `encode()`         | Encode string → `ModuleGrid` (auto-selects symbol size)  |
| `layoutGrid()`     | `ModuleGrid` → `PaintScene` with DM defaults             |
| `encodeAndLayout()`| Encode + layout in one step                              |
| `gridToString()`   | `ModuleGrid` → debug string of '0'/'1' lines             |

---

## Encoding pipeline

```
input string
  → ASCII encoding    (char+1; digit pairs → 130+pair)
  → symbol selection  (smallest symbol whose data_cw ≥ codeword count)
  → pad to capacity   (EOM=129, then scrambled pads per §5.2.3)
  → RS blocks + ECC   (GF(256)/0x12D, b=1 convention)
  → interleave blocks (data round-robin then ECC round-robin)
  → grid init         (L-finder + timing border + alignment borders)
  → Utah placement    (diagonal codeword placement — no masking)
  → ModuleGrid
```

### ASCII encoding rules

| Input          | Codewords    | Rule                       |
|----------------|--------------|----------------------------|
| `"A"` (65)     | `[66]`       | ASCII value + 1            |
| `" "` (32)     | `[33]`       | ASCII value + 1            |
| `"12"`         | `[142]`      | 130 + 12 (digit pair)      |
| `"1234"`       | `[142, 174]` | two digit pairs             |
| Extended ASCII | 2 codewords  | UPPER_SHIFT (235) + offset |

### Symbol sizes — squares (ISO/IEC 16022:2006 Table 7)

| Size      | Data CW | ECC CW | Blocks |
|-----------|---------|--------|--------|
| 10 × 10   | 3       | 5      | 1      |
| 12 × 12   | 5       | 7      | 1      |
| 14 × 14   | 8       | 10     | 1      |
| 16 × 16   | 12      | 12     | 1      |
| 18 × 18   | 18      | 14     | 1      |
| …         | …       | …      | …      |
| 144 × 144 | 1558    | 620    | 10     |

---

## Dependencies

- `coding_adventures_barcode_2d` (path: `../barcode-2d`) — provides `ModuleGrid`,
  `makeModuleGrid`, `setModule`, `layout()`, and `PaintScene`.

---

## Building and testing

```bash
# From this package directory:
dart pub get
dart test -r expanded
```

Or via the monorepo build tool (which handles dependency order automatically):

```bash
cd code/programs/go/build-tool && go build -o build-tool . && ./build-tool
```

---

## How it fits in the stack

```
coding_adventures_data_matrix
       │
       └── coding_adventures_barcode_2d
                │
                └── coding_adventures_paint_instructions
```

The encoder produces a `ModuleGrid` (abstract boolean grid). The barcode-2d
library converts it to a `PaintScene` (pixel-level paint instructions). A paint
backend (SVG, Canvas, Metal) then renders the scene.
