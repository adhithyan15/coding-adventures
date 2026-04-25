# @coding-adventures/pdf417

PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The name encodes its geometry: each codeword contains
exactly 4 bars and 4 spaces (8 elements) and occupies exactly 17 modules of
horizontal space.

## Where PDF417 is used

| Application | Detail |
|---|---|
| AAMVA | North American driver's licences and government IDs |
| IATA BCBP | Airline boarding passes |
| USPS | Domestic shipping labels |
| US immigration | Form I-94, customs declarations |
| Healthcare | Patient wristbands, medication labels |

## How it fits in the stack

```text
raw bytes
  → @coding-adventures/pdf417 (this package)
  → ModuleGrid    — abstract boolean grid  (@coding-adventures/barcode-2d)
  → layout()      — pixel coordinates      (@coding-adventures/barcode-2d)
  → PaintScene    — render instructions    (@coding-adventures/paint-instructions)
  → paint backend — SVG, Metal, Canvas, terminal, …
```

## Usage

```typescript
import { encode, encodeAndLayout, type PDF417Options } from "@coding-adventures/pdf417";

// Encode with automatic ECC level and dimensions.
const grid = encode(new TextEncoder().encode("Hello, PDF417!"));
console.log(`${grid.rows} × ${grid.cols} modules`);

// Encode with explicit options.
const grid2 = encode(new TextEncoder().encode("Hello, PDF417!"), {
  eccLevel:  4,   // higher redundancy
  columns:   6,   // 6 data columns
  rowHeight: 4,   // 4 module-rows per logical row
});

// Produce a PaintScene for rendering.
const scene = encodeAndLayout(new TextEncoder().encode("Hello, PDF417!"));
console.log(`${scene.width} × ${scene.height} px`);
```

## Encoding pipeline

1. **Byte compaction** — codeword 924 latch. 6 bytes → 5 base-900 codewords;
   remaining bytes are direct (1 codeword each).
2. **Length descriptor** — first codeword = total codewords in the symbol.
3. **Reed-Solomon ECC** — GF(929) with b=3 convention, roots α³…α^{k+2}.
   Auto-level selection: level 2 for ≤40 data codewords, up to level 6.
4. **Dimension selection** — auto: `c = ⌈√(total/3)⌉`, `r = ⌈total/c⌉`,
   clamped to 3–90 rows × 1–30 columns.
5. **Padding** — codeword 900 fills any unused slots.
6. **Row indicators** — LRI and RRI per row encode row count, column count,
   and ECC level across a repeating 3-cluster cycle.
7. **Cluster table lookup** — each codeword maps to a 17-module bar/space
   pattern that differs by cluster (row mod 3), enabling single-row
   identification.
8. **Start/stop patterns** — `11111111010101000` (17 modules) and
   `111111101000101001` (18 modules) bracket every row.
9. **ModuleGrid** — boolean 2D grid output (`true` = dark module).

## v0.1.0 scope

This release implements **byte compaction only**. Text and numeric compaction
are planned for v0.2.0.

## GF(929) is implemented inline

Unlike QR Code (which uses `@coding-adventures/gf256`), PDF417 operates over
GF(929) — a different prime field. The arithmetic is implemented inline in this
package (~20 lines, log/antilog tables built at module load time) rather than
depending on a separate package that doesn't yet exist on main.

## Dependencies

- [`@coding-adventures/barcode-2d`](../barcode-2d) — `ModuleGrid` type,
  `layout()` function, and re-exported `PaintScene` type.

## Testing

```sh
npx vitest run --coverage
```

61 tests covering GF arithmetic, byte compaction, row indicators,
start/stop patterns, module dimensions, ECC bounds, integration tests,
error handling, determinism, and cross-row structural correctness.
Coverage: 97.65% statements, 100% functions.
