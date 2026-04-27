# @coding-adventures/micro-qr

Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.

Micro QR Code is the compact variant of QR Code, designed for applications
where even the smallest standard QR Code (21×21 at version 1) is too large.
Common use cases include surface-mount component labels, circuit board
markings, and miniature industrial tags.

## What is Micro QR?

Regular QR Code uses three identical 7×7 finder patterns at three corners
so a scanner can identify orientation from any angle. Micro QR uses only
**one** finder pattern, in the top-left corner. Because there is only one,
orientation is always unambiguous — the data area is always to the bottom-right
of the single finder. This saves enormous space, making symbols 50–75% smaller
than equivalent QR Codes.

## Symbol Sizes

| Symbol | Size | Max numeric | Max alphanumeric | Max bytes |
|--------|------|-------------|------------------|-----------|
| M1     | 11×11 | 5           | —               | —         |
| M2     | 13×13 | 10          | 6               | 4         |
| M3     | 15×15 | 23          | 14              | 9         |
| M4     | 17×17 | 35          | 21              | 15        |

## Installation

```bash
npm install @coding-adventures/micro-qr
```

## Usage

```typescript
import { encode, mqrLayout, encodeAndLayout } from "@coding-adventures/micro-qr";

// Encode to a ModuleGrid (abstract boolean grid)
const grid = encode("HELLO");
// grid.rows === 13, grid.cols === 13 (M2 symbol)

// Encode with explicit version and ECC level
const m4Grid = encode("https://example.com", { version: "M4", ecc: "L" });

// Convert to a PaintScene for rendering
const scene = mqrLayout(grid);

// Encode + layout in one step
const scene2 = encodeAndLayout("HELLO", { version: "M4", ecc: "M" });
```

## API

### `encode(input, options?)`

Encodes a string to a Micro QR Code `ModuleGrid`.

- `options.version?: "M1" | "M2" | "M3" | "M4"` — force a specific symbol size.
  If omitted, the smallest symbol that fits the input is chosen.
- `options.ecc?: "DETECTION" | "L" | "M" | "Q"` — error correction level.
  If omitted, the encoder tries L, M, Q in order.

Returns a `ModuleGrid` with `moduleShape: "square"`.

### `mqrLayout(grid, config?)`

Converts a `ModuleGrid` to a `PaintScene` using `barcode-2d`'s `layout()`.
Defaults to `quietZoneModules: 2` (the Micro QR minimum, half of regular QR's 4).

### `encodeAndLayout(input, options?, config?)`

Convenience: `encode` + `mqrLayout` in one call.

### `explain(input, options?)`

Returns an `AnnotatedModuleGrid` for visualizer use (v0.1.0: annotations are null).

## Error Handling

```typescript
import {
  MicroQRError,
  InputTooLongError,
  UnsupportedModeError,
  ECCNotAvailableError,
} from "@coding-adventures/micro-qr";

try {
  encode("a".repeat(20)); // exceeds M3-L byte capacity (9)
} catch (e) {
  if (e instanceof InputTooLongError) {
    // Input too long for any Micro QR symbol at any ECC level
  }
}
```

## Key Differences from Regular QR Code

| Feature | Regular QR | Micro QR |
|---------|-----------|----------|
| Finder patterns | 3 | 1 |
| Timing strips | Row 6, col 6 | Row 0, col 0 |
| Quiet zone | 4 modules | 2 modules |
| ECC levels | L, M, Q, H | DETECTION, L, M, Q |
| Mask patterns | 8 | 4 |
| Format info copies | 2 | 1 |
| Format XOR mask | 0x5412 | 0x4445 |
| Max capacity | 7089 numeric | 35 numeric |

## Dependencies

- `@coding-adventures/barcode-2d` — `ModuleGrid` type and `layout()` function
- `@coding-adventures/gf256` — GF(256) multiplication for the Reed-Solomon encoder
