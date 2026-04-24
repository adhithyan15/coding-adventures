# @coding-adventures/data-matrix

Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.

Produces valid, scannable Data Matrix ECC200 symbols from any input string.
Outputs a `ModuleGrid` (abstract boolean grid) that can be rendered to SVG,
canvas, or any other backend via the `barcode-2d` paint pipeline.

## What is Data Matrix?

Data Matrix was invented in 1989 and standardised as ISO/IEC 16022:2006.
The ECC200 variant uses Reed-Solomon error correction over GF(256). It appears
on printed circuit boards (traceability), pharmaceutical unit-dose packaging
(US FDA DSCSA mandate), aerospace parts (etched on metal), and medical devices.

Key differences from QR Code:

- **No masking** — the diagonal Utah placement distributes bits naturally.
- **L-shaped finder + clock border** — a solid-dark L on two sides plus
  alternating clock on the other two, instead of three finder squares.
- **Utah diagonal placement** — codewords placed in a diagonal zigzag pattern
  (looks like the US state of Utah from above).
- **GF(256)/0x12D** — uses a different primitive polynomial than QR Code (0x11D).

## Usage

```typescript
import { encode, renderSvg, encodeAndLayout } from "@coding-adventures/data-matrix";

// Encode to a boolean module grid (true = dark module)
const grid = encode("Hello World");
console.log(grid.rows, grid.cols); // 16 16

// Render directly to SVG
const svg = renderSvg("Hello World");
// svg is a complete <svg>...</svg> document

// Encode + layout for custom rendering
const scene = encodeAndLayout("Hello World", {}, {
  moduleSizePx: 4,
  quietZoneModules: 2,
});
```

## API

### `encode(input, options?): ModuleGrid`

Encode a string or `Uint8Array` to a Data Matrix ECC200 module grid.

Selects the smallest symbol that fits the input. Throws `InputTooLongError`
if the input exceeds the 144×144 maximum (1558 data codewords).

**Options:**
- `shape?: "square" | "rectangular" | "any"` — symbol shape preference (default: `"square"`)
- `mode?: "ascii"` — encoding mode (default: `"ascii"`)

### `encodeAndLayout(input, options?, config?): PaintScene`

Encode and convert to a `PaintScene` for rendering. Quiet zone defaults to
1 module (narrower than QR's 4 because the L-finder is self-delimiting).

### `renderSvg(input, options?, config?): string`

Encode and render to a complete SVG string. Do not inject via `innerHTML`;
use `DOMParser` instead.

### `explain(input, options?): AnnotatedModuleGrid`

Encode with per-module role annotations (for interactive visualizers).

## Encoding

v0.1.0 implements ASCII mode:

- Two consecutive ASCII digits → one codeword (130 + value) — saves 50% space
- Single ASCII character → one codeword (ASCII + 1)
- Extended ASCII (128–255) → UPPER_SHIFT + shifted value

## Symbol sizes

Supports all 24 standard square sizes (10×10 through 144×144) and all 6
rectangular sizes (8×18 through 16×48). The smallest symbol that fits the
encoded data is selected automatically.

## Dependencies

- `@coding-adventures/barcode-2d` — `ModuleGrid` type and `layout()` function
- `@coding-adventures/paint-vm-svg` — SVG rendering backend

Note: GF(256)/0x12D arithmetic is implemented inline (the shared `gf256`
package uses 0x11D which is not compatible with Data Matrix).

## Coverage

Tests cover: GF arithmetic, ASCII encoding, pad codewords, RS encoding,
Utah placement, symbol border structure, multi-region symbols, integration
pipeline, and the cross-language test corpus.

Coverage: 93%+ statements, 95%+ branches, 96%+ functions.
